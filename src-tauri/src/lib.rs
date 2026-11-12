//! The desktop shell: a window, and the bridge between it and the engine.
//!
//! Everything that matters lives in `sub20-core`. This file only exposes
//! it to the window as commands and forwards job events back.

mod media;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sub20_core::hardware::Report;
use sub20_core::history::Entry;
use sub20_core::job::{EncodeOptions, JobEvent, JobId, JobSpec, Origin, Trim};
use sub20_core::plan::{self, Plan};
use sub20_core::settings::{OutputMode, Settings};
use sub20_core::watch::{self, WatchStatus, Watcher};
use sub20_core::{thumbs, Engine, MediaInfo};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_opener::OpenerExt;

struct AppState {
    engine: Engine,
    watcher: Arc<Watcher>,
    thumbs_dir: PathBuf,
    /// Videos passed on the command line ("Open with Sub20").
    opened_with: Vec<PathBuf>,
    /// Serves previews over loopback HTTP where the asset protocol cannot
    /// play video (Linux).
    media: Option<media::MediaServer>,
    /// Files this app has written. Only these may be previewed or copied to
    /// the clipboard as results.
    results: Arc<Mutex<HashSet<PathBuf>>>,
}

type CommandResult<T> = Result<T, String>;

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FfmpegStatus {
    found: bool,
    version: Option<String>,
    folder: Option<PathBuf>,
}

async fn ffmpeg_state(engine: &Engine) -> FfmpegStatus {
    match engine.tools() {
        Ok(tools) => FfmpegStatus {
            found: true,
            version: tools.version().await,
            folder: tools.ffmpeg.parent().map(Path::to_path_buf),
        },
        Err(_) => FfmpegStatus { found: false, version: None, folder: None },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    settings: Settings,
    ffmpeg: FfmpegStatus,
    version: &'static str,
    opened_with: Vec<PathBuf>,
}

/// Everything the window needs when it opens.
#[tauri::command]
async fn bootstrap(state: State<'_, AppState>) -> CommandResult<Bootstrap> {
    Ok(Bootstrap {
        settings: state.engine.settings().get(),
        ffmpeg: ffmpeg_state(&state.engine).await,
        version: env!("CARGO_PKG_VERSION"),
        opened_with: state.opened_with.clone(),
    })
}

#[tauri::command]
async fn ffmpeg_status(state: State<'_, AppState>) -> CommandResult<FfmpegStatus> {
    Ok(ffmpeg_state(&state.engine).await)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Added {
    path: PathBuf,
    name: String,
    info: Option<MediaInfo>,
    /// Where the window should load the video from, when not the asset
    /// protocol.
    play_url: Option<String>,
    /// Why the file was not accepted.
    error: Option<String>,
}

/// Reads the facts of each dropped or picked file. Folders are opened one
/// level deep.
#[tauri::command]
async fn add_videos(app: AppHandle, state: State<'_, AppState>, paths: Vec<PathBuf>) -> CommandResult<Vec<Added>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut inside: Vec<PathBuf> = std::fs::read_dir(&path)
                .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| watch::is_video(p)).collect())
                .unwrap_or_default();
            inside.sort();
            files.extend(inside);
        } else {
            files.push(path);
        }
    }

    let mut added = Vec::with_capacity(files.len());
    for path in files {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let outcome = state.engine.probe(&path).await;
        let mut play_url = None;
        let (info, error) = match outcome {
            Ok(info) if !info.has_video => (None, Some("This file has no video track.".to_string())),
            Ok(info) if info.duration <= 0.0 => (None, Some("The length of this video could not be read.".to_string())),
            Ok(info) => {
                // Let the window play this one file.
                let _ = app.asset_protocol_scope().allow_file(&path);
                play_url = state.media.as_ref().map(|server| server.url_for(&path));
                (Some(info), None)
            }
            Err(e) => (None, Some(text(e))),
        };
        added.push(Added { path, name, info, play_url, error });
    }
    Ok(added)
}

/// Thumbnails for the trim timeline, as file paths the window may load.
#[tauri::command]
async fn filmstrip(
    state: State<'_, AppState>,
    path: PathBuf,
    duration: f64,
    count: usize,
) -> CommandResult<Vec<PathBuf>> {
    let tools = state.engine.tools().map_err(text)?;
    thumbs::filmstrip(&tools, &path, duration, count, 96, &state.thumbs_dir).await.map_err(text)
}

/// One frame as a `data:` URL, for sources the window cannot play itself.
#[tauri::command]
async fn preview_frame(state: State<'_, AppState>, path: PathBuf, at: f64) -> CommandResult<String> {
    let tools = state.engine.tools().map_err(text)?;
    let jpeg = thumbs::frame(&tools, &path, at, 720).await.map_err(text)?;
    Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpeg)))
}

/// What a job would do, without doing it. Cheap; the window calls it while
/// a trim handle is being dragged.
#[tauri::command]
fn estimate(info: MediaInfo, trim: Option<Trim>, options: EncodeOptions) -> CommandResult<Plan> {
    plan::plan(&info, trim, &options).map_err(text)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    path: PathBuf,
    trim: Option<Trim>,
    options: EncodeOptions,
    /// What to call the result, as typed.
    #[serde(default)]
    output_name: Option<String>,
}

/// Queues one job per video and returns their ids in the same order.
#[tauri::command]
async fn compress(state: State<'_, AppState>, jobs: Vec<Request>) -> CommandResult<Vec<JobId>> {
    state.engine.tools().map_err(text)?;
    let settings = state.engine.settings().get();
    let output_dir = match settings.output_mode {
        OutputMode::Folder => settings.output_folder.clone(),
        OutputMode::SameFolder | OutputMode::Replace => None,
    };
    let replace_original = settings.output_mode == OutputMode::Replace;
    Ok(jobs
        .into_iter()
        .map(|job| {
            state.engine.submit(JobSpec {
                input: job.path,
                trim: job.trim,
                options: job.options,
                output_dir: output_dir.clone(),
                origin: Origin::Manual,
                output_name: job.output_name,
                replace_original,
            })
        })
        .collect())
}

#[tauri::command]
async fn cancel(state: State<'_, AppState>, id: String) -> CommandResult<bool> {
    Ok(state.engine.cancel(&id))
}

#[tauri::command]
async fn cancel_all(state: State<'_, AppState>) -> CommandResult<()> {
    state.engine.cancel_all();
    Ok(())
}

#[tauri::command]
async fn save_settings(state: State<'_, AppState>, settings: Settings) -> CommandResult<Settings> {
    let before = state.engine.settings().get();
    let saved = state.engine.settings().set(settings).map_err(text)?;
    if saved.ffmpeg_folder != before.ffmpeg_folder {
        state.engine.forget_tools().await;
    }
    if saved.watch != before.watch
        || saved.profiles != before.profiles
        || saved.default_profile != before.default_profile
    {
        state.watcher.reload();
    }
    if saved.history_keep_days != before.history_keep_days {
        let _ = state.engine.history().prune(saved.history_keep_days);
    }
    state.engine.settings_changed();
    Ok(saved)
}

/// Which encoders work on this machine. `refresh` runs the tests again.
#[tauri::command]
async fn encoders(state: State<'_, AppState>, refresh: bool) -> CommandResult<Report> {
    state.engine.hardware(refresh).await.map(|report| (*report).clone()).map_err(text)
}

#[tauri::command]
async fn history_list(state: State<'_, AppState>) -> CommandResult<Vec<Entry>> {
    Ok(state.engine.history().list())
}

#[tauri::command]
async fn history_remove(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    state.engine.history().remove(&id).map_err(text)
}

#[tauri::command]
async fn history_clear(state: State<'_, AppState>) -> CommandResult<()> {
    state.engine.history().clear().map_err(text)
}

#[tauri::command]
async fn watch_status(state: State<'_, AppState>) -> CommandResult<WatchStatus> {
    Ok(state.watcher.status())
}

/// Shows a file in Finder / Explorer / the file manager.
#[tauri::command]
fn reveal(app: AppHandle, path: PathBuf) -> CommandResult<()> {
    if !path.exists() {
        return Err("That file is no longer there.".into());
    }
    app.opener().reveal_item_in_dir(&path).map_err(text)
}

/// Opens a finished video in the default player.
#[tauri::command]
fn open_file(app: AppHandle, path: PathBuf) -> CommandResult<()> {
    if !path.exists() {
        return Err("That file is no longer there.".into());
    }
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(text)
}

/// Refuses paths this app did not write.
fn produced(state: &AppState, path: &Path) -> CommandResult<()> {
    if !state.results.lock().expect("results lock").contains(path) {
        return Err("Only videos Sub20 made can be used here.".into());
    }
    if !path.is_file() {
        return Err("That file is no longer there.".into());
    }
    Ok(())
}

/// Lets the window play a finished video, for the Result view of the stage.
#[tauri::command]
fn result_url(app: AppHandle, state: State<'_, AppState>, path: PathBuf) -> CommandResult<Option<String>> {
    produced(&state, &path)?;
    app.asset_protocol_scope().allow_file(&path).map_err(text)?;
    Ok(state.media.as_ref().map(|server| server.url_for(&path)))
}

/// Renames a finished video to what was typed, keeping its extension.
/// Returns the new path.
#[tauri::command]
fn rename_result(app: AppHandle, state: State<'_, AppState>, path: PathBuf, name: String) -> CommandResult<PathBuf> {
    produced(&state, &path)?;
    let renamed = state.engine.rename_result(&path, &name).map_err(text)?;
    let mut results = state.results.lock().expect("results lock");
    results.remove(&path);
    results.insert(renamed.clone());
    let _ = app.asset_protocol_scope().allow_file(&renamed);
    Ok(renamed)
}

/// Puts a finished video on the clipboard as a file, so pasting into Discord
/// (or a folder) attaches it.
#[tauri::command]
fn copy_file(state: State<'_, AppState>, path: PathBuf) -> CommandResult<()> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    produced(&state, &path)?;
    let file = path.to_string_lossy().into_owned();
    let clipboard = ClipboardContext::new().map_err(|e| format!("The clipboard could not be opened: {e}"))?;
    clipboard.set_files(vec![file]).map_err(|e| format!("The clipboard refused the file: {e}"))
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            let thumbs_dir = cache_dir.join("thumbs");
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&thumbs_dir)?;
            thumbs::prune_cache(&thumbs_dir, Duration::from_secs(14 * 86_400));
            // Scratch left over from a crash.
            let _ = std::fs::remove_dir_all(cache_dir.join("work"));
            app.asset_protocol_scope().allow_directory(&thumbs_dir, true)?;

            // An FFmpeg shipped inside the app wins over one on PATH.
            let mut bundled = Vec::new();
            if let Ok(resources) = app.path().resource_dir() {
                bundled.push(resources.join("ffmpeg"));
                bundled.push(resources);
            }
            if let Some(beside) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)) {
                bundled.push(beside);
            }

            let results: Arc<Mutex<HashSet<PathBuf>>> = Arc::default();
            let produced = results.clone();
            let window = app.handle().clone();
            let sink = Arc::new(move |event: JobEvent| {
                if let JobEvent::Done { result, .. } = &event {
                    produced.lock().expect("results lock").insert(result.output.clone());
                }
                let _ = window.emit("job", event);
            });
            let engine = Engine::new(&data_dir, &cache_dir.join("work"), bundled, sink);
            let watcher = Arc::new(Watcher::new(engine.clone(), &data_dir));

            // Test the encoders and start the folder watch in the background.
            let (warm_engine, warm_watcher) = (engine.clone(), watcher.clone());
            tauri::async_runtime::spawn(async move {
                warm_watcher.reload();
                let _ = warm_engine.hardware(false).await;
            });

            // Files handed over by "Open with" or dropped on the app icon.
            let opened_with: Vec<PathBuf> = std::env::args_os()
                .skip(1)
                .map(PathBuf::from)
                .filter(|path| path.exists())
                .filter_map(|path| std::path::absolute(path).ok())
                .collect();

            let media = if cfg!(target_os = "linux") {
                tauri::async_runtime::block_on(media::MediaServer::start())
                    .map_err(|e| log::warn!("video previews will show still frames: {e}"))
                    .ok()
            } else {
                None
            };

            app.manage(AppState { engine, watcher, thumbs_dir, opened_with, media, results });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            ffmpeg_status,
            add_videos,
            filmstrip,
            preview_frame,
            estimate,
            compress,
            cancel,
            cancel_all,
            save_settings,
            encoders,
            history_list,
            history_remove,
            history_clear,
            watch_status,
            reveal,
            open_file,
            result_url,
            copy_file,
            rename_result,
        ])
        .build(tauri::generate_context!())
        .expect("failed to start Sub20");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // Stop FFmpeg before the process goes away, so nothing keeps
            // encoding in the background and no partial files are left.
            if let Some(state) = app.try_state::<AppState>() {
                if state.engine.active() > 0 {
                    state.engine.cancel_all();
                    std::thread::sleep(Duration::from_millis(600));
                }
            }
        }
    });
}
