//! Compresses videos from the command line with the same engine the app
//! uses, and prints what happened. Handy for trying real clips.
//!
//! cargo run -p sub20-core --example compress -- [--mb 20] [--encoder NAME] [--out DIR] FILE...

use std::path::PathBuf;
use std::sync::Arc;

use sub20_core::job::{EncodeOptions, JobEvent, JobSpec, Origin};
use sub20_core::Engine;

#[tokio::main]
async fn main() {
    let mut options = EncodeOptions::default();
    let mut out: Option<PathBuf> = None;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mb" => options.target_mb = args.next().and_then(|v| v.parse().ok()).expect("--mb needs a number"),
            "--encoder" => options.encoder = args.next(),
            "--out" => out = args.next().map(PathBuf::from),
            _ => files.push(PathBuf::from(arg)),
        }
    }
    if files.is_empty() {
        eprintln!("usage: compress [--mb 20] [--encoder NAME] [--out DIR] FILE...");
        std::process::exit(2);
    }

    let scratch = std::env::temp_dir().join("sub20-example");
    let engine = Engine::new(
        &scratch.join("data"),
        &scratch.join("work"),
        Vec::new(),
        Arc::new(|event: JobEvent| match event {
            JobEvent::Started { encoder, video_kbps, width, height, .. } => {
                println!("  started: {encoder}, {width}x{height}, {video_kbps} kbps")
            }
            JobEvent::Retry { .. } | JobEvent::Fallback { .. } => println!("  {event:?}"),
            JobEvent::Log { line, .. } if !line.starts_with('"') && !line.contains("ffmpeg") => {
                println!("  note: {line}")
            }
            _ => {}
        }),
    );
    // History off: trying clips shouldn't fill the app's history.
    engine.settings().update(|s| s.history_enabled = false).expect("settings");

    for file in files {
        println!("{}", file.display());
        let spec = JobSpec {
            input: file,
            trim: None,
            options: options.clone(),
            output_dir: out.clone(),
            origin: Origin::Manual,
            output_name: None,
            replace_original: false,
        };
        let (_, done) = engine.submit_and_wait(spec);
        match done.await.expect("engine stopped") {
            Ok(r) => println!(
                "  done: {:.2} MB of {:.0} MB ({:.1}% of the limit), {}x{}, {} kbps, {} in {:.1}s, {} attempt(s){}",
                r.size_bytes as f64 / 1048576.0,
                r.original_bytes as f64 / 1048576.0,
                r.size_bytes as f64 / (options.target_mb * 1048576.0) * 100.0,
                r.width,
                r.height,
                r.video_kbps,
                r.encoder,
                r.elapsed_seconds,
                r.attempts,
                r.fell_back_from.map(|f| format!(", fell back from {f}")).unwrap_or_default(),
            ),
            Err(error) => println!("  failed: {error}"),
        }
    }
}
