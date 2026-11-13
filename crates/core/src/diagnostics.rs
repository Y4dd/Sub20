//! The text behind "Copy diagnostics": what a bug report needs, with the
//! user's home folder replaced by `~` so it can be pasted in public.

use std::fmt::Write as _;
use std::path::Path;

use crate::hardware::{Report, State};
use crate::settings::Settings;

/// Everything the report is made from. The shell gathers it.
pub struct Facts<'a> {
    pub app_version: &'a str,
    /// "FFmpeg included" or "without FFmpeg".
    pub build: &'a str,
    /// Operating system name, version and architecture.
    pub system: &'a str,
    /// `ffmpeg -version`'s first line and where FFmpeg was found.
    pub ffmpeg: Option<(&'a str, &'a Path)>,
    pub report: Option<&'a Report>,
    pub settings: &'a Settings,
    /// The end of the crash log, when there is one.
    pub crashes: Option<&'a str>,
    /// The FFmpeg log of the job the user was looking at.
    pub job_log: &'a [String],
    /// The user's home folder, to hide.
    pub home: Option<&'a Path>,
}

/// The report as Markdown, ready to paste into a GitHub issue.
pub fn report(facts: &Facts) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "**Sub20** {} ({})", facts.app_version, facts.build);
    let _ = writeln!(out, "**System** {}", facts.system);
    match facts.ffmpeg {
        Some((version, folder)) => {
            let _ = writeln!(out, "**FFmpeg** {version}, in {}", folder.display());
        }
        None => out.push_str("**FFmpeg** not found\n"),
    }

    let s = facts.settings;
    let parallel = if s.parallel_jobs == 0 { "automatic".to_string() } else { s.parallel_jobs.to_string() };
    let _ = writeln!(
        out,
        "**Settings** videos at once: {parallel}; switched-off encoders: {}; FFmpeg folder: {}",
        if s.disabled_encoders.is_empty() { "none".to_string() } else { s.disabled_encoders.join(", ") },
        if s.ffmpeg_folder.is_some() { "chosen" } else { "automatic" },
    );

    if let Some(report) = facts.report {
        out.push_str("\n**Encoders**\n");
        for encoder in report.encoders.iter().filter(|e| e.state != State::Elsewhere) {
            let state = match encoder.state {
                State::Works => "works",
                State::Failed => "failed",
                State::Missing => "not in this FFmpeg",
                State::Elsewhere => continue,
            };
            let _ = write!(out, "- {}: {state}", encoder.name);
            if let Some(device) = &encoder.device {
                let _ = write!(out, " on {device}");
            }
            if let (State::Failed, Some(detail)) = (encoder.state, &encoder.detail) {
                let _ = write!(out, " ({detail})");
            }
            out.push('\n');
        }
    }

    if let Some(crashes) = facts.crashes.filter(|c| !c.trim().is_empty()) {
        let _ = write!(out, "\n**Crash log**\n```\n{}\n```\n", crashes.trim_end());
    }
    if !facts.job_log.is_empty() {
        let _ = write!(out, "\n**FFmpeg log of the last job**\n```\n{}\n```\n", facts.job_log.join("\n"));
    }

    match facts.home {
        Some(home) => hide_home(&out, home),
        None => out,
    }
}

/// Replaces the home folder with `~`, in both slash styles on Windows.
fn hide_home(text: &str, home: &Path) -> String {
    let home = home.to_string_lossy();
    let home = home.trim_end_matches(['/', '\\']);
    if home.len() < 2 {
        return text.to_string();
    }
    text.replace(home, "~").replace(&home.replace('\\', "/"), "~")
}

/// The last `lines` lines of a log file, if it exists.
pub fn tail_of(path: &Path, lines: usize) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let all: Vec<&str> = text.lines().collect();
    Some(all[all.len().saturating_sub(lines)..].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoders::{Codec, Family};
    use crate::hardware::EncoderStatus;
    use std::path::PathBuf;

    fn status(name: &str, state: State, detail: Option<&str>) -> EncoderStatus {
        EncoderStatus {
            name: name.into(),
            label: name.into(),
            codec: Codec::H264,
            family: Family::Cpu,
            family_label: "CPU".into(),
            state,
            detail: detail.map(Into::into),
            device: None,
        }
    }

    #[test]
    fn the_report_names_what_a_bug_needs_and_hides_the_home_folder() {
        let tested = Report {
            encoders: vec![
                status("h264_nvenc", State::Works, None),
                status("h264_qsv", State::Failed, Some("No device available")),
                status("h264_videotoolbox", State::Elsewhere, None),
            ],
            ffmpeg_version: Some("ffmpeg version 9.0.2".into()),
            checked_at: 0,
            tone_map: true,
        };
        let home = PathBuf::from(r"C:\Users\sam");
        let folder = PathBuf::from(r"C:\Users\sam\scoop\shims");
        let log = vec![r"Input #0, mov, from 'C:/Users/sam/Videos/clip.mp4':".to_string()];
        let text = report(&Facts {
            app_version: "0.2.0",
            build: "without FFmpeg",
            system: "Windows 11 (x86_64)",
            ffmpeg: Some(("ffmpeg version 9.0.2", &folder)),
            report: Some(&tested),
            settings: &Settings::default(),
            crashes: Some("panicked at src/lib.rs:1:1: oops"),
            job_log: &log,
            home: Some(&home),
        });

        assert!(text.contains("**Sub20** 0.2.0 (without FFmpeg)"), "{text}");
        assert!(text.contains(r"in ~\scoop\shims"), "{text}");
        assert!(text.contains("- h264_nvenc: works"), "{text}");
        assert!(text.contains("- h264_qsv: failed (No device available)"), "{text}");
        assert!(!text.contains("videotoolbox"), "other systems' encoders are left out: {text}");
        assert!(text.contains("oops"), "{text}");
        assert!(text.contains("from '~/Videos/clip.mp4'"), "{text}");
        assert!(!text.contains("sam"), "the user's name never appears: {text}");
    }

    #[test]
    fn a_missing_ffmpeg_and_empty_logs_are_said_plainly() {
        let text = report(&Facts {
            app_version: "0.2.0",
            build: "FFmpeg included",
            system: "Linux",
            ffmpeg: None,
            report: None,
            settings: &Settings::default(),
            crashes: Some("  \n"),
            job_log: &[],
            home: None,
        });
        assert!(text.contains("**FFmpeg** not found"), "{text}");
        assert!(!text.contains("Crash log"), "{text}");
        assert!(!text.contains("last job"), "{text}");
    }
}
