//! End-to-end tests that run real FFmpeg encodes.
//!
//! They need `ffmpeg` and `ffprobe` on PATH and are skipped (with a note)
//! when those are missing.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sub20_core::encoders::Codec;
use sub20_core::ffmpeg::{self, Tools};
use sub20_core::job::{AudioChoice, Container, Effort, EncodeOptions, JobEvent, JobSpec, Origin, Trim};
use sub20_core::settings::{Existing, Originals};
use sub20_core::watch::Watcher;
use sub20_core::{probe, thumbs, Engine};

const MB: u64 = 1024 * 1024;

#[derive(Default)]
struct Events(Mutex<Vec<JobEvent>>);

impl Events {
    fn all(&self) -> Vec<JobEvent> {
        self.0.lock().unwrap().clone()
    }
}

struct Bench {
    _dir: tempfile::TempDir,
    root: PathBuf,
    engine: Engine,
    events: Arc<Events>,
    tools: Tools,
}

fn tools() -> Option<Tools> {
    match Tools::locate(None, &[]) {
        Ok(tools) => Some(tools),
        Err(_) => {
            eprintln!("skipped: FFmpeg is not installed");
            None
        }
    }
}

/// A busy 720p test pattern with a tone, `seconds` long.
async fn make_source(tools: &Tools, path: &Path, seconds: u32, size: &str) {
    let mut cmd = tools.ffmpeg();
    cmd.args(["-y", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg(format!("testsrc2=s={size}:r=30:d={seconds}"))
        .args(["-f", "lavfi", "-i"])
        .arg(format!("sine=frequency=440:duration={seconds}"))
        .args(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "18", "-pix_fmt", "yuv420p"])
        .args(["-c:a", "aac", "-b:a", "128k", "-shortest"])
        .arg(path);
    let out = ffmpeg::run(&mut cmd, Duration::from_secs(120)).await.unwrap();
    assert!(out.ok, "could not make the test video: {}", out.stderr);
}

async fn bench() -> Option<Bench> {
    let tools = tools()?;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let events = Arc::new(Events::default());
    let sink = events.clone();
    let engine = Engine::new(
        &root.join("data"),
        &root.join("work"),
        Vec::new(),
        Arc::new(move |event: JobEvent| sink.0.lock().unwrap().push(event)),
    );
    Some(Bench { _dir: dir, root, engine, events, tools })
}

fn spec(input: &Path, options: EncodeOptions) -> JobSpec {
    JobSpec {
        input: input.to_path_buf(),
        trim: None,
        options,
        output_dir: None,
        origin: Origin::Manual,
        output_name: None,
        replace_original: false,
    }
}

fn no_leftovers(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(!name.contains(".part."), "partial file left behind: {name}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn fits_the_target_and_reports_progress() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("pattern.mp4");
    make_source(&b.tools, &input, 20, "1280x720").await;

    // libx264 by name: on a machine with a GPU the automatic choice would be
    // a hardware encoder (see `a_hardware_encoder_that_misses_hands_over_to_the_cpu`).
    let options = EncodeOptions { target_mb: 1.0, encoder: Some("libx264".into()), ..EncodeOptions::default() };
    let (_, done) = b.engine.submit_and_wait(spec(&input, options));
    let result = done.await.unwrap().expect("the job should succeed");

    assert!(result.size_bytes <= MB, "{} bytes is over 1 MB", result.size_bytes);
    assert!(result.size_bytes > MB * 6 / 10, "{} bytes wastes the budget", result.size_bytes);
    assert_eq!(result.output.file_name().unwrap(), "pattern-1mb.mp4");
    assert_eq!(result.encoder, "libx264");

    let info = probe::probe(&b.tools, &result.output).await.unwrap();
    assert!((info.duration - 20.0).abs() < 0.3, "duration {}", info.duration);
    assert_eq!(info.video_codec.as_deref(), Some("h264"));
    assert_eq!(info.audio_codec.as_deref(), Some("aac"));
    assert_eq!((info.width, info.height), (result.width, result.height));

    let events = b.events.all();
    assert!(matches!(events.first(), Some(JobEvent::Queued { .. })));
    assert!(matches!(events.last(), Some(JobEvent::Done { .. })));
    let percents: Vec<f64> = events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Progress { percent, .. } => Some(*percent),
            _ => None,
        })
        .collect();
    assert!(percents.len() >= 3, "expected several progress updates, got {percents:?}");
    assert!(events.iter().any(|e| matches!(e, JobEvent::Started { encoder, .. } if encoder == "libx264")));
    assert_eq!(b.engine.history().list().len(), 1);
    no_leftovers(&b.root);
}

#[tokio::test(flavor = "multi_thread")]
async fn trim_keeps_only_the_chosen_part() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("long.mp4");
    make_source(&b.tools, &input, 20, "1280x720").await;

    let mut job = spec(&input, EncodeOptions { target_mb: 0.5, ..EncodeOptions::default() });
    job.trim = Some(Trim { start: 5.0, end: 11.0 });
    job.output_dir = Some(b.root.join("out"));
    let (_, done) = b.engine.submit_and_wait(job);
    let result = done.await.unwrap().expect("the job should succeed");

    assert!(result.output.starts_with(b.root.join("out")));
    assert!(result.size_bytes <= MB / 2, "{} bytes is over 0.5 MB", result.size_bytes);
    let info = probe::probe(&b.tools, &result.output).await.unwrap();
    assert!((info.duration - 6.0).abs() < 0.2, "kept {} s, wanted 6", info.duration);
    assert_eq!(result.duration, 6.0);
    assert!(b.engine.history().list()[0].trimmed);
}

#[tokio::test(flavor = "multi_thread")]
async fn hevc_av1_and_two_pass_all_land_under_the_limit() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("clip.mp4");
    make_source(&b.tools, &input, 8, "854x480").await;
    let report = b.engine.hardware(false).await.unwrap();

    let cases = [
        ("libx265", EncodeOptions { codec: Codec::Hevc, target_mb: 0.5, ..Default::default() }, "hevc", "aac"),
        (
            "libsvtav1",
            EncodeOptions {
                codec: Codec::Av1,
                target_mb: 0.5,
                container: Container::Mkv,
                effort: Effort::P2,
                audio: AudioChoice::Opus,
                ..Default::default()
            },
            "av1",
            "opus",
        ),
        ("libx264", EncodeOptions { effort: Effort::Max, target_mb: 0.4, ..Default::default() }, "h264", "aac"),
        (
            "libx265",
            EncodeOptions { codec: Codec::Hevc, effort: Effort::Max, target_mb: 0.4, ..Default::default() },
            "hevc",
            "aac",
        ),
    ];
    for (encoder, options, video, audio) in cases {
        if !report.works(encoder) {
            eprintln!("skipped {encoder}: not in this FFmpeg build");
            continue;
        }
        let target = (options.target_mb * MB as f64) as u64;
        let two_pass = options.effort == Effort::Max;
        let options = EncodeOptions { encoder: Some(encoder.into()), ..options };
        let (id, done) = b.engine.submit_and_wait(spec(&input, options));
        let result = done.await.unwrap().unwrap_or_else(|e| panic!("{encoder} failed: {e}"));
        assert_eq!(result.encoder, encoder);
        assert!(result.size_bytes <= target, "{encoder}: {} bytes over {target}", result.size_bytes);
        let info = probe::probe(&b.tools, &result.output).await.unwrap();
        assert_eq!(info.video_codec.as_deref(), Some(video), "{encoder}");
        assert_eq!(info.audio_codec.as_deref(), Some(audio), "{encoder}");
        assert!((info.duration - 8.0).abs() < 0.3, "{encoder}: duration {}", info.duration);
        if two_pass {
            let phases: Vec<_> = b
                .events
                .all()
                .into_iter()
                .filter(|e| e.id() == id)
                .filter_map(|e| match e {
                    JobEvent::Progress { phase, .. } => Some(phase),
                    _ => None,
                })
                .collect();
            assert!(phases.contains(&sub20_core::job::Phase::Analysis), "{encoder}: no first pass seen");
        }
    }
    // Scratch folders for pass logs are cleaned up.
    let leftovers = std::fs::read_dir(b.root.join("work")).map(|d| d.count()).unwrap_or(0);
    assert_eq!(leftovers, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_hardware_encoder_falls_back_to_the_cpu() {
    let Some(b) = bench().await else { return };
    let report = b.engine.hardware(false).await.unwrap();
    if report.works("h264_nvenc") {
        eprintln!("skipped: this machine has a working NVENC");
        return;
    }
    let input = b.root.join("clip.mp4");
    make_source(&b.tools, &input, 4, "640x360").await;
    let options = EncodeOptions { encoder: Some("h264_nvenc".into()), target_mb: 0.3, ..Default::default() };
    let (_, done) = b.engine.submit_and_wait(spec(&input, options));
    let result = done.await.unwrap().expect("the job should finish on the CPU");
    assert_eq!(result.encoder, "libx264");
    assert!(!result.hardware);
    assert_eq!(result.fell_back_from.as_deref(), Some("h264_nvenc"));
    assert!(b.events.all().iter().any(|e| matches!(e, JobEvent::Fallback { to, .. } if to == "libx264")));
}

/// A tight limit on a busy clip: h264_nvenc cannot go below about 600 kbps at
/// 720p30 and lands ~80% over. The job must hand over to the CPU and still
/// fit, never save an over-limit file. Other GPUs may hit the target
/// themselves; either way the result fits.
#[tokio::test(flavor = "multi_thread")]
async fn a_hardware_encoder_that_misses_hands_over_to_the_cpu() {
    let Some(b) = bench().await else { return };
    let report = b.engine.hardware(false).await.unwrap();
    let Some(hardware) = report.best(Codec::H264, &Default::default()).filter(|e| e.family.is_hardware()) else {
        eprintln!("skipped: no working H.264 hardware encoder");
        return;
    };
    let input = b.root.join("pattern.mp4");
    make_source(&b.tools, &input, 20, "1280x720").await;

    let options = EncodeOptions { target_mb: 1.0, ..EncodeOptions::default() };
    let (_, done) = b.engine.submit_and_wait(spec(&input, options));
    let result = done.await.unwrap().expect("the job should succeed");

    assert!(result.size_bytes <= MB, "{}: {} bytes is over 1 MB", result.encoder, result.size_bytes);
    let events = b.events.all();
    assert!(events.iter().any(|e| matches!(e, JobEvent::Started { encoder, .. } if encoder == hardware.name)));
    if result.encoder != hardware.name {
        assert_eq!(result.encoder, "libx264");
        assert_eq!(result.fell_back_from.as_deref(), Some(hardware.name));
        assert!(events.iter().any(|e| matches!(e, JobEvent::Fallback { from, to, .. }
            if from == hardware.name && to == "libx264")));
    }
    no_leftovers(&b.root);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_stops_the_encode_and_cleans_up() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("slow.mp4");
    make_source(&b.tools, &input, 30, "1920x1080").await;
    // One job at a time, so the second waits in the queue whatever the core count.
    b.engine.settings().update(|s| s.parallel_jobs = 1).unwrap();
    // Slow on purpose, so there is time to cancel.
    let options = EncodeOptions { target_mb: 20.0, effort: Effort::P7, auto_resolution: false, ..Default::default() };
    let (first, first_done) = b.engine.submit_and_wait(spec(&input, options.clone()));
    // The second job waits in the queue behind the first.
    let (second, second_done) = b.engine.submit_and_wait(spec(&input, options));

    // Wait until the first is really encoding.
    for _ in 0..200 {
        if b.events.all().iter().any(|e| matches!(e, JobEvent::Progress { percent, .. } if *percent > 0.0)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(b.engine.cancel(&second), "a queued job can be canceled");
    assert!(b.engine.cancel(&first), "a running job can be canceled");
    assert_eq!(first_done.await.unwrap().unwrap_err(), "canceled");
    assert_eq!(second_done.await.unwrap().unwrap_err(), "canceled");

    let events = b.events.all();
    let canceled = events.iter().filter(|e| matches!(e, JobEvent::Canceled { .. })).count();
    assert_eq!(canceled, 2);
    assert!(!events.iter().any(|e| matches!(e, JobEvent::Started { id, .. } if *id == second)));
    assert!(!b.engine.cancel(&first), "a finished job cannot be canceled again");
    assert_eq!(b.engine.active(), 0);
    no_leftovers(&b.root);
    assert!(!b.root.join("slow-20mb.mp4").exists());
    assert!(b.engine.history().list().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn jobs_run_in_the_order_they_were_submitted() {
    let Some(b) = bench().await else { return };
    // One at a time: jobs pass the queue in order, but jobs running side by
    // side race to their `Started` event, so only a single slot makes that
    // event's order the queue's order.
    b.engine.settings().update(|s| s.parallel_jobs = 1).unwrap();
    let mut waiting = Vec::new();
    let mut ids = Vec::new();
    for n in 0..6 {
        let input = b.root.join(format!("clip{n}.mp4"));
        make_source(&b.tools, &input, 1, "320x240").await;
        let options = EncodeOptions { target_mb: 0.1, effort: Effort::P1, ..Default::default() };
        let (id, done) = b.engine.submit_and_wait(spec(&input, options));
        ids.push(id);
        waiting.push(done);
    }
    for done in waiting {
        done.await.unwrap().expect("each job should succeed");
    }
    let started: Vec<String> = b
        .events
        .all()
        .into_iter()
        .filter_map(|e| match e {
            JobEvent::Started { id, .. } => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(started, ids, "jobs started out of order");
}

#[tokio::test(flavor = "multi_thread")]
async fn tiny_targets_are_retried_until_they_fit() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("short.mp4");
    make_source(&b.tools, &input, 2, "1280x720").await;
    // 60 KB for two busy seconds: the first try usually overshoots.
    // libx264 by name: a GPU encoder would add a hand-over to the CPU.
    let options =
        EncodeOptions { target_mb: 0.06, effort: Effort::P1, encoder: Some("libx264".into()), ..Default::default() };
    let (id, done) = b.engine.submit_and_wait(spec(&input, options));
    let outcome = done.await.unwrap();
    let retries = b.events.all().iter().filter(|e| e.id() == id && matches!(e, JobEvent::Retry { .. })).count();
    // A limit this tight is at the edge of what x264 can do, and its output
    // depends on the thread count: on a 4-core CI runner three tries can stay
    // just over. Either way the promise holds: a result that fits, or no file.
    match outcome {
        Ok(result) => {
            eprintln!(
                "size {} bytes after {} attempt(s), {retries} retry event(s)",
                result.size_bytes, result.attempts
            );
            assert_eq!(result.attempts as usize, retries + 1);
            assert!(result.size_bytes <= (0.06 * MB as f64) as u64, "{} bytes is over the limit", result.size_bytes);
        }
        Err(message) => {
            eprintln!("over the limit after {retries} retry event(s): {message}");
            assert!(message.starts_with("Could not get under the size limit"), "{message}");
            assert!(retries >= 1, "gave up without retrying");
            let saved: Vec<_> = std::fs::read_dir(&b.root)
                .unwrap()
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with("short-"))
                .collect();
            assert!(saved.is_empty(), "a result over the limit was saved: {saved:?}");
            no_leftovers(&b.root);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn audio_only_and_muted() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("talk.mp4");
    make_source(&b.tools, &input, 5, "640x360").await;

    let options = EncodeOptions { audio_only: true, audio: AudioChoice::Aac, audio_kbps: 96, ..Default::default() };
    let (_, done) = b.engine.submit_and_wait(spec(&input, options));
    let result = done.await.unwrap().expect("audio extraction should succeed");
    assert_eq!(result.output.file_name().unwrap(), "talk-audio.m4a");
    let info = probe::probe(&b.tools, &result.output).await.unwrap();
    assert!(!info.has_video && info.has_audio);

    let options = EncodeOptions { audio: AudioChoice::None, target_mb: 0.3, ..Default::default() };
    let (_, done) = b.engine.submit_and_wait(spec(&input, options));
    let result = done.await.unwrap().expect("muted encode should succeed");
    let info = probe::probe(&b.tools, &result.output).await.unwrap();
    assert!(info.has_video && !info.has_audio);
    assert_eq!(result.audio_kbps, 0);
}

/// The loudest sample in a file's sound, in dB (0 is full scale).
async fn max_volume(tools: &Tools, path: &Path) -> f64 {
    let mut cmd = tools.ffmpeg();
    cmd.args(["-hide_banner", "-i"]).arg(path).args(["-map", "0:a:0", "-af", "volumedetect", "-f", "null", "-"]);
    let out = ffmpeg::run(&mut cmd, Duration::from_secs(60)).await.unwrap();
    let line = out.stderr.lines().find(|l| l.contains("max_volume:")).expect("volumedetect printed nothing");
    line.split("max_volume:").nth(1).unwrap().trim().trim_end_matches(" dB").parse().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_microphone_on_its_own_track_is_mixed_in() {
    let Some(b) = bench().await else { return };
    // How OBS records with the mic kept apart: track 1 is the game (silent
    // here, stereo), track 2 the microphone (a tone, mono).
    let input = b.root.join("recording.mkv");
    let mut cmd = b.tools.ffmpeg();
    cmd.args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=s=640x360:r=30:d=4"])
        .args(["-f", "lavfi", "-i", "anullsrc=channel_layout=stereo:sample_rate=48000"])
        .args(["-f", "lavfi", "-i", "sine=frequency=660:sample_rate=48000:duration=4"])
        .args(["-map", "0:v", "-map", "1:a", "-map", "2:a", "-t", "4"])
        .args(["-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac", "-b:a", "96k"])
        .arg(&input);
    let out = ffmpeg::run(&mut cmd, Duration::from_secs(60)).await.unwrap();
    assert!(out.ok, "could not make the test recording: {}", out.stderr);
    let info = probe::probe(&b.tools, &input).await.unwrap();
    assert_eq!(info.audio_tracks, 2);

    let options = EncodeOptions { target_mb: 2.0, encoder: Some("libx264".into()), ..Default::default() };
    let (_, done) = b.engine.submit_and_wait(spec(&input, options.clone()));
    let mixed = done.await.unwrap().expect("the mixed encode should succeed");
    let out = probe::probe(&b.tools, &mixed.output).await.unwrap();
    assert_eq!((out.audio_tracks, out.audio_channels), (1, Some(2)), "one stereo track comes out");
    let loudest = max_volume(&b.tools, &mixed.output).await;
    // The tone is made at about -18 dB. amix's default would halve it to about
    // -24 dB; summed at full volume it stays near -18.
    assert!(loudest > -21.0, "the microphone should be heard at full volume, loudest sample {loudest} dB");
    assert!(mixed.size_bytes <= 2 * MB);

    // With mixing off only the silent game track is kept, as before.
    let first_only = EncodeOptions { mix_audio: false, ..options };
    let (_, done) = b.engine.submit_and_wait(spec(&input, first_only));
    let first = done.await.unwrap().expect("the first-track encode should succeed");
    let loudest = max_volume(&b.tools, &first.output).await;
    assert!(loudest < -60.0, "only the silent first track should be kept, loudest sample {loudest} dB");
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_input_fails_with_a_readable_message() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("notes.mp4");
    std::fs::write(&input, b"this is not a video").unwrap();
    let (_, done) = b.engine.submit_and_wait(spec(&input, EncodeOptions::default()));
    let error = done.await.unwrap().unwrap_err();
    assert!(error.starts_with("notes.mp4 couldn't be read as a video"), "{error}");
    assert!(matches!(b.events.all().last(), Some(JobEvent::Failed { .. })));
}

#[tokio::test(flavor = "multi_thread")]
async fn filmstrip_and_preview_frames() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("clip.mp4");
    make_source(&b.tools, &input, 6, "640x360").await;
    let cache = b.root.join("cache");
    let mut heard = Vec::new();
    let strip = thumbs::filmstrip(&b.tools, &input, 6.0, 12, 64, &cache, |i, path| heard.push((i, path.to_path_buf())))
        .await
        .unwrap();
    assert_eq!(strip.len(), 12);
    // Each thumbnail was announced once, as it was made, at its own place.
    heard.sort();
    assert_eq!(heard, strip.iter().cloned().enumerate().collect::<Vec<_>>());
    for path in &strip {
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(&bytes[..2], &[0xFF, 0xD8], "{} is not a JPEG", path.display());
    }
    // A second call is served from the cache.
    let started = std::time::Instant::now();
    let again = thumbs::filmstrip(&b.tools, &input, 6.0, 12, 64, &cache, |_, _| {}).await.unwrap();
    assert_eq!(strip, again);
    assert!(started.elapsed() < Duration::from_millis(200));

    let frame = thumbs::frame(&b.tools, &input, 3.0, 360).await.unwrap();
    assert_eq!(&frame[..2], &[0xFF, 0xD8]);
}

#[tokio::test(flavor = "multi_thread")]
async fn folder_watch_compresses_new_files_and_moves_originals() {
    let Some(b) = bench().await else { return };
    let inbox = b.root.join("inbox");
    let outbox = b.root.join("outbox");
    std::fs::create_dir_all(&inbox).unwrap();
    // Already there before the watch starts: must be left alone.
    make_source(&b.tools, &inbox.join("old.mp4"), 2, "320x240").await;

    b.engine
        .settings()
        .update(|s| {
            s.profiles[0].options.target_mb = 0.2;
            s.watch.enabled = true;
            s.watch.folder = Some(inbox.clone());
            s.watch.output_mode = sub20_core::settings::OutputMode::Folder;
            s.watch.output_folder = Some(outbox.clone());
            s.watch.originals = Originals::Move;
            s.watch.existing = Existing::NewOnly;
            s.watch.stable_seconds = 2;
            s.watch.poll_seconds = 2;
        })
        .unwrap();
    let watcher = Watcher::new(b.engine.clone(), &b.root.join("data"));
    watcher.reload();
    assert!(watcher.status().running, "{:?}", watcher.status().problem);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Write elsewhere, then move in, like a finished download.
    let staging = b.root.join("new.mp4");
    make_source(&b.tools, &staging, 3, "320x240").await;
    std::fs::rename(&staging, inbox.join("new.mp4")).unwrap();

    let expected = outbox.join("new-0.2mb.mp4");
    for _ in 0..120 {
        if expected.exists() && watcher.status().done == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(expected.exists(), "the watch did not produce {}", expected.display());
    assert_eq!(watcher.status().done, 1);
    assert!(outbox.join("processed").join("new.mp4").exists(), "the original was not moved");
    assert!(!inbox.join("new.mp4").exists());
    assert!(inbox.join("old.mp4").exists(), "a file that was already there was touched");
    assert!(!outbox.join("old-0.2mb.mp4").exists());
    assert_eq!(b.engine.history().list()[0].origin, Origin::Watch);

    // Switching it off stops the loop.
    b.engine.settings().update(|s| s.watch.enabled = false).unwrap();
    watcher.reload();
    assert!(!watcher.status().running);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_typed_name_is_used_and_a_result_can_be_renamed() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("clip.mp4");
    make_source(&b.tools, &input, 2, "640x360").await;
    let options = EncodeOptions { target_mb: 0.5, encoder: Some("libx264".into()), ..Default::default() };
    let job = JobSpec { output_name: Some("  for: discord.mp4 ".into()), ..spec(&input, options) };
    let (_, done) = b.engine.submit_and_wait(job);
    let result = done.await.unwrap().expect("the job should succeed");
    assert_eq!(result.output, b.root.join("for discord.mp4"));
    assert!(!result.replaced_original);
    assert!(input.exists(), "the original stays without Replace");

    let renamed = b.engine.rename_result(&result.output, "final cut").unwrap();
    assert_eq!(renamed, b.root.join("final cut.mp4"));
    assert!(renamed.exists() && !result.output.exists());
    assert_eq!(b.engine.history().list()[0].result.output, renamed, "history follows the rename");
    no_leftovers(&b.root);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_damaged_recording_says_what_to_do() {
    let Some(b) = bench().await else { return };
    let input = b.root.join("recording.mp4");
    make_source(&b.tools, &input, 4, "640x360").await;
    // A recording cut off before it was finished: an MP4's index is written last.
    let bytes = std::fs::read(&input).unwrap();
    std::fs::write(&input, &bytes[..bytes.len() * 2 / 5]).unwrap();

    let (_, done) = b.engine.submit_and_wait(spec(&input, EncodeOptions::default()));
    let error = done.await.unwrap().unwrap_err();
    assert!(error.starts_with("recording.mp4 couldn't be read as a video"), "{error}");
    assert!(error.contains("still being recorded or copied"), "{error}");
    no_leftovers(&b.root);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_source_moved_away_while_waiting_in_line_is_named() {
    let Some(b) = bench().await else { return };
    let first = b.root.join("first.mp4");
    let second = b.root.join("second.mp4");
    make_source(&b.tools, &first, 8, "1280x720").await;
    std::fs::copy(&first, &second).unwrap();
    b.engine.settings().update(|s| s.parallel_jobs = 1).unwrap();

    let options = EncodeOptions { target_mb: 2.0, encoder: Some("libx264".into()), ..EncodeOptions::default() };
    let (_, first_done) = b.engine.submit_and_wait(spec(&first, options.clone()));
    let (_, second_done) = b.engine.submit_and_wait(spec(&second, options));
    // The second waits behind the first; it is gone by the time its turn comes.
    std::fs::remove_file(&second).unwrap();

    first_done.await.unwrap().expect("the first job is unaffected");
    let error = second_done.await.unwrap().unwrap_err();
    assert_eq!(error, "second.mp4 was moved or deleted while it was being compressed. Put it back and compress again.");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_read_only_results_folder_says_what_to_do() {
    use std::os::unix::fs::PermissionsExt;
    let Some(b) = bench().await else { return };
    let input = b.root.join("clip.mp4");
    make_source(&b.tools, &input, 3, "640x360").await;
    let locked = b.root.join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(locked.join("probe"), b"").is_ok() {
        eprintln!("skipped: running as a user who can write anywhere (root)");
        return;
    }

    let job = JobSpec { output_dir: Some(locked.clone()), ..spec(&input, EncodeOptions::default()) };
    let (_, done) = b.engine.submit_and_wait(job);
    let error = done.await.unwrap().unwrap_err();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(error.starts_with("Sub20 isn't allowed to save in"), "{error}");
    assert!(error.contains("Choose another folder for results in Settings"), "{error}");
}
