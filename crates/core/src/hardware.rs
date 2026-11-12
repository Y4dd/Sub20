//! Finding out which encoders really work on this machine.
//!
//! FFmpeg lists every encoder it was built with, whether or not the GPU and
//! driver behind it exist. So each candidate encodes a couple of real frames
//! before the app trusts it.

use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::encoders::{self, Codec, Encoder, Family, CATALOG, NEVER_AUTOMATIC};
use crate::ffmpeg::{self, Tools};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Encoded test frames successfully.
    Works,
    /// FFmpeg has it, but the test encode failed (no GPU, old driver, …).
    Failed,
    /// This FFmpeg build does not include it.
    Missing,
    /// Belongs to another operating system.
    Elsewhere,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderStatus {
    pub name: String,
    pub label: String,
    pub codec: Codec,
    pub family: Family,
    pub family_label: String,
    pub state: State,
    /// Why the test failed, in FFmpeg's words.
    pub detail: Option<String>,
    /// Render node that passed the test (Linux QSV/VAAPI).
    pub device: Option<String>,
}

/// The outcome of testing every encoder.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub encoders: Vec<EncoderStatus>,
    pub ffmpeg_version: Option<String>,
    /// Unix time of the test run.
    pub checked_at: u64,
}

impl Report {
    pub fn status(&self, name: &str) -> Option<&EncoderStatus> {
        self.encoders.iter().find(|e| e.name == name)
    }

    pub fn works(&self, name: &str) -> bool {
        self.status(name).is_some_and(|e| e.state == State::Works)
    }

    pub fn device(&self, name: &str) -> Option<&str> {
        self.status(name).and_then(|e| e.device.as_deref())
    }

    /// The preferred working encoder for a codec, skipping any the user
    /// switched off. Hardware comes first.
    pub fn best(&self, codec: Codec, disabled: &HashSet<String>) -> Option<Encoder> {
        encoders::for_codec(codec)
            .find(|e| self.works(e.name) && !disabled.contains(e.name) && !NEVER_AUTOMATIC.contains(&e.name))
    }

    /// The CPU encoder for a codec, if it works. Used when hardware fails
    /// mid-job; ignores the user's switches because the job must finish.
    pub fn cpu(&self, codec: Codec) -> Option<Encoder> {
        encoders::find(codec.cpu_encoder()).filter(|e| self.works(e.name))
    }
}

/// Render nodes under `/dev/dri`, with whether each belongs to an Intel GPU.
#[cfg(target_os = "linux")]
fn render_nodes() -> Vec<(String, bool)> {
    let mut nodes = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("renderD") {
                continue;
            }
            let vendor = std::fs::read_to_string(format!("/sys/class/drm/{name}/device/vendor")).unwrap_or_default();
            let intel = vendor.trim().eq_ignore_ascii_case("0x8086");
            nodes.push((format!("/dev/dri/{name}"), intel));
        }
    }
    nodes.sort();
    nodes
}

#[cfg(not(target_os = "linux"))]
fn render_nodes() -> Vec<(String, bool)> {
    Vec::new()
}

const SOURCE: &str = "color=black:s=256x256:d=0.2:r=10";

/// Device that passed (when the family needs one), or FFmpeg's complaint.
type TestOutcome = Result<Option<String>, String>;

/// One way of trying an encoder: flags before the input, an optional filter,
/// flags after the encoder name, and the device being tried.
type Attempt = (Vec<String>, Option<String>, Vec<&'static str>, Option<String>);

/// Encodes two frames with `encoder`.
async fn test(tools: &Tools, encoder: Encoder) -> TestOutcome {
    let limit = Duration::from_secs(25);
    let mut attempts: Vec<Attempt> = Vec::new();
    let s = |v: &str| v.to_string();
    match encoder.family {
        Family::Qsv if cfg!(windows) => {
            attempts.push((vec![s("-init_hw_device"), s("qsv=hw")], None, vec!["-pix_fmt", "yuv420p"], None));
            attempts.push((Vec::new(), None, vec!["-pix_fmt", "yuv420p"], None));
        }
        Family::Qsv => {
            for (node, intel) in render_nodes() {
                if !intel {
                    continue;
                }
                attempts.push((
                    vec![
                        s("-init_hw_device"),
                        format!("vaapi=va:{node}"),
                        s("-init_hw_device"),
                        s("qsv=hw@va"),
                        s("-filter_hw_device"),
                        s("hw"),
                    ],
                    Some(s("format=nv12,hwupload=extra_hw_frames=64")),
                    Vec::new(),
                    Some(node),
                ));
            }
        }
        Family::Vaapi => {
            for (node, _) in render_nodes() {
                attempts.push((
                    vec![s("-init_hw_device"), format!("vaapi=va:{node}"), s("-filter_hw_device"), s("va")],
                    Some(s("format=nv12|vaapi,hwupload")),
                    Vec::new(),
                    Some(node),
                ));
            }
        }
        Family::Mf => attempts.push((Vec::new(), None, vec!["-pix_fmt", "yuv420p", "-hw_encoding", "1"], None)),
        Family::VideoToolbox => attempts.push((Vec::new(), None, vec!["-pix_fmt", "yuv420p", "-allow_sw", "0"], None)),
        Family::Nvenc | Family::Amf | Family::Cpu => {
            attempts.push((Vec::new(), None, vec!["-pix_fmt", "yuv420p"], None))
        }
    }
    if attempts.is_empty() {
        return Err("no compatible graphics device was found".into());
    }

    let mut last_error = String::new();
    for (before, filter, after, device) in attempts {
        let mut cmd = tools.ffmpeg();
        cmd.args(["-y", "-loglevel", "error"]).args(&before).args(["-f", "lavfi", "-i", SOURCE]);
        if let Some(filter) = &filter {
            cmd.args(["-vf", filter]);
        }
        cmd.args(["-c:v", encoder.name]).args(&after);
        if encoder.name == "libaom-av1" {
            cmd.args(["-cpu-used", "8"]);
        }
        cmd.args(["-frames:v", "2", "-f", "null", "-"]);
        match ffmpeg::run(&mut cmd, limit).await {
            Ok(out) if out.ok => return Ok(device),
            Ok(out) => last_error = ffmpeg::tail(&out.stderr, 2),
            Err(e) => last_error = e.to_string(),
        }
    }
    if last_error.is_empty() {
        last_error = "the test encode failed".into();
    }
    Err(last_error)
}

/// Names of the encoders this FFmpeg build includes.
async fn built_in(tools: &Tools) -> HashSet<String> {
    let mut cmd = tools.ffmpeg();
    cmd.arg("-encoders");
    let Ok(out) = ffmpeg::run(&mut cmd, Duration::from_secs(15)).await else {
        return HashSet::new();
    };
    // Lines look like ` V....D libx264   libx264 H.264 / AVC …`.
    out.stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let flags = parts.next()?;
            let name = parts.next()?;
            let is_flags = flags.len() >= 6 && flags.chars().all(|c| c == '.' || c.is_ascii_uppercase());
            (is_flags && flags.starts_with('V')).then(|| name.to_string())
        })
        .collect()
}

/// Tests every encoder in the catalog. Takes a few seconds.
pub async fn detect(tools: &Tools) -> Report {
    let listed = built_in(tools).await;
    let mut report = Report {
        encoders: Vec::with_capacity(CATALOG.len()),
        ffmpeg_version: tools.version().await,
        checked_at: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    };

    // Families are tested side by side; encoders within one family take
    // turns, because consumer GPUs limit simultaneous encode sessions.
    let mut families: JoinSet<Vec<(Encoder, TestOutcome)>> = JoinSet::new();
    let mut seen = HashSet::new();
    for family in CATALOG.iter().map(|e| e.family) {
        if !seen.insert(family) || !family.applies_here() {
            continue;
        }
        let candidates: Vec<Encoder> =
            CATALOG.iter().copied().filter(|e| e.family == family && listed.contains(e.name)).collect();
        let tools = tools.clone();
        families.spawn(async move {
            let mut results = Vec::new();
            for encoder in candidates {
                let outcome = test(&tools, encoder).await;
                results.push((encoder, outcome));
            }
            results
        });
    }
    let mut outcomes = std::collections::HashMap::new();
    while let Some(joined) = families.join_next().await {
        for (encoder, outcome) in joined.unwrap_or_default() {
            outcomes.insert(encoder.name, outcome);
        }
    }

    for encoder in CATALOG {
        let (state, detail, device) = if !encoder.family.applies_here() {
            (State::Elsewhere, None, None)
        } else if !listed.contains(encoder.name) {
            (State::Missing, None, None)
        } else {
            match outcomes.remove(encoder.name) {
                Some(Ok(device)) => (State::Works, None, device),
                Some(Err(why)) => (State::Failed, Some(why), None),
                None => (State::Failed, Some("the test did not run".into()), None),
            }
        };
        report.encoders.push(EncoderStatus {
            name: encoder.name.to_string(),
            label: encoders::describe(encoder.name),
            codec: encoder.codec,
            family: encoder.family,
            family_label: encoder.family.label().to_string(),
            state,
            detail,
            device,
        });
    }
    log::info!(
        "encoders that work: {}",
        report
            .encoders
            .iter()
            .filter(|e| e.state == State::Works)
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(working: &[&str]) -> Report {
        Report {
            encoders: CATALOG
                .iter()
                .map(|e| EncoderStatus {
                    name: e.name.into(),
                    label: encoders::describe(e.name),
                    codec: e.codec,
                    family: e.family,
                    family_label: e.family.label().into(),
                    state: if working.contains(&e.name) { State::Works } else { State::Failed },
                    detail: None,
                    device: None,
                })
                .collect(),
            ..Report::default()
        }
    }

    #[test]
    fn hardware_wins_when_it_works() {
        let r = report(&["libx264", "h264_nvenc", "libx265", "libsvtav1", "libaom-av1"]);
        let none = HashSet::new();
        assert_eq!(r.best(Codec::H264, &none).unwrap().name, "h264_nvenc");
        assert_eq!(r.best(Codec::Hevc, &none).unwrap().name, "libx265");
        assert_eq!(r.best(Codec::Av1, &none).unwrap().name, "libsvtav1");
    }

    #[test]
    fn switched_off_encoders_are_skipped() {
        let r = report(&["libx264", "h264_nvenc"]);
        let off: HashSet<String> = ["h264_nvenc".to_string()].into();
        assert_eq!(r.best(Codec::H264, &off).unwrap().name, "libx264");
        assert!(r.best(Codec::Hevc, &off).is_none());
    }

    #[test]
    fn libaom_is_never_picked_automatically() {
        let r = report(&["libaom-av1"]);
        assert!(r.best(Codec::Av1, &HashSet::new()).is_none());
        assert!(r.works("libaom-av1"));
    }
}
