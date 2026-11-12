//! Folder watch: compress whatever appears in a folder.
//!
//! The folder is scanned on a timer rather than through OS notifications.
//! That is slower to react but behaves the same on network shares, synced
//! folders and every platform, and it makes "has this file finished
//! copying?" easy to answer: wait until it stops changing.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::engine::{looks_generated, Engine};
use crate::history;
use crate::job::{JobSpec, Origin};
use crate::settings::{write_atomic, Existing, Originals, OutputMode, WatchSettings};

/// File types the watch picks up.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "webm", "m4v", "wmv", "flv", "mpeg", "mpg", "ts", "m2ts", "3gp", "3g2", "mts", "mxf",
    "ogv", "vob",
];

pub fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| VIDEO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Size and modification time: enough to tell whether a file changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Fingerprint {
    size: u64,
    modified: u64,
}

fn fingerprint(path: &Path) -> Option<Fingerprint> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(Fingerprint { size: meta.len(), modified })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Outcome {
    Done,
    Failed,
    /// Was already there when a "new files only" watch began.
    Skipped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Seen {
    print: Fingerprint,
    outcome: Outcome,
}

/// What the watch remembers between runs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Memory {
    folder: Option<PathBuf>,
    /// The "what was already here" snapshot has been taken for `folder`.
    baselined: bool,
    files: HashMap<String, Seen>,
}

/// Shown in Settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub running: bool,
    pub folder: Option<PathBuf>,
    /// Unix time of the last scan.
    pub last_scan: u64,
    pub in_progress: u32,
    pub done: u32,
    pub failed: u32,
    /// Why the watch cannot run, when it cannot.
    pub problem: Option<String>,
}

struct Shared {
    engine: Engine,
    memory_path: PathBuf,
    memory: Mutex<Memory>,
    status: Mutex<WatchStatus>,
    in_flight: Mutex<HashSet<String>>,
}

/// Owns the background scan. Call [`Watcher::reload`] whenever the watch
/// settings change.
pub struct Watcher {
    shared: Arc<Shared>,
    stop: Mutex<Option<watch::Sender<bool>>>,
}

impl Watcher {
    pub fn new(engine: Engine, data_dir: &Path) -> Watcher {
        let memory_path = data_dir.join("watch.json");
        let memory = std::fs::read_to_string(&memory_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Watcher {
            shared: Arc::new(Shared {
                engine,
                memory_path,
                memory: Mutex::new(memory),
                status: Mutex::new(WatchStatus::default()),
                in_flight: Mutex::new(HashSet::new()),
            }),
            stop: Mutex::new(None),
        }
    }

    pub fn status(&self) -> WatchStatus {
        self.shared.status.lock().expect("status lock").clone()
    }

    /// Stops the current scan loop and starts a new one if the settings ask
    /// for it. Must be called from within a Tokio runtime.
    pub fn reload(&self) {
        if let Some(stop) = self.stop.lock().expect("stop lock").take() {
            let _ = stop.send(true);
        }
        let config = self.shared.engine.settings().get().watch;
        let mut status = self.shared.status.lock().expect("status lock");
        *status = WatchStatus { done: status.done, failed: status.failed, ..WatchStatus::default() };
        if !config.enabled {
            return;
        }
        let Some(folder) = config.folder.clone().filter(|f| f.is_dir()) else {
            status.problem = Some("Choose a folder that exists.".into());
            return;
        };
        if config.output_mode == OutputMode::Folder && config.output_folder.is_none() {
            status.problem = Some("Choose where the results go.".into());
            return;
        }
        status.running = true;
        status.folder = Some(folder.clone());
        drop(status);

        // A different folder starts from a clean slate.
        {
            let mut memory = self.shared.memory.lock().expect("memory lock");
            if memory.folder.as_ref() != Some(&folder) {
                *memory = Memory { folder: Some(folder.clone()), ..Memory::default() };
            }
        }

        let (stop_tx, stop_rx) = watch::channel(false);
        *self.stop.lock().expect("stop lock") = Some(stop_tx);
        tokio::spawn(scan_loop(self.shared.clone(), config, folder, stop_rx));
    }
}

fn candidates(root: &Path, recursive: bool, skip: Option<&Path>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                // Never descend into our own output or "processed" folders.
                if recursive && name != "processed" && Some(path.as_path()) != skip {
                    pending.push(path);
                }
                continue;
            }
            let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if is_video(&path) && !looks_generated(&stem) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

async fn scan_loop(shared: Arc<Shared>, config: WatchSettings, folder: PathBuf, mut stop: watch::Receiver<bool>) {
    let mut settling: HashMap<String, (Fingerprint, Instant)> = HashMap::new();
    loop {
        scan(&shared, &config, &folder, &mut settling);
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(u64::from(config.poll_seconds))) => {}
            _ = stop.changed() => return,
        }
    }
}

fn scan(
    shared: &Arc<Shared>,
    config: &WatchSettings,
    folder: &Path,
    settling: &mut HashMap<String, (Fingerprint, Instant)>,
) {
    let skip = match config.output_mode {
        OutputMode::Folder => config.output_folder.as_deref(),
        OutputMode::SameFolder => None,
    };
    let files = candidates(folder, config.recursive, skip);
    let now = Instant::now();

    let mut memory = shared.memory.lock().expect("memory lock");
    if !memory.baselined {
        if config.existing == Existing::NewOnly {
            for path in &files {
                if let Some(print) = fingerprint(path) {
                    memory.files.insert(key(path), Seen { print, outcome: Outcome::Skipped });
                }
            }
        }
        memory.baselined = true;
        save(shared, &memory);
    }

    let mut ready = Vec::new();
    for path in files {
        let k = key(&path);
        let Some(print) = fingerprint(&path) else { continue };
        if memory.files.get(&k).is_some_and(|seen| seen.print == print) {
            continue; // Already handled in this exact state.
        }
        if shared.in_flight.lock().expect("flight lock").contains(&k) {
            continue;
        }
        match settling.get(&k) {
            Some((previous, since)) if *previous == print => {
                if now.duration_since(*since) >= Duration::from_secs(u64::from(config.stable_seconds)) {
                    settling.remove(&k);
                    ready.push((path, k, print));
                }
            }
            _ => {
                settling.insert(k, (print, now));
            }
        }
    }
    drop(memory);

    for (path, k, print) in ready {
        dispatch(shared, config, path, k, print);
    }
    let mut status = shared.status.lock().expect("status lock");
    status.last_scan = history::now();
    status.in_progress = shared.in_flight.lock().expect("flight lock").len() as u32;
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn save(shared: &Shared, memory: &Memory) {
    if let Ok(text) = serde_json::to_string(memory) {
        if let Err(e) = write_atomic(&shared.memory_path, &text) {
            log::warn!("could not save folder-watch state: {e}");
        }
    }
}

fn dispatch(shared: &Arc<Shared>, config: &WatchSettings, path: PathBuf, k: String, print: Fingerprint) {
    let settings = shared.engine.settings().get();
    let options = config
        .profile
        .as_deref()
        .and_then(|name| settings.profile(name))
        .map(|p| p.options.clone())
        .unwrap_or_else(|| settings.default_options());
    let output_dir = match config.output_mode {
        OutputMode::Folder => config.output_folder.clone(),
        OutputMode::SameFolder => None,
    };
    shared.in_flight.lock().expect("flight lock").insert(k.clone());
    let (_, outcome) = shared.engine.submit_and_wait(JobSpec {
        input: path.clone(),
        trim: None,
        options,
        output_dir,
        origin: Origin::Watch,
    });

    let shared = shared.clone();
    let originals = config.originals;
    tokio::spawn(async move {
        let result = outcome.await.unwrap_or_else(|_| Err("the job was dropped".into()));
        shared.in_flight.lock().expect("flight lock").remove(&k);
        let outcome = match &result {
            Ok(result) => {
                if let Err(e) = put_away(&path, &result.output, originals) {
                    log::warn!("folder watch could not tidy {}: {e}", path.display());
                }
                Outcome::Done
            }
            Err(reason) => {
                log::warn!("folder watch could not compress {}: {reason}", path.display());
                Outcome::Failed
            }
        };
        {
            let mut memory = shared.memory.lock().expect("memory lock");
            memory.files.insert(k, Seen { print, outcome });
            save(&shared, &memory);
        }
        let mut status = shared.status.lock().expect("status lock");
        match outcome {
            Outcome::Done => status.done += 1,
            _ => status.failed += 1,
        }
        status.in_progress = shared.in_flight.lock().expect("flight lock").len() as u32;
    });
}

/// Applies the "what happens to the original" choice after a good result.
fn put_away(source: &Path, output: &Path, originals: Originals) -> std::io::Result<()> {
    match originals {
        Originals::Keep => Ok(()),
        Originals::Delete => std::fs::remove_file(source),
        Originals::Move => {
            let dir = output.parent().unwrap_or(Path::new(".")).join("processed");
            std::fs::create_dir_all(&dir)?;
            let name = source.file_name().unwrap_or_default();
            let mut destination = dir.join(name);
            let mut n = 2;
            while destination.exists() {
                let stem = source.file_stem().unwrap_or_default().to_string_lossy();
                let ext = source.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                destination = dir.join(format!("{stem}-{n}{ext}"));
                n += 1;
            }
            // Rename fails across drives; copy and delete then.
            std::fs::rename(source, &destination).or_else(|_| {
                std::fs::copy(source, &destination)?;
                std::fs::remove_file(source)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_videos_and_skips_our_own_output() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("processed")).unwrap();
        for name in ["a.mp4", "b.MOV", "notes.txt", "a-10mb.mp4", ".hidden.mp4", "sub/c.mkv", "processed/d.mp4"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        let names = |recursive| -> Vec<String> {
            candidates(root, recursive, None)
                .iter()
                .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"))
                .collect()
        };
        assert_eq!(names(false), ["a.mp4", "b.MOV"]);
        assert_eq!(names(true), ["a.mp4", "b.MOV", "sub/c.mkv"]);
    }

    #[test]
    fn moving_originals_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("clip.mp4");
        let output = dir.path().join("clip-10mb.mp4");
        std::fs::create_dir_all(dir.path().join("processed")).unwrap();
        std::fs::write(dir.path().join("processed/clip.mp4"), b"earlier").unwrap();
        std::fs::write(&source, b"original").unwrap();
        put_away(&source, &output, Originals::Move).unwrap();
        assert!(!source.exists());
        assert_eq!(std::fs::read(dir.path().join("processed/clip.mp4")).unwrap(), b"earlier");
        assert_eq!(std::fs::read(dir.path().join("processed/clip-2.mp4")).unwrap(), b"original");
    }
}
