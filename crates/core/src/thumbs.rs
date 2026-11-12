//! Still frames for the trim timeline and the preview.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::error::{Error, IoContext, Result};
use crate::ffmpeg::{self, Tools};

/// Frames extracted at once. Each is a short-lived FFmpeg process.
const PARALLEL: usize = 4;

fn cache_key(input: &Path, count: usize, height: u32) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    count.hash(&mut hasher);
    height.hash(&mut hasher);
    if let Ok(meta) = std::fs::metadata(input) {
        meta.len().hash(&mut hasher);
        if let Ok(modified) = meta.modified() {
            modified.hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

/// Cuts `count` evenly spaced thumbnails, `height` pixels tall, and returns
/// their paths in order. Results are cached under `cache_dir`, keyed by the
/// file's path, size and modification time.
///
/// `on_frame` hears about each thumbnail as soon as it exists (cached ones
/// first), with its place in the strip, so the timeline can fill in as it
/// goes. Frames are cut roughly left to right.
pub async fn filmstrip(
    tools: &Tools,
    input: &Path,
    duration: f64,
    count: usize,
    height: u32,
    cache_dir: &Path,
    mut on_frame: impl FnMut(usize, &Path),
) -> Result<Vec<PathBuf>> {
    let count = count.clamp(1, 60);
    let dir = cache_dir.join(cache_key(input, count, height));
    std::fs::create_dir_all(&dir).ctx("could not create the thumbnail cache")?;
    let paths: Vec<PathBuf> = (0..count).map(|i| dir.join(format!("{i:03}.jpg"))).collect();

    let limit = Arc::new(Semaphore::new(PARALLEL));
    let mut tasks = JoinSet::new();
    for (i, path) in paths.iter().enumerate() {
        if path.is_file() {
            on_frame(i, path);
            continue;
        }
        // Sample the middle of each slice; stay clear of the very end, where
        // seeking can land past the last frame.
        let at = ((i as f64 + 0.5) / count as f64 * duration).min((duration - 0.1).max(0.0));
        let (tools, input, path, limit) = (tools.clone(), input.to_path_buf(), path.clone(), limit.clone());
        tasks.spawn(async move {
            let _slot = limit.acquire().await;
            let mut cmd = tools.ffmpeg();
            // Thumbnails need not be frame-exact: take the keyframe at the
            // seek point instead of decoding forward to the exact time.
            cmd.args(["-y", "-loglevel", "error", "-noaccurate_seek", "-ss", &format!("{at:.3}")])
                .arg("-i")
                .arg(&input)
                .args(["-frames:v", "1", "-an", "-sn", "-dn"])
                .args(["-vf", &format!("scale=-2:{height}:flags=fast_bilinear")])
                .args(["-q:v", "6"])
                .arg(&path);
            let _ = ffmpeg::run(&mut cmd, Duration::from_secs(30)).await;
            i
        });
    }
    while let Some(done) = tasks.join_next().await {
        if let Ok(i) = done {
            if paths[i].is_file() {
                on_frame(i, &paths[i]);
            }
        }
    }

    // A frame can be missing (broken spot in the file). Borrow a neighbor so
    // the strip has no holes.
    let made: Vec<bool> = paths.iter().map(|p| p.is_file()).collect();
    if !made.iter().any(|m| *m) {
        return Err(Error::Encode("no preview frames could be read from this video".into()));
    }
    let filled = (0..count)
        .map(|i| {
            let nearest = (0..count).filter(|j| made[*j]).min_by_key(|j| j.abs_diff(i)).expect("at least one frame");
            paths[nearest].clone()
        })
        .collect();
    Ok(filled)
}

/// One JPEG frame at `at` seconds, at most `max_height` pixels tall. Used
/// when the window cannot play the source format itself.
pub async fn frame(tools: &Tools, input: &Path, at: f64, max_height: u32) -> Result<Vec<u8>> {
    let mut cmd = tools.ffmpeg();
    cmd.args(["-loglevel", "error", "-ss", &format!("{:.3}", at.max(0.0))])
        .arg("-i")
        .arg(input)
        .args(["-frames:v", "1", "-an", "-sn", "-dn"])
        .args(["-vf", &format!("scale=-2:'min(ih,{max_height})':flags=bilinear")])
        .args(["-c:v", "mjpeg", "-q:v", "4", "-f", "image2pipe", "pipe:1"]);
    let (ok, bytes, stderr) = ffmpeg::run_bytes(&mut cmd, Duration::from_secs(30)).await?;
    if ok && !bytes.is_empty() {
        Ok(bytes)
    } else {
        Err(Error::Encode(ffmpeg::tail(&stderr, 1)))
    }
}

/// Deletes cached thumbnail sets not touched for `max_age`.
pub fn prune_cache(cache_dir: &Path, max_age: Duration) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else { return };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > max_age);
        if old && entry.path().is_dir() {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}
