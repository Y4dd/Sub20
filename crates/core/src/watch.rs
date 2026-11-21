//! Folder watch: put whatever appears in a folder on the bench, or compress
//! it straight away (`WatchSettings::compress`).
//!
//! The folder is scanned on a timer rather than through OS notifications.
//! That is slower to react but behaves the same on network shares, synced
//! folders and every platform, and it makes "has this file finished
//! copying?" easy to answer: wait until it stops changing.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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

/// A recording app's clip folder, offered so the watch can be pointed at it
/// in one click.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipFolder {
    /// The app that saves there, as people know it.
    pub app: String,
    pub path: PathBuf,
    /// Clips land in a subfolder per game.
    pub recursive: bool,
}

/// The clip folders of recording apps found at their default places in the
/// Videos folder. Windows only: that's where these apps run.
///
/// Steam isn't here: it keeps recordings as stream segments rather than video
/// files, and saves an exported clip wherever the player chooses.
pub fn clip_folders(videos: &Path) -> Vec<ClipFolder> {
    if cfg!(windows) {
        clip_folders_in(videos)
    } else {
        Vec::new()
    }
}

fn clip_folders_in(videos: &Path) -> Vec<ClipFolder> {
    [
        // The NVIDIA app: Instant Replay and recordings, one folder per game.
        ("NVIDIA app", "NVIDIA", true),
        // Xbox Game Bar (Win+Alt+R, Win+Alt+G).
        ("Xbox Game Bar", "Captures", false),
    ]
    .into_iter()
    .map(|(app, folder, recursive)| ClipFolder { app: app.into(), path: videos.join(folder), recursive })
    .filter(|found| found.path.is_dir())
    .collect()
}

/// Size and modification time: enough to tell whether a file changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Fingerprint {
    size: u64,
    modified: u64,
}

/// The file's own size and date, asked of the file itself: exact even while
/// another program is still writing it.
fn fingerprint(path: &Path) -> Option<Fingerprint> {
    print_of(&std::fs::metadata(path).ok()?)
}

fn print_of(meta: &std::fs::Metadata) -> Option<Fingerprint> {
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(Fingerprint { size: meta.len(), modified })
}

/// A video found in the folder, with the size and date its directory listing
/// gave. Those come free with the listing on Windows, where asking the file
/// itself means opening it; but NTFS may not update them while a file is
/// still open for writing, so they only tell "unchanged since handled" and
/// anything else is asked of the file (`fingerprint`).
struct Found {
    path: PathBuf,
    listed: Option<Fingerprint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Outcome {
    Done,
    Failed,
    /// Was already there when a "new files only" watch began.
    Skipped,
    /// Handed over to go on the bench.
    Benched,
}

/// Takes the clips the watch puts on the bench (the shell: the window's
/// bench, or the saved one while there is no window).
pub type BenchHand = Arc<dyn Fn(Vec<PathBuf>) + Send + Sync>;

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
    /// Clips put on the bench since the app opened.
    pub benched: u32,
    /// Why the watch cannot run, when it cannot.
    pub problem: Option<String>,
    /// Clips that arrived while Sub20 was closed, held until the user says
    /// whether to compress them ([`Watcher::decide`]). Zero when none are held.
    pub waiting: u32,
}

/// More clips than this arriving while Sub20 was closed are held for a
/// question at launch rather than all compressed at once.
pub const PILE: usize = 5;

struct Shared {
    engine: Engine,
    memory_path: PathBuf,
    memory: Mutex<Memory>,
    status: Mutex<WatchStatus>,
    in_flight: Mutex<HashSet<String>>,
    /// The pile held at launch (keys), while the question is open.
    held: Mutex<Option<HashSet<String>>>,
    bench: BenchHand,
}

/// Owns the background scan. Call [`Watcher::reload`] whenever the watch
/// settings change.
pub struct Watcher {
    shared: Arc<Shared>,
    stop: Mutex<Option<watch::Sender<bool>>>,
    /// No scan has run since Sub20 started, so what is new arrived while it
    /// was closed.
    launched: AtomicBool,
}

impl Watcher {
    pub fn new(engine: Engine, data_dir: &Path, bench: BenchHand) -> Watcher {
        let memory_path = data_dir.join("watch.json");
        let memory = crate::settings::read_json(&memory_path)
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
                held: Mutex::new(None),
                bench,
            }),
            stop: Mutex::new(None),
            launched: AtomicBool::new(true),
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
        let ask = self.launched.swap(false, Ordering::SeqCst);
        let mut status = self.shared.status.lock().expect("status lock");
        // A question about a pile stays open while the folder stays the same.
        let mut held = self.shared.held.lock().expect("held lock");
        if !config.enabled || config.folder != status.folder {
            *held = None;
        }
        let waiting = held.as_ref().map_or(0, |keys| keys.len() as u32);
        drop(held);
        *status = WatchStatus {
            done: status.done,
            failed: status.failed,
            benched: status.benched,
            waiting,
            ..WatchStatus::default()
        };
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
        tokio::spawn(scan_loop(self.shared.clone(), config, folder, stop_rx, ask));
    }

    /// Answers the question about clips that arrived while Sub20 was closed:
    /// compress them, or skip them for good.
    pub fn decide(&self, compress: bool) {
        let Some(held) = self.shared.held.lock().expect("held lock").take() else { return };
        if !compress {
            let mut memory = self.shared.memory.lock().expect("memory lock");
            for k in held {
                if let Some(print) = fingerprint(Path::new(&k)) {
                    memory.files.insert(k, Seen { print, outcome: Outcome::Skipped });
                }
            }
            save(&self.shared, &memory);
        }
        self.shared.status.lock().expect("status lock").waiting = 0;
    }
}

/// Files that are new or changed since the watch last handled them.
fn arrivals(files: &[Found], memory: &Memory, in_flight: &HashSet<String>) -> Vec<String> {
    files
        .iter()
        .filter_map(|found| {
            let k = key(&found.path);
            let print = fingerprint(&found.path)?;
            let handled = memory.files.get(&k).is_some_and(|seen| seen.print == print);
            (!handled && !in_flight.contains(&k)).then_some(k)
        })
        .collect()
}

fn candidates(root: &Path, recursive: bool, skip: Option<&Path>) -> Vec<Found> {
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
            let Ok(kind) = entry.file_type() else { continue };
            // A link is followed, as before: to a folder, or to a video.
            let linked = kind.is_symlink();
            if kind.is_dir() || (linked && path.is_dir()) {
                // Never descend into our own output or "processed" folders.
                if recursive && name != "processed" && Some(path.as_path()) != skip {
                    pending.push(path);
                }
                continue;
            }
            let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if is_video(&path) && !looks_generated(&stem) {
                let listed =
                    if linked { fingerprint(&path) } else { entry.metadata().ok().as_ref().and_then(print_of) };
                found.push(Found { path, listed });
            }
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

async fn scan_loop(
    shared: Arc<Shared>,
    config: WatchSettings,
    folder: PathBuf,
    mut stop: watch::Receiver<bool>,
    mut ask: bool,
) {
    let mut settling: HashMap<String, (Fingerprint, Instant)> = HashMap::new();
    loop {
        scan(&shared, &config, &folder, &mut settling, ask);
        ask = false;
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
    ask: bool,
) {
    let skip = match config.output_mode {
        OutputMode::Folder => config.output_folder.as_deref(),
        OutputMode::SameFolder | OutputMode::Replace => None,
    };
    let files = candidates(folder, config.recursive, skip);
    let now = Instant::now();

    let mut memory = shared.memory.lock().expect("memory lock");
    if !memory.baselined {
        if config.existing == Existing::NewOnly {
            for found in &files {
                if let Some(print) = fingerprint(&found.path) {
                    memory.files.insert(key(&found.path), Seen { print, outcome: Outcome::Skipped });
                }
            }
        }
        memory.baselined = true;
        save(shared, &memory);
    }

    // Many clips that arrived while Sub20 was closed wait for a question,
    // rather than all starting at once on a computer that may be busy. On
    // the bench they cost nothing, so there is nothing to ask.
    if ask && config.compress {
        let new = arrivals(&files, &memory, &shared.in_flight.lock().expect("flight lock"));
        if new.len() > PILE {
            shared.status.lock().expect("status lock").waiting = new.len() as u32;
            *shared.held.lock().expect("held lock") = Some(new.into_iter().collect());
        }
    }
    let held = shared.held.lock().expect("held lock").clone().unwrap_or_default();

    let mut ready = Vec::new();
    for Found { path, listed } in files {
        let k = key(&path);
        if held.contains(&k) {
            continue;
        }
        // Most files are ones handled long ago, and the listing says so
        // without opening them (thousands, in a busy clip folder).
        let seen = memory.files.get(&k).map(|seen| seen.print);
        if listed.is_some() && listed == seen {
            continue;
        }
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
    if !config.compress && !ready.is_empty() {
        // Marked first, so a clip is handed over once, whatever the bench
        // then does with it.
        for (_, k, print) in &ready {
            memory.files.insert(k.clone(), Seen { print: *print, outcome: Outcome::Benched });
        }
        save(shared, &memory);
        drop(memory);
        let count = ready.len() as u32;
        (shared.bench)(ready.into_iter().map(|(path, _, _)| path).collect());
        shared.status.lock().expect("status lock").benched += count;
        ready = Vec::new();
    } else {
        drop(memory);
    }

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
        OutputMode::SameFolder | OutputMode::Replace => None,
    };
    shared.in_flight.lock().expect("flight lock").insert(k.clone());
    let (_, outcome) = shared.engine.submit_and_wait(JobSpec {
        input: path.clone(),
        trim: None,
        options,
        output_dir,
        origin: Origin::Watch,
        output_name: None,
        replace_original: false,
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
    fn clip_folders_are_offered_only_where_they_exist() {
        let videos = tempfile::tempdir().unwrap();
        assert!(clip_folders_in(videos.path()).is_empty());

        std::fs::create_dir_all(videos.path().join("NVIDIA").join("The Finals")).unwrap();
        std::fs::write(videos.path().join("Captures"), b"a file, not a folder").unwrap();
        let found = clip_folders_in(videos.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].app, "NVIDIA app");
        assert_eq!(found[0].path, videos.path().join("NVIDIA"));
        assert!(found[0].recursive, "the NVIDIA app saves a folder per game");

        std::fs::remove_file(videos.path().join("Captures")).unwrap();
        std::fs::create_dir(videos.path().join("Captures")).unwrap();
        let apps: Vec<_> = clip_folders_in(videos.path()).into_iter().map(|f| f.app).collect();
        assert_eq!(apps, ["NVIDIA app", "Xbox Game Bar"]);
    }

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
                .map(|found| found.path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"))
                .collect()
        };
        assert_eq!(names(false), ["a.mp4", "b.MOV"]);
        // A closed file's listing agrees with the file itself, so a handled
        // file is skipped without being opened.
        for found in candidates(root, true, None) {
            assert!(found.listed.is_some());
            assert_eq!(found.listed, fingerprint(&found.path), "{}", found.path.display());
        }
        assert_eq!(names(true), ["a.mp4", "b.MOV", "sub/c.mkv"]);
    }

    /// A watcher as Sub20 makes one at launch, over `inbox`, new files only,
    /// compressing straight away.
    fn launch(root: &Path, inbox: &Path) -> Watcher {
        launch_with(root, inbox, true, Arc::new(|_| panic!("a compressing watch benched a clip")))
    }

    fn launch_with(root: &Path, inbox: &Path, compress: bool, bench: BenchHand) -> Watcher {
        let engine = Engine::new(&root.join("data"), &root.join("work"), Vec::new(), Arc::new(|_| {}));
        engine
            .settings()
            .update(|s| {
                s.watch.enabled = true;
                s.watch.folder = Some(inbox.to_path_buf());
                s.watch.compress = compress;
                s.watch.stable_seconds = 2;
                s.watch.poll_seconds = 2;
            })
            .unwrap();
        let watcher = Watcher::new(engine, &root.join("data"), bench);
        watcher.reload();
        watcher
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn new_clips_go_on_the_bench_once_and_are_not_compressed() {
        let dir = tempfile::tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        std::fs::write(inbox.join("old.mp4"), b"already there").unwrap();
        let handed: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let bench = |handed: &Arc<Mutex<Vec<PathBuf>>>| -> BenchHand {
            let handed = handed.clone();
            Arc::new(move |paths| handed.lock().unwrap().extend(paths))
        };

        let first = launch_with(dir.path(), &inbox, false, bench(&handed));
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Seven at once: on the bench there is no question about a pile.
        for n in 1..=7 {
            std::fs::write(inbox.join(format!("clip{n}.mp4")), b"not really a video").unwrap();
        }
        // Settled after two scans two seconds apart.
        tokio::time::sleep(Duration::from_millis(4600)).await;
        let mut names: Vec<_> =
            handed.lock().unwrap().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, (1..=7).map(|n| format!("clip{n}.mp4")).collect::<Vec<_>>(), "not the one already there");
        let status = first.status();
        assert_eq!((status.waiting, status.benched, status.in_progress, status.done), (0, 7, 0, 0), "{status:?}");
        drop(first);

        // A relaunch doesn't hand them over again.
        let again = launch_with(dir.path(), &inbox, false, bench(&handed));
        tokio::time::sleep(Duration::from_millis(4600)).await;
        assert_eq!(handed.lock().unwrap().len(), 7);
        assert_eq!(again.status().benched, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_pile_that_arrived_while_closed_waits_for_an_answer() {
        let dir = tempfile::tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();

        // The first run only notes what is already there.
        let first = launch(dir.path(), &inbox);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(first.status().waiting, 0);
        drop(first);

        // Seven clips land while Sub20 is closed.
        for n in 1..=7 {
            std::fs::write(inbox.join(format!("clip{n}.mp4")), b"not really a video").unwrap();
        }
        let again = launch(dir.path(), &inbox);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(again.status().waiting, 7, "{:?}", again.status());

        // Skipped: they are never sent, even once they have settled.
        again.decide(false);
        assert_eq!(again.status().waiting, 0);
        tokio::time::sleep(Duration::from_secs(5)).await;
        let status = again.status();
        assert_eq!((status.in_progress, status.done, status.failed), (0, 0, 0), "{status:?}");

        // A third launch doesn't ask about them again.
        let third = launch(dir.path(), &inbox);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(third.status().waiting, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_few_clips_that_arrived_while_closed_are_not_asked_about() {
        let dir = tempfile::tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        drop(launch(dir.path(), &inbox));
        tokio::time::sleep(Duration::from_millis(300)).await;
        for n in 1..=PILE {
            std::fs::write(inbox.join(format!("clip{n}.mp4")), b"x").unwrap();
        }
        let again = launch(dir.path(), &inbox);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(again.status().waiting, 0);
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
