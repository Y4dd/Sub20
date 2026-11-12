//! `sub20-core` — the engine behind the Sub20 desktop app.
//!
//! The crate has no GUI dependency. It knows how to:
//!
//! * locate FFmpeg and FFprobe ([`ffmpeg`]),
//! * read a video's facts ([`probe`]),
//! * turn "fit this in N MB" into bitrates, a resolution and a frame rate
//!   ([`plan`]),
//! * find out which encoders really work on this machine ([`hardware`]),
//! * build the FFmpeg command line ([`command`]),
//! * run a queue of jobs with progress, cancel, CPU fallback and a re-encode
//!   when a file lands over its target ([`engine`]),
//! * remember settings, profiles and history ([`settings`], [`history`]),
//! * watch a folder and compress whatever appears in it ([`watch`]),
//! * cut thumbnails for the trim timeline ([`thumbs`]),
//! * write a bug report's diagnostics ([`diagnostics`]).

pub mod command;
pub mod diagnostics;
pub mod encoders;
pub mod engine;
pub mod error;
pub mod ffmpeg;
pub mod hardware;
pub mod history;
pub mod job;
pub mod output;
pub mod plan;
pub mod probe;
pub mod progress;
pub mod settings;
pub mod thumbs;
pub mod timecode;
pub mod watch;

pub use engine::{Engine, EventSink};
pub use error::{Error, Result};
pub use job::{EncodeOptions, JobEvent, JobId, JobResult, JobSpec, Trim};
pub use probe::MediaInfo;
