//! The desktop shell: a window, and the bridge between it and the engine.
//!
//! Everything that matters lives in `sub20-core`. This file only exposes
//! it to the window as commands and forwards job events back.

mod media;
mod notify;
#[cfg(windows)]
mod sign_in;
#[cfg(windows)]
mod tray;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sub20_core::bench::{self, BenchClip};
use sub20_core::crawl::{self, Found};
use sub20_core::cut::Cut;
use sub20_core::hardware::Report;
use sub20_core::history::Entry;
use sub20_core::job::{EncodeOptions, JobEvent, JobId, JobSpec, Origin, Trim};
use sub20_core::plan::{self, Plan};
use sub20_core::settings::{OutputMode, Settings};
use sub20_core::watch::{self, Arrival, WatchStatus, Watcher};
use sub20_core::watched::{WatchedJob, WatchedJobs};
use sub20_core::{fetch, thumbs, Engine, MediaInfo};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

struct AppState {
    engine: Engine,
    watcher: Arc<Watcher>,
    thumbs_dir: PathBuf,
    /// Serves previews over loopback HTTP where the asset protocol cannot
    /// play video (Linux).
    media: Option<media::MediaServer>,
    /// Files this app has written. Only these may be previewed or copied to
    /// the clipboard as results.
    results: Arc<Mutex<HashSet<PathBuf>>>,
    /// The update `check_update` found, kept for `install_update`.
    update: Mutex<Option<Update>>,
    /// The user chose "Stop and quit"; let the exit through.
    quitting: AtomicBool,
    /// The quit dialog is showing.
    asking: AtomicBool,
    /// Where panics are written (`log_crashes`).
    crash_log: PathBuf,
    ffmpeg_notice: Option<PathBuf>,
    /// Where a downloaded FFmpeg is installed (`download_ffmpeg`).
    fetched_ffmpeg: PathBuf,
    /// Scratch for a download in progress.
    fetch_dir: PathBuf,
    /// Folders the user chose in `pick_folder` this session. A new FFmpeg
    /// folder must be one of them, since the FFmpeg found there runs.
    picked: Mutex<HashSet<PathBuf>>,
    /// This session's watch jobs, kept here so a window built again after
    /// closing still lists them.
    watched: Arc<Mutex<WatchedJobs>>,
    /// The bench as it was left (`sub20_core::bench`).
    bench_file: PathBuf,
    /// An update is downloading or installing, from the window or the tray.
    installing: AtomicBool,
    /// Written just before an update installs, so the new version comes
    /// back the way this one was: in the tray or with the window
    /// (`back_from_update`).
    tray_record: PathBuf,
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
    /// Why the FFmpeg in `folder` didn't start, when it was found but
    /// wouldn't run (`found` is then false).
    problem: Option<String>,
}

async fn ffmpeg_state(engine: &Engine) -> FfmpegStatus {
    let Ok(tools) = engine.tools() else {
        return FfmpegStatus { found: false, version: None, folder: None, problem: None };
    };
    let folder = tools.ffmpeg.parent().map(Path::to_path_buf);
    match tools.check().await {
        Ok(version) => FfmpegStatus { found: true, version: Some(version), folder, problem: None },
        // There, but it won't run: the window treats it as missing and says
        // why, and the next look starts afresh.
        Err(why) => {
            log::warn!("FFmpeg in {} did not start: {why}", folder.as_deref().unwrap_or(Path::new("?")).display());
            engine.forget_tools().await;
            FfmpegStatus { found: false, version: None, folder, problem: Some(why) }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    settings: Settings,
    ffmpeg: FfmpegStatus,
    version: &'static str,
    opened_with: Vec<PathBuf>,
    /// Whether FFmpeg ships inside this build.
    bundled: bool,
    /// The licence and origin of the FFmpeg inside, when there is one.
    ffmpeg_notice: Option<PathBuf>,
    /// The FFmpeg this system can download when it has none.
    ffmpeg_offer: Option<fetch::Offer>,
    /// This session's watch jobs, newest first.
    watched: Vec<WatchedJob>,
    /// A watch notice clicked while there was no window: what it opens.
    notice: Option<notify::Clicked>,
    /// The bench as it was left, with what the watch put on it meanwhile
    /// (plan 7's 7.13).
    bench: Vec<BenchClip>,
}

/// Everything the window needs when it opens.
#[tauri::command]
async fn bootstrap(state: State<'_, AppState>) -> CommandResult<Bootstrap> {
    // From here on, opened videos, notice clicks and clips the watch
    // benches go to the window as events. The locks are held while it
    // becomes ready, so nothing slips between being held for it and being
    // sent to it.
    let (opened_with, notice, bench) = {
        let mut waiting = OPENED.lock().unwrap_or_else(|e| e.into_inner());
        let mut notice = NOTICE.lock().unwrap_or_else(|e| e.into_inner());
        WINDOW_READY.store(true, Ordering::SeqCst);
        (in_name_order(std::mem::take(&mut *waiting)), notice.take(), bench::load(&state.bench_file))
    };
    Ok(Bootstrap {
        bench,
        settings: state.engine.settings().get(),
        ffmpeg: ffmpeg_state(&state.engine).await,
        version: env!("CARGO_PKG_VERSION"),
        watched: state.watched.lock().expect("watched lock").list(),
        notice,
        opened_with,
        bundled: bundled(),
        ffmpeg_notice: state.ffmpeg_notice.clone(),
        ffmpeg_offer: fetch::offer(),
    })
}

#[tauri::command]
async fn ffmpeg_status(state: State<'_, AppState>) -> CommandResult<FfmpegStatus> {
    Ok(ffmpeg_state(&state.engine).await)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Fetching {
    /// "downloading", "checking" or "installing".
    stage: &'static str,
    /// How much of the download has arrived, 0 to 1, when its size is known.
    fraction: Option<f64>,
    /// Bytes received so far, across all files.
    received: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Fetched {
    ffmpeg: FfmpegStatus,
    notice: PathBuf,
}

static FETCHING: AtomicBool = AtomicBool::new(false);

/// Downloads the FFmpeg pinned for this system from Sub20's release page,
/// checks its SHA-256, and installs it where the engine looks.
#[tauri::command]
async fn download_ffmpeg(state: State<'_, AppState>, on_progress: Channel<Fetching>) -> CommandResult<Fetched> {
    if FETCHING.swap(true, Ordering::SeqCst) {
        return Err("FFmpeg is already downloading.".into());
    }
    let _ = std::fs::remove_dir_all(&state.fetch_dir);
    let fetched = fetch_ffmpeg(&state, &on_progress).await;
    let _ = std::fs::remove_dir_all(&state.fetch_dir);
    FETCHING.store(false, Ordering::SeqCst);
    let notice = fetched?;
    state.engine.forget_tools().await;
    Ok(Fetched { ffmpeg: ffmpeg_state(&state.engine).await, notice })
}

async fn fetch_ffmpeg(state: &AppState, on_progress: &Channel<Fetching>) -> CommandResult<PathBuf> {
    use tokio::io::AsyncWriteExt;

    let offer = fetch::offer().ok_or("Sub20 has no FFmpeg download for this system. Install FFmpeg yourself.")?;
    std::fs::create_dir_all(&state.fetch_dir).map_err(text)?;
    // The updater's TLS runs on ring; this uses the same.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .user_agent(concat!("Sub20/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(text)?;

    let count = offer.files.len() as f64;
    let mut received = 0;
    let mut archives = Vec::new();
    for (index, file) in offer.files.iter().enumerate() {
        let mut response = client.get(&file.url).send().await.map_err(|e| {
            format!("Couldn't reach GitHub to download FFmpeg: {e}. Check the connection and try again.")
        })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(
                "FFmpeg is no longer where this version of Sub20 expects it. Update Sub20, or install FFmpeg yourself."
                    .into(),
            );
        }
        if !response.status().is_success() {
            return Err(format!("GitHub answered {} to the FFmpeg download. Try again in a while.", response.status()));
        }
        let size = response.content_length();
        let path = state.fetch_dir.join(&file.name);
        let mut out = tokio::fs::File::create(&path).await.map_err(text)?;
        let mut this_file = 0;
        let mut told = std::time::Instant::now();
        on_progress.send(Fetching { stage: "downloading", fraction: Some(index as f64 / count), received }).ok();
        while let Some(chunk) =
            response.chunk().await.map_err(|e| format!("The FFmpeg download stopped: {e}. Try again."))?
        {
            out.write_all(&chunk).await.map_err(|e| format!("Couldn't save the FFmpeg download: {e}"))?;
            this_file += chunk.len() as u64;
            received += chunk.len() as u64;
            if told.elapsed() >= Duration::from_millis(100) {
                told = std::time::Instant::now();
                let fraction = size.map(|size| (index as f64 + (this_file as f64 / size as f64).min(1.0)) / count);
                on_progress.send(Fetching { stage: "downloading", fraction, received }).ok();
            }
        }
        out.flush().await.map_err(text)?;
        drop(out);

        on_progress.send(Fetching { stage: "checking", fraction: None, received }).ok();
        let (checked, sha256) = (path.clone(), file.sha256.clone());
        tokio::task::spawn_blocking(move || fetch::check(&checked, &sha256)).await.map_err(text)?.map_err(text)?;
        archives.push(path);
    }

    on_progress.send(Fetching { stage: "installing", fraction: None, received }).ok();
    let unpacked = state.fetch_dir.join("unpacked");
    for archive in &archives {
        fetch::unpack(archive, &unpacked).await.map_err(text)?;
    }
    let dest = state.fetched_ffmpeg.clone();
    tokio::task::spawn_blocking(move || fetch::install(&unpacked, &dest, &offer)).await.map_err(text)?.map_err(text)
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

/// The files to add for what was dropped or chosen: folders are searched
/// with their subfolders (`sub20_core::crawl`).
#[tauri::command]
async fn find_videos(paths: Vec<PathBuf>) -> CommandResult<Found> {
    tokio::task::spawn_blocking(move || crawl::find_videos(&paths)).await.map_err(text)
}

/// Reads the facts of each file. The window sends a few at a time, so they
/// are read side by side and each shows as soon as it is read; a folder
/// here is searched as `find_videos` does.
#[tauri::command]
async fn add_videos(app: AppHandle, state: State<'_, AppState>, paths: Vec<PathBuf>) -> CommandResult<Vec<Added>> {
    let files = tokio::task::spawn_blocking(move || crawl::find_videos(&paths).files).await.map_err(text)?;

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

#[derive(Clone, Serialize)]
struct Thumb {
    index: usize,
    path: PathBuf,
}

/// Thumbnails for the trim timeline, as file paths the window may load.
/// Each one is also sent through `on_thumb` as soon as it exists, so the
/// strip fills in while the rest are cut.
#[tauri::command]
async fn filmstrip(
    state: State<'_, AppState>,
    path: PathBuf,
    duration: f64,
    count: usize,
    on_thumb: Channel<Thumb>,
) -> CommandResult<Vec<PathBuf>> {
    local(&path)?;
    let tools = state.engine.tools().map_err(text)?;
    thumbs::filmstrip(&tools, &path, duration, count, 96, &state.thumbs_dir, |index, path| {
        let _ = on_thumb.send(Thumb { index, path: path.to_path_buf() });
    })
    .await
    .map_err(text)
}

/// One frame as a `data:` URL, for sources the window cannot play itself.
#[tauri::command]
async fn preview_frame(state: State<'_, AppState>, path: PathBuf, at: f64) -> CommandResult<String> {
    local(&path)?;
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
    for job in &jobs {
        local(&job.path)?;
    }
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
async fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> CommandResult<Settings> {
    let before = state.engine.settings().get();
    if let Some(folder) = &settings.ffmpeg_folder {
        let known =
            before.ffmpeg_folder.as_ref() == Some(folder) || state.picked.lock().expect("picked lock").contains(folder);
        if !known {
            return Err("That FFmpeg folder wasn't chosen in the folder dialog. Choose it again.".into());
        }
    }
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
    follow_tray(&app, &saved);
    follow_sign_in(&app, &saved);
    Ok(saved)
}

/// Keeps the bench as the window has it now, for the next launch or the
/// next window (plan 7's 7.13). The window calls it a moment after the
/// bench changes.
#[tauri::command]
async fn save_bench(state: State<'_, AppState>, clips: Vec<BenchClip>) -> CommandResult<()> {
    // The same lock as the watch's additions, so the two never interleave.
    let _turn = OPENED.lock().unwrap_or_else(|e| e.into_inner());
    bench::save(&state.bench_file, &clips).map_err(text)
}

/// Clips the folder watch puts on the bench: saved with it first, so they
/// wait there while there is no window, then sent to the window if there is
/// one. Results Sub20 made are left out (a name typed for a result isn't
/// one the watch recognises as its own).
fn bench_arrivals(
    app: &AppHandle,
    file: &Path,
    results: &Mutex<HashSet<PathBuf>>,
    notices: &Arc<notify::Notices>,
    paths: Vec<PathBuf>,
) {
    let paths: Vec<PathBuf> = {
        let made = results.lock().expect("results lock");
        paths.into_iter().filter(|path| !made.contains(path)).collect()
    };
    if paths.is_empty() {
        return;
    }
    {
        let _turn = OPENED.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = bench::add(file, &paths) {
            log::warn!("could not save the bench: {e}");
        }
        if WINDOW_READY.load(Ordering::SeqCst) {
            let _ = app.emit("benched", paths.clone());
        }
    }
    // With no window, a notice says they are waiting (plan 7's 7.6).
    if window_gone(app) && wants_notices(app) {
        notices.benched(app, &paths);
    }
}

/// The watch's notices are switched on.
fn wants_notices(app: &AppHandle) -> bool {
    app.try_state::<AppState>().is_some_and(|state| state.engine.settings().get().watch.notify)
}

/// A folder chosen in the system's dialog, or `None` if it was closed. The
/// shell opens the dialog, not the window, so it knows which folders the
/// user really chose (`AppState::picked`).
#[tauri::command]
async fn pick_folder(app: AppHandle, state: State<'_, AppState>) -> CommandResult<Option<PathBuf>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut dialog = app.dialog().file().set_title("Choose a folder");
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    dialog.pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let Some(folder) = rx.await.map_err(text)? else { return Ok(None) };
    let path = folder.into_path().map_err(text)?;
    state.picked.lock().expect("picked lock").insert(path.clone());
    Ok(Some(path))
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

/// Compress the clips that arrived while Sub20 was closed, or skip them.
#[tauri::command]
async fn watch_decide(state: State<'_, AppState>, compress: bool) -> CommandResult<WatchStatus> {
    state.watcher.decide(compress);
    Ok(state.watcher.status())
}

/// Pauses the watch for this session, or watches again.
#[tauri::command]
async fn watch_pause(app: AppHandle, state: State<'_, AppState>, paused: bool) -> CommandResult<WatchStatus> {
    set_paused(&app, &state.watcher, paused);
    Ok(state.watcher.status())
}

/// Pauses or resumes the watch, and the tray's menu follows. Needs the
/// runtime, like `Watcher::reload`.
pub(crate) fn set_paused(app: &AppHandle, watcher: &Watcher, paused: bool) {
    if paused {
        watcher.pause();
    } else {
        watcher.resume();
    }
    #[cfg(windows)]
    tray::refresh(app);
    #[cfg(not(windows))]
    let _ = app;
}

/// The tray's "Pause watching" or "Resume watching".
#[cfg(windows)]
pub(crate) fn toggle_pause(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let paused = !state.watcher.status().paused;
    set_paused(app, &state.watcher, paused);
}

/// Recording apps' clip folders found on this machine, for the watch shortcuts.
#[tauri::command]
fn clip_folders(app: AppHandle) -> Vec<watch::ClipFolder> {
    app.path().video_dir().map(|videos| watch::clip_folders(&videos)).unwrap_or_default()
}

/// Shows a file in Explorer or the file manager.
#[tauri::command]
fn reveal(app: AppHandle, path: PathBuf) -> CommandResult<()> {
    if !path.exists() {
        return Err("That file is no longer there.".into());
    }
    app.opener().reveal_item_in_dir(&path).map_err(text)
}

/// Opens a finished video in the default player, or FFmpeg's licence
/// notice. Nothing else: the system's default program for a path the
/// window names could be the program itself.
#[tauri::command]
fn open_file(app: AppHandle, state: State<'_, AppState>, path: PathBuf) -> CommandResult<()> {
    let notice =
        state.ffmpeg_notice.as_deref() == Some(path.as_path()) || path == state.fetched_ffmpeg.join("FFMPEG.txt");
    if notice {
        if !path.is_file() {
            return Err("That file is no longer there.".into());
        }
    } else {
        produced(&state, &path)?;
    }
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(text)
}

/// Refuses a source that isn't an absolute path. FFmpeg reads `http://…` or
/// `concat:…` as a URL; what the window sends always came from a drop, a
/// dialog or the system, and those are absolute.
fn local(path: &Path) -> CommandResult<()> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err("That isn't a file Sub20 can read.".into())
    }
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

/// A finished video's keyframe times, where a cut of it can start.
#[tauri::command]
async fn keyframes(state: State<'_, AppState>, path: PathBuf) -> CommandResult<Vec<f64>> {
    produced(&state, &path)?;
    state.engine.keyframes(&path).await.map_err(text)
}

/// Cuts a finished video shorter by copying its packets; the uncut file
/// goes to the bin.
#[tauri::command]
async fn cut_result(
    app: AppHandle,
    state: State<'_, AppState>,
    path: PathBuf,
    start: f64,
    end: f64,
) -> CommandResult<Cut> {
    produced(&state, &path)?;
    let cut = state.engine.cut_result(&path, start, end).await.map_err(text)?;
    let mut results = state.results.lock().expect("results lock");
    if !path.exists() {
        results.remove(&path);
    }
    results.insert(cut.output.clone());
    let _ = app.asset_protocol_scope().allow_file(&cut.output);
    Ok(cut)
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

/// Videos handed to Sub20 by the system ("Open with", Explorer's "Compress
/// with Sub20", a file manager) that the window hasn't taken yet. Static,
/// because a second launch can hand files over before the app's state
/// exists.
static OPENED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// Set when the window takes the waiting videos; later ones are sent to it
/// as `opened` events. Cleared when the window is destroyed, so they wait
/// again for the next one.
static WINDOW_READY: AtomicBool = AtomicBool::new(false);
/// A watch notice clicked while no window was ready to hear it: what it
/// opens, until `bootstrap` takes it.
static NOTICE: Mutex<Option<notify::Clicked>> = Mutex::new(None);

/// There is no window, and none is being built: nobody sees the bench or the
/// question about a pile, so a notice says so.
pub(crate) fn window_gone(app: &AppHandle) -> bool {
    app.get_webview_window("main").is_none() && !BUILDING.load(Ordering::SeqCst)
}

/// Brings the main window forward, building it again from its config if it
/// was closed.
pub(crate) fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // One build at a time: Explorer starts Sub20 once per selected file, so
    // several launches can ask for the window within milliseconds.
    if BUILDING.swap(true, Ordering::SeqCst) {
        return;
    }
    // Built on a thread of its own: on Windows, building a window inside an
    // event handler (a tray click, a second launch) can deadlock WebView2.
    let app = app.clone();
    std::thread::spawn(move || {
        if let Some(config) = app.config().app.windows.iter().find(|w| w.label == "main") {
            if let Err(e) = tauri::WebviewWindowBuilder::from_config(&app, config).and_then(|b| b.build()) {
                log::warn!("could not open the window: {e}");
            }
        }
        BUILDING.store(false, Ordering::SeqCst);
    });
}

/// The main window is being built (`show_window`).
static BUILDING: AtomicBool = AtomicBool::new(false);

/// Whether the watch is to keep going from the tray once the window closes:
/// a watch on, `keep_watching` on, and the tray icon really there, so the
/// process is never left with no way back to it. Windows only (plan 7).
#[cfg(windows)]
fn keeps_watching(settings: &Settings) -> bool {
    settings.watch.enabled && settings.watch.keep_watching
}

/// Closing the window leaves the process running, in the tray.
fn stays_in_tray(app: &AppHandle) -> bool {
    #[cfg(windows)]
    {
        let Some(state) = app.try_state::<AppState>() else { return false };
        !state.quitting.load(Ordering::SeqCst) && keeps_watching(&state.engine.settings().get()) && tray::shown(app)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        false
    }
}

/// Puts the tray icon up or takes it down to match the settings.
fn follow_tray(app: &AppHandle, settings: &Settings) {
    #[cfg(windows)]
    tray::follow(app, keeps_watching(settings));
    #[cfg(not(windows))]
    let _ = (app, settings);
}

/// The argument a start at sign-in carries (`sign_in`).
const TRAY_ARG: &str = "--tray";

/// Sub20 is to start in the tray at sign-in: the setting, with the watch
/// it keeps going (Windows; plan 7's 7.9).
fn starts_at_sign_in(settings: &Settings) -> bool {
    cfg!(windows) && settings.watch.enabled && settings.watch.keep_watching && settings.watch.start_at_sign_in
}

/// Writes or removes the Run value to match the settings. Never from
/// `tauri dev` or a plain debug build, which share the installed app's
/// identifier and settings and would point the value at a build folder.
fn follow_sign_in(app: &AppHandle, settings: &Settings) {
    #[cfg(windows)]
    if !tauri::is_dev() {
        if let Err(e) = sign_in::follow(&app.config().identifier, starts_at_sign_in(settings)) {
            log::warn!("couldn't change the start at sign-in: {e}");
        }
    }
    #[cfg(not(windows))]
    let _ = (app, settings);
}

/// A watch notice was clicked: the window comes forward, built again if it
/// was closed, and opens what it names (see `notify::listen`).
pub(crate) fn notice_clicked(app: &AppHandle, clicked: notify::Clicked) {
    {
        let mut held = NOTICE.lock().unwrap_or_else(|e| e.into_inner());
        if !WINDOW_READY.load(Ordering::SeqCst) {
            *held = Some(clicked);
            drop(held);
            show_window(app);
            return;
        }
    }
    show_window(app);
    let _ = app.emit("notice", clicked);
}
/// A batch of opened videos is being gathered for the window.
static GATHERING: AtomicBool = AtomicBool::new(false);
/// How long no more videos must arrive before a batch goes to the window.
/// Explorer starts Sub20 once per selected file, so twenty selected videos
/// arrive as twenty launches a few milliseconds apart.
const OPENED_QUIET: Duration = Duration::from_millis(150);

/// One batch of opened videos as the window gets it: each once, in name
/// order as Explorer shows it (clip2 before clip10), since the
/// launches that brought them arrive in no useful order.
fn in_name_order(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths.retain(|path| seen.insert(path.clone()));
    let name = |path: &PathBuf| path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    paths.sort_by(|a, b| natural(&name(a), &name(b)).then_with(|| a.cmp(b)));
    paths
}

/// Compares names with runs of digits taken as numbers.
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let run = |chars: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(c) = chars.next_if(char::is_ascii_digit) {
                        digits.push(c);
                    }
                    digits.trim_start_matches('0').to_string()
                };
                let (x, y) = (run(&mut a), run(&mut b));
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(&y));
                if order.is_ne() {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                a.next();
                b.next();
            }
        }
    }
}

/// The videos among command-line arguments, made absolute against the folder
/// they were given in.
fn opened_paths<P: Into<PathBuf>>(args: impl IntoIterator<Item = P>, cwd: &Path) -> Vec<PathBuf> {
    args.into_iter()
        .map(Into::into)
        .map(|path| if path.is_absolute() { path } else { cwd.join(path) })
        .filter(|path| path.exists())
        .collect()
}

/// Brings the window forward, built again if it was closed, with the videos
/// a second launch asked to open (none for a plain launch).
fn hand_over(app: &AppHandle, paths: Vec<PathBuf>) {
    show_window(app);
    if paths.is_empty() {
        return;
    }
    let mut waiting = OPENED.lock().unwrap_or_else(|e| e.into_inner());
    waiting.extend(paths);
    // Before the window is up, `bootstrap` takes everything at once. After,
    // videos that arrive together go to it together, once they stop coming.
    if WINDOW_READY.load(Ordering::SeqCst) && !GATHERING.swap(true, Ordering::SeqCst) {
        let app = app.clone();
        std::thread::spawn(move || {
            let mut counted = usize::MAX;
            loop {
                std::thread::sleep(OPENED_QUIET);
                let mut waiting = OPENED.lock().unwrap_or_else(|e| e.into_inner());
                if waiting.len() == counted {
                    GATHERING.store(false, Ordering::SeqCst);
                    // The window closed while they gathered: they wait for
                    // the next one.
                    if !WINDOW_READY.load(Ordering::SeqCst) {
                        return;
                    }
                    let batch = in_name_order(std::mem::take(&mut *waiting));
                    drop(waiting);
                    let _ = app.emit("opened", batch);
                    return;
                }
                counted = waiting.len();
            }
        });
    }
}

/// Whether this is the download with FFmpeg inside: its FFmpeg sits beside
/// the program as `sub20-ffmpeg`, where Tauri puts sidecars on every system.
/// Read from the install rather than compiled in, so one compile serves both
/// downloads.
fn bundled() -> bool {
    static BUNDLED: OnceLock<bool> = OnceLock::new();
    *BUNDLED.get_or_init(|| {
        std::env::current_exe().ok().and_then(|exe| exe.parent().map(has_bundled_ffmpeg)).unwrap_or(false)
    })
}

fn has_bundled_ffmpeg(dir: &Path) -> bool {
    dir.join(format!("sub20-ffmpeg{}", std::env::consts::EXE_SUFFIX)).is_file()
}

/// Which update feed this build follows, so an install keeps the kind it
/// started as: with FFmpeg inside or without ("lean"), and on Linux the
/// package it came in. The updater installs a deb with dpkg and an rpm with
/// rpm, and refuses anything else, so a deb install must be sent a deb.
fn update_target() -> String {
    use tauri::utils::{config::BundleType, platform::bundle_type};
    let package = match bundle_type() {
        Some(BundleType::Deb) => Some("deb"),
        Some(BundleType::Rpm) => Some("rpm"),
        _ => None,
    };
    update_key(std::env::consts::OS, std::env::consts::ARCH, package, bundled())
}

/// `linux-x86_64-deb-lean` and the like; the keys `scripts/collect-release.mjs`
/// writes into `latest.json`. An AppImage and the Windows installer have no
/// package part.
fn update_key(os: &str, arch: &str, package: Option<&str>, bundled: bool) -> String {
    let package = package.map(|p| format!("-{p}")).unwrap_or_default();
    let lean = if bundled { "" } else { "-lean" };
    format!("{os}-{arch}{package}{lean}")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    version: String,
    notes: Option<String>,
}

/// Asks the release feed for a newer version. `None` means up to date.
#[tauri::command]
async fn check_update(app: AppHandle) -> CommandResult<Option<UpdateInfo>> {
    find_update(&app).await
}

/// Asks the release feed, keeps what it offers for `install`, and puts it in
/// the tray's menu. The window asks at its start; the shell asks only when
/// Sub20 starts without one (plan 7's 7.7).
async fn find_update(app: &AppHandle) -> CommandResult<Option<UpdateInfo>> {
    let recorder = app.clone();
    let update = app
        .updater_builder()
        .target(update_target())
        .timeout(Duration::from_secs(20))
        // The last moment before the installer takes over: the new version
        // is to come back the way this one is, in the tray or with the
        // window. The updater starts it with this one's arguments, so a
        // copy started at sign-in (`--tray`) whose window was opened would
        // otherwise come back without it.
        .on_before_exit(move || {
            if let Some(state) = recorder.try_state::<AppState>() {
                let version = env!("CARGO_PKG_VERSION");
                let record = if window_gone(&recorder) { version.to_string() } else { format!("{version} window") };
                let _ = std::fs::write(&state.tray_record, record);
            }
        })
        .build()
        .map_err(text)?
        .check()
        .await
        .map_err(text)?;
    let info = update.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() });
    if let Some(state) = app.try_state::<AppState>() {
        *state.update.lock().expect("update lock") = update;
    }
    #[cfg(windows)]
    tray::refresh(app);
    Ok(info)
}

/// Downloads and installs the update `check_update` found, then restarts.
/// Running jobs are stopped first, as on quit.
#[tauri::command]
async fn install_update(app: AppHandle) -> CommandResult<()> {
    install(&app).await
}

/// Installs the update found last, one install at a time. On Windows the
/// installer takes over and the process exits inside; a failure is
/// returned, and the update stays offered so it can be tried again.
async fn install(app: &AppHandle) -> CommandResult<()> {
    let state = app.state::<AppState>();
    let update = state.update.lock().expect("update lock").clone().ok_or("No update has been found yet.")?;
    if state.installing.swap(true, Ordering::SeqCst) {
        return Err("The update is already installing.".into());
    }
    #[cfg(windows)]
    tray::refresh(app);
    if state.engine.active() > 0 {
        state.engine.cancel_all();
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
        // The installer may have failed to start after the record was
        // written: this version stays, so the record must not.
        let _ = std::fs::remove_file(&state.tray_record);
        state.installing.store(false, Ordering::SeqCst);
        #[cfg(windows)]
        tray::refresh(app);
        return Err(text(e));
    }
    // The jobs are already stopped; do not ask about them on the way out.
    state.quitting.store(true, Ordering::SeqCst);
    app.restart()
}

/// The tray's "Update to x.y.z": asks first while videos are compressing,
/// since installing stops them, and says so with a notice if it fails, as
/// there may be no window to say it in.
#[cfg(windows)]
pub(crate) async fn update_from_tray(app: AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let running = state.engine.active();
    if running > 0 {
        // One dialog at a time, shared with the quit question.
        if state.asking.swap(true, Ordering::SeqCst) {
            return;
        }
        let message = if running == 1 {
            "A video is still compressing. Updating stops it and deletes the unfinished file.".to_string()
        } else {
            format!("{running} videos are still compressing. Updating stops them and deletes the unfinished files.")
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.dialog()
            .message(message)
            .title("Stop compressing and update?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom("Stop and update".into(), "Keep compressing".into()))
            .show(move |stop| {
                let _ = tx.send(stop);
            });
        let stop = rx.await.unwrap_or(false);
        state.asking.store(false, Ordering::SeqCst);
        if !stop {
            return;
        }
    }
    if let Err(e) = install(&app).await {
        notify::update_failed(&app, &e);
    }
}

/// How the version an update installed is to come back.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Comeback {
    Tray,
    Window,
}

/// Reads and removes the record an update leaves (`find_update`): how this
/// start is to look when it is the new version that update installed.
/// Removed on every read, so a start by hand later is as usual.
fn back_from_update(record: &Path) -> Option<Comeback> {
    let recorded = std::fs::read_to_string(record).ok()?;
    let _ = std::fs::remove_file(record);
    comeback(&recorded, env!("CARGO_PKG_VERSION"))
}

/// The record names the version that wrote it, then "window" when its
/// window was open (versions before 0.9.6 removed the record instead).
/// The same version as this one means the install never happened.
fn comeback(recorded: &str, running: &str) -> Option<Comeback> {
    let mut words = recorded.split_whitespace();
    match words.next() {
        Some(version) if version != running => {}
        _ => return None,
    }
    Some(if words.next() == Some("window") { Comeback::Window } else { Comeback::Tray })
}

/// How a start looks.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Start {
    Window,
    Tray,
    /// A start at sign-in that is no longer wanted, or has no tray icon to
    /// go to: nothing to show, so nothing stays.
    Exit,
}

/// Videos handed over always open the window, and so does a start after
/// an update that was installed with the window open. An update from the
/// tray comes back in the tray when the icon is there. A start at sign-in
/// (`--tray`) goes to the tray when the setting still wants it and the
/// icon is up, and otherwise exits: it never opens a window by itself.
fn how_to_start(tray_arg: bool, comeback: Option<Comeback>, signs_in: bool, in_tray: bool, opened: bool) -> Start {
    match comeback {
        _ if opened => Start::Window,
        Some(Comeback::Window) => Start::Window,
        Some(Comeback::Tray) if in_tray => Start::Tray,
        Some(Comeback::Tray) => Start::Window,
        None if !tray_arg => Start::Window,
        None if signs_in && in_tray => Start::Tray,
        None => Start::Exit,
    }
}

/// Asks before quitting while videos are compressing or waiting. Returns
/// true when the quit may go ahead now; otherwise a dialog is up, and
/// "Stop and quit" exits the app. `manual_only` counts only the videos the
/// user asked for: closing the window into the tray leaves the watch's
/// running, so it asks about the others alone.
fn may_quit(app: &AppHandle, manual_only: bool) -> bool {
    let Some(state) = app.try_state::<AppState>() else { return true };
    let running = if manual_only { state.engine.active_manual() } else { state.engine.active() };
    if running == 0 || state.quitting.load(Ordering::SeqCst) {
        return true;
    }
    // One dialog at a time: closing the window again while it is up does nothing.
    if state.asking.swap(true, Ordering::SeqCst) {
        return false;
    }
    let message = if running == 1 {
        "A video is still compressing. Quitting stops it and deletes the unfinished file.".to_string()
    } else {
        format!("{running} videos are still compressing. Quitting stops them and deletes the unfinished files.")
    };
    let handle = app.clone();
    app.dialog()
        .message(message)
        .title("Stop compressing and quit?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Stop and quit".into(), "Keep compressing".into()))
        .show(move |stop| {
            let state = handle.state::<AppState>();
            state.asking.store(false, Ordering::SeqCst);
            if stop {
                state.quitting.store(true, Ordering::SeqCst);
                handle.exit(0);
            }
        });
    false
}

/// "FFmpeg included" or "without FFmpeg", as `update_target` decides.
fn build_kind() -> &'static str {
    if bundled() {
        "FFmpeg included"
    } else {
        "without FFmpeg"
    }
}

/// Writes every panic to `crash.log` in the app's log folder, after the usual
/// message. Copy diagnostics includes the end of the file.
fn log_crashes(path: PathBuf) {
    let usual = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        usual(info);
        let when =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let entry =
            format!("[{when}] Sub20 {} ({}), thread {thread}: {info}\n", env!("CARGO_PKG_VERSION"), build_kind());
        // Start over once the file gets large; the newest crash matters most.
        let fresh = std::fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false);
        let file = std::fs::OpenOptions::new().create(true).append(!fresh).write(true).truncate(fresh).open(&path);
        if let Ok(mut file) = file {
            use std::io::Write as _;
            let _ = file.write_all(entry.as_bytes());
        }
    }));
}

/// Puts what a bug report needs on the clipboard, ready to paste. `log` is
/// the FFmpeg log of the job the window is showing.
#[tauri::command]
async fn copy_diagnostics(app: AppHandle, state: State<'_, AppState>, log: Vec<String>) -> CommandResult<()> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let text = diagnostics(&app, &state, log).await;
    let clipboard = ClipboardContext::new().map_err(|e| format!("The clipboard could not be opened: {e}"))?;
    clipboard.set_text(text).map_err(|e| format!("The clipboard refused the text: {e}"))
}

async fn diagnostics(app: &AppHandle, state: &AppState, log: Vec<String>) -> String {
    let ffmpeg = state.engine.tools().ok();
    let checked = match &ffmpeg {
        Some(tools) => Some(tools.check().await),
        None => None,
    };
    let folder = ffmpeg.as_ref().and_then(|tools| tools.ffmpeg.parent().map(Path::to_path_buf));
    // Encoders are listed only from an FFmpeg that runs; one that doesn't
    // would list every encoder as missing.
    let report = match &checked {
        Some(Ok(_)) => state.engine.hardware(false).await.ok(),
        _ => None,
    };
    let info = os_info::get();
    let system = format!("{} {} ({})", info.os_type(), info.version(), std::env::consts::ARCH);
    let crashes = sub20_core::diagnostics::tail_of(&state.crash_log, 40);
    let home = app.path().home_dir().ok();
    let settings = state.engine.settings().get();
    sub20_core::diagnostics::report(&sub20_core::diagnostics::Facts {
        app_version: env!("CARGO_PKG_VERSION"),
        build: build_kind(),
        system: &system,
        ffmpeg: match (&checked, &folder) {
            (Some(Ok(version)), Some(folder)) => Some((version.as_str(), folder.as_path())),
            _ => None,
        },
        ffmpeg_problem: match (&checked, &folder) {
            (Some(Err(why)), Some(folder)) => Some((why.as_str(), folder.as_path())),
            _ => None,
        },
        report: report.as_deref(),
        settings: &settings,
        crashes: crashes.as_deref(),
        job_log: &log,
        home: home.as_deref(),
    })
}

pub fn run() {
    let app = tauri::Builder::default()
        // First, so a second launch ("Open with" while Sub20 is open, several
        // files from Explorer) hands its videos over and quits before
        // anything else starts.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // A start at sign-in while Sub20 already runs shows nothing.
            if argv.iter().any(|arg| arg == TRAY_ARG) {
                return;
            }
            hand_over(app, opened_paths(argv.into_iter().skip(1), Path::new(&cwd)));
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            let thumbs_dir = cache_dir.join("thumbs");
            let log_dir = app.path().app_log_dir()?;
            std::fs::create_dir_all(&log_dir)?;
            let crash_log = log_dir.join("crash.log");
            log_crashes(crash_log.clone());
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&thumbs_dir)?;
            thumbs::prune_cache(&thumbs_dir, Duration::from_secs(14 * 86_400));
            // Scratch left over from a crash.
            let _ = std::fs::remove_dir_all(cache_dir.join("work"));
            app.asset_protocol_scope().allow_directory(&thumbs_dir, true)?;

            // An FFmpeg shipped inside the app wins over one on PATH, and so
            // does one the lean build downloaded (`download_ffmpeg`).
            let fetched_ffmpeg = app.path().app_local_data_dir()?.join("ffmpeg");
            let mut bundled = Vec::new();
            let ffmpeg_notice = app
                .path()
                .resource_dir()
                .ok()
                .map(|resources| resources.join("ffmpeg").join("FFMPEG.txt"))
                .filter(|notice| notice.is_file())
                .or_else(|| Some(fetched_ffmpeg.join("FFMPEG.txt")).filter(|notice| notice.is_file()));
            if let Ok(resources) = app.path().resource_dir() {
                bundled.push(resources.join("ffmpeg"));
                bundled.push(resources);
            }
            if let Some(beside) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)) {
                bundled.push(beside);
            }
            bundled.push(fetched_ffmpeg.clone());

            let results: Arc<Mutex<HashSet<PathBuf>>> = Arc::default();
            let produced = results.clone();
            let window = app.handle().clone();
            notify::listen(app.handle());
            let notices = Arc::new(notify::Notices::default());
            let watched: Arc<Mutex<WatchedJobs>> = Arc::default();
            let following = watched.clone();
            let telling = notices.clone();
            let sink = Arc::new(move |event: JobEvent| {
                if let JobEvent::Done { result, .. } = &event {
                    produced.lock().expect("results lock").insert(result.output.clone());
                }
                following.lock().expect("watched lock").see(&event);
                notices.see(&window, &event, || wants_notices(&window));
                let _ = window.emit("job", event);
            });
            let engine = Engine::new(&data_dir, &cache_dir.join("work"), bundled, sink);
            let bench_file = data_dir.join("bench.json");
            let tray_record = data_dir.join("restart-in-tray");
            let (handing, bench_to, made) = (app.handle().clone(), bench_file.clone(), results.clone());
            let watcher = Arc::new(Watcher::new(
                engine.clone(),
                &data_dir,
                Arc::new(move |arrival| match arrival {
                    Arrival::Bench(paths) => bench_arrivals(&handing, &bench_to, &made, &telling, paths),
                    // With a window, the window asks; with none, a notice
                    // says there is a question (plan 7's 7.6).
                    Arrival::Pile(_) => {
                        if window_gone(&handing) && wants_notices(&handing) {
                            telling.pile(&handing);
                        }
                    }
                }),
            ));
            let (warm_engine, warm_watcher) = (engine.clone(), watcher.clone());

            // Files handed over by "Open with" or dropped on the app icon wait
            // for the window, which is built below once the state exists.
            let cwd = std::env::current_dir().unwrap_or_default();
            let args: Vec<_> = std::env::args_os().skip(1).collect();
            let tray_arg = cfg!(windows) && args.iter().any(|arg| arg == TRAY_ARG);
            OPENED
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend(opened_paths(args.into_iter().filter(|arg| arg != TRAY_ARG), &cwd));

            let media = if cfg!(target_os = "linux") {
                tauri::async_runtime::block_on(media::MediaServer::start())
                    .map_err(|e| log::warn!("video previews will show still frames: {e}"))
                    .ok()
            } else {
                None
            };

            app.manage(AppState {
                engine,
                watcher,
                thumbs_dir,
                media,
                results,
                update: Mutex::default(),
                quitting: AtomicBool::new(false),
                asking: AtomicBool::new(false),
                crash_log,
                ffmpeg_notice,
                fetched_ffmpeg,
                fetch_dir: cache_dir.join("ffmpeg-download"),
                picked: Mutex::default(),
                watched,
                bench_file,
                installing: AtomicBool::new(false),
                tray_record: tray_record.clone(),
            });
            let settings = app.state::<AppState>().engine.settings().get();
            follow_tray(app.handle(), &settings);
            follow_sign_in(app.handle(), &settings);
            // The window isn't made from the config at startup (`create:
            // false`), so it is built the same way here as when it comes
            // back after closing.
            let start = how_to_start(
                tray_arg,
                back_from_update(&tray_record),
                starts_at_sign_in(&settings),
                stays_in_tray(app.handle()),
                !OPENED.lock().unwrap_or_else(|e| e.into_inner()).is_empty(),
            );
            match start {
                Start::Window => show_window(app.handle()),
                Start::Tray => {}
                Start::Exit => {
                    app.handle().exit(0);
                    return Ok(());
                }
            }
            let windowless = start == Start::Tray;

            // Test the encoders and start the folder watch in the background,
            // once the window is on its way: a pile found by the first scan
            // is then the window's to ask about, not a notice's. With no
            // window, the shell checks for updates itself.
            let checker = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                warm_watcher.reload();
                if windowless && settings.check_updates {
                    let _ = find_update(&checker).await;
                }
                let _ = warm_engine.hardware(false).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            ffmpeg_status,
            download_ffmpeg,
            find_videos,
            add_videos,
            filmstrip,
            preview_frame,
            estimate,
            compress,
            cancel,
            cancel_all,
            save_settings,
            save_bench,
            pick_folder,
            encoders,
            history_list,
            history_remove,
            history_clear,
            watch_status,
            watch_decide,
            watch_pause,
            clip_folders,
            reveal,
            open_file,
            result_url,
            copy_file,
            rename_result,
            keyframes,
            cut_result,
            check_update,
            install_update,
            copy_diagnostics,
        ])
        .build(tauri::generate_context!())
        .expect("failed to start Sub20");

    app.run(|app, event| match event {
        RunEvent::WindowEvent { event: WindowEvent::CloseRequested { api, .. }, .. } => {
            // Into the tray, only the user's own videos are worth asking
            // about (plan 7's "Needs the owner" keeps that open).
            if !may_quit(app, stays_in_tray(app)) {
                api.prevent_close();
            }
        }
        RunEvent::WindowEvent { label, event: WindowEvent::Destroyed, .. } if label == "main" => {
            // Opened videos and notice clicks wait for the next window.
            let _waiting = OPENED.lock().unwrap_or_else(|e| e.into_inner());
            let _notice = NOTICE.lock().unwrap_or_else(|e| e.into_inner());
            WINDOW_READY.store(false, Ordering::SeqCst);
        }
        // No code: the last window closed. With the watch kept going, the
        // process stays in the tray. A code (Quit, an update's restart)
        // asks as before.
        RunEvent::ExitRequested { code: None, api, .. } if stays_in_tray(app) => api.prevent_exit(),
        RunEvent::ExitRequested { api, .. } => {
            if !may_quit(app, false) {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => {
            // Stop FFmpeg before the process goes away, so nothing keeps
            // encoding in the background and no partial files are left.
            if let Some(state) = app.try_state::<AppState>() {
                if state.engine.active() > 0 {
                    state.engine.cancel_all();
                    std::thread::sleep(Duration::from_millis(600));
                }
            }
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_download_with_ffmpeg_inside_is_told_by_its_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_bundled_ffmpeg(dir.path()));
        // A plain `ffmpeg` beside the program is someone's own, not ours.
        std::fs::write(dir.path().join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX)), b"x").unwrap();
        assert!(!has_bundled_ffmpeg(dir.path()));
        std::fs::write(dir.path().join(format!("sub20-ffmpeg{}", std::env::consts::EXE_SUFFIX)), b"x").unwrap();
        assert!(has_bundled_ffmpeg(dir.path()));
    }

    #[test]
    fn only_the_new_version_comes_back_as_the_old_one_was() {
        assert_eq!(comeback("0.9.6", "0.9.7"), Some(Comeback::Tray));
        assert_eq!(comeback("0.9.6\n", "0.9.7"), Some(Comeback::Tray));
        assert_eq!(comeback("0.9.6 window", "0.9.7"), Some(Comeback::Window));
        // The install never happened: start as usual.
        assert_eq!(comeback("0.9.7", "0.9.7"), None);
        assert_eq!(comeback("0.9.7 window", "0.9.7"), None);
        assert_eq!(comeback("", "0.9.7"), None);

        let dir = tempfile::tempdir().unwrap();
        let record = dir.path().join("restart-in-tray");
        assert_eq!(back_from_update(&record), None, "no record, start as usual");
        std::fs::write(&record, "0.0.1").unwrap();
        assert_eq!(back_from_update(&record), Some(Comeback::Tray));
        assert!(!record.exists(), "read once");
        assert_eq!(back_from_update(&record), None, "a later start is as usual");
    }

    #[test]
    fn a_start_at_sign_in_never_opens_the_window_by_itself() {
        use Start::*;
        // (--tray, after an update, setting wants it, icon up, videos handed over)
        let cases = [
            ((false, None, false, false, false), Window),
            ((false, None, true, true, false), Window),
            // Signed in: the tray, or nothing at all.
            ((true, None, true, true, false), Tray),
            ((true, None, false, true, false), Exit),
            ((true, None, true, false, false), Exit),
            ((true, None, false, false, false), Exit),
            // An update from the tray comes back there; without the icon,
            // the window, since the user just chose to update.
            ((false, Some(Comeback::Tray), false, true, false), Tray),
            ((false, Some(Comeback::Tray), false, false, false), Window),
            // The updater passes --tray on: a copy started at sign-in whose
            // window was open comes back with it, setting or not.
            ((true, Some(Comeback::Window), true, true, false), Window),
            ((true, Some(Comeback::Window), false, false, false), Window),
            ((true, Some(Comeback::Tray), false, true, false), Tray),
            // Videos always open the window.
            ((true, None, true, true, true), Window),
            ((false, Some(Comeback::Tray), true, true, true), Window),
        ];
        for ((tray_arg, comeback, signs_in, in_tray, opened), expected) in cases {
            assert_eq!(
                how_to_start(tray_arg, comeback, signs_in, in_tray, opened),
                expected,
                "{tray_arg} {comeback:?} {signs_in} {in_tray} {opened}"
            );
        }
    }

    #[test]
    fn the_uninstaller_removes_the_value_this_identifier_writes() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let id = config["identifier"].as_str().unwrap();
        let hooks = include_str!("../windows/hooks.nsh");
        for key in [
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        ] {
            let line = format!(r#"DeleteRegValue HKCU "{key}" "{id}""#);
            assert!(hooks.contains(&line), "hooks.nsh lacks: {line}");
        }
    }

    #[test]
    fn each_install_asks_for_updates_of_its_own_kind() {
        assert_eq!(update_key("windows", "x86_64", None, true), "windows-x86_64");
        assert_eq!(update_key("windows", "x86_64", None, false), "windows-x86_64-lean");
        assert_eq!(update_key("linux", "x86_64", None, true), "linux-x86_64");
        assert_eq!(update_key("linux", "x86_64", Some("deb"), true), "linux-x86_64-deb");
        assert_eq!(update_key("linux", "x86_64", Some("rpm"), false), "linux-x86_64-rpm-lean");
    }

    #[test]
    fn videos_opened_together_arrive_once_each_in_name_order() {
        let paths =
            ["b/clip10.mp4", "a/Clip2.mp4", "c/clip1.mp4", "a/Clip2.mp4", "d/clip02b.mp4"].map(PathBuf::from).to_vec();
        let ordered = in_name_order(paths);
        assert_eq!(ordered, ["c/clip1.mp4", "a/Clip2.mp4", "d/clip02b.mp4", "b/clip10.mp4"].map(PathBuf::from));
        // NVIDIA app names sort by when they were recorded.
        assert_eq!(
            natural("valorant 2026.11.12 - 21.03.44.02.mp4", "valorant 2026.11.12 - 9.10.00.01.mp4"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn opened_files_are_made_absolute_and_missing_ones_dropped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clip.mp4"), b"x").unwrap();
        let absolute = dir.path().join("clip.mp4");
        let found = opened_paths(["clip.mp4", "--flag", "gone.mp4", absolute.to_str().unwrap()], dir.path());
        assert_eq!(found, [absolute.clone(), absolute]);
    }
}
