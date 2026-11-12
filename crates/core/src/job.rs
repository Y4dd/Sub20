//! The data that describes a compression job and what it reports back.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::encoders::Codec;

pub type JobId = String;

/// Keep only the part of the video between `start` and `end` (seconds).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trim {
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioChoice {
    Opus,
    Aac,
    /// Drop the audio track.
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    Mp4,
    Mkv,
}

/// Speed against quality. `P1` is fastest, `P7` slowest; `Max` adds a second
/// pass on CPU encoders and uses the slowest preset everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    Max,
}

impl Effort {
    /// 0 for `P1` up to 7 for `Max`.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// NVENC tuning. Ignored by every other encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tune {
    Hq,
    Ll,
    Ull,
    Lossless,
}

impl Tune {
    pub fn id(self) -> &'static str {
        match self {
            Tune::Hq => "hq",
            Tune::Ll => "ll",
            Tune::Ull => "ull",
            Tune::Lossless => "lossless",
        }
    }
}

/// Everything the user can choose about how a video is compressed. This is
/// also what a saved profile stores.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EncodeOptions {
    /// Size limit for the finished file, in MB (1 MB = 1024 × 1024 bytes).
    pub target_mb: f64,
    /// When set, encode the video at this bitrate and ignore `target_mb`.
    pub video_kbps: Option<f64>,
    pub codec: Codec,
    /// A specific FFmpeg encoder. `None` picks the best one that works.
    pub encoder: Option<String>,
    pub audio: AudioChoice,
    pub audio_kbps: u32,
    pub effort: Effort,
    pub tune: Tune,
    pub container: Container,
    /// Let the planner lower the resolution when the bitrate is too thin.
    pub auto_resolution: bool,
    /// Auto resolution never goes below this (short side, pixels).
    pub min_auto_height: u32,
    /// Manual limit on the shorter side ("720p"), used when
    /// `auto_resolution` is off. Works the same for portrait and landscape.
    pub max_short_side: Option<u32>,
    /// Manual limits on each side, used when `auto_resolution` is off.
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    /// Cap the frame rate. Slower sources are left alone.
    pub max_fps: Option<f64>,
    /// Write only the audio track (`.m4a` or `.opus`).
    pub audio_only: bool,
    /// Fragmented MP4: no finalize step at the end, slightly less compatible.
    pub fast_finalize: bool,
    /// Ask the GPU to decode the source too.
    pub hardware_decode: bool,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        EncodeOptions {
            target_mb: 9.7,
            video_kbps: None,
            codec: Codec::H264,
            encoder: None,
            audio: AudioChoice::Opus,
            audio_kbps: 128,
            effort: Effort::P4,
            tune: Tune::Hq,
            container: Container::Mp4,
            auto_resolution: true,
            min_auto_height: 240,
            max_short_side: None,
            max_width: None,
            max_height: None,
            max_fps: None,
            audio_only: false,
            fast_finalize: false,
            hardware_decode: false,
        }
    }
}

/// Who asked for the job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    #[default]
    Manual,
    Watch,
}

/// One video to compress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSpec {
    pub input: PathBuf,
    #[serde(default)]
    pub trim: Option<Trim>,
    pub options: EncodeOptions,
    /// Where the result goes. `None` puts it next to the source.
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub origin: Origin,
}

/// What a finished job produced.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub output: PathBuf,
    pub size_bytes: u64,
    pub original_bytes: u64,
    /// Length of the result in seconds (after trimming).
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub video_kbps: u32,
    pub audio_kbps: u32,
    /// The FFmpeg encoder that produced the file.
    pub encoder: String,
    pub hardware: bool,
    /// The encoder first tried, when a fallback happened.
    pub fell_back_from: Option<String>,
    /// How many full encodes it took (1 unless the file came out too big).
    pub attempts: u32,
    pub elapsed_seconds: f64,
    /// `None` in fixed-bitrate mode.
    pub target_mb: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Analyzing,
    /// First pass of a two-pass encode.
    Analysis,
    Encoding,
    Finalizing,
}

/// Progress and lifecycle messages, sent to the UI as they happen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum JobEvent {
    Queued {
        id: JobId,
        input: PathBuf,
        origin: Origin,
    },
    Started {
        id: JobId,
        encoder: String,
        hardware: bool,
        video_kbps: u32,
        width: u32,
        height: u32,
    },
    Progress {
        id: JobId,
        phase: Phase,
        /// 0–100 for the current attempt. Starts over when a `Retry` happens.
        percent: f64,
        /// Encoding speed relative to real time.
        speed: Option<f64>,
        eta_seconds: Option<f64>,
        out_bytes: u64,
    },
    Log {
        id: JobId,
        line: String,
    },
    /// A hardware encoder failed and the job continues on another one.
    Fallback {
        id: JobId,
        from: String,
        to: String,
        reason: String,
    },
    /// The file came out over the limit and is being encoded again.
    Retry {
        id: JobId,
        attempt: u32,
        over_percent: f64,
        video_kbps: u32,
    },
    Done {
        id: JobId,
        result: JobResult,
    },
    Failed {
        id: JobId,
        error: String,
    },
    Canceled {
        id: JobId,
    },
}

impl JobEvent {
    pub fn id(&self) -> &str {
        match self {
            JobEvent::Queued { id, .. }
            | JobEvent::Started { id, .. }
            | JobEvent::Progress { id, .. }
            | JobEvent::Log { id, .. }
            | JobEvent::Fallback { id, .. }
            | JobEvent::Retry { id, .. }
            | JobEvent::Done { id, .. }
            | JobEvent::Failed { id, .. }
            | JobEvent::Canceled { id } => id,
        }
    }
}
