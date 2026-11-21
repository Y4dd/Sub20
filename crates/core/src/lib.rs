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
//! * cut a finished result shorter without encoding it again ([`cut`]),
//! * write a bug report's diagnostics ([`diagnostics`]),
//! * say what went wrong in words that name the fix ([`advice`]),
//! * download FFmpeg for the lean build ([`fetch`]).

// The planner and its types: no FFmpeg, no runtime, builds for WebAssembly.
pub mod command;
pub mod cut;
pub mod encoders;
pub mod error;
pub mod history;
pub mod job;
pub mod plan;
pub mod probe;
pub mod progress;
pub mod settings;
pub mod timecode;
pub mod watched;

// The engine: runs FFmpeg and touches the system (the default `engine`
// feature).
#[cfg(feature = "engine")]
pub mod advice;
#[cfg(feature = "engine")]
pub mod crawl;
#[cfg(feature = "engine")]
pub mod diagnostics;
#[cfg(feature = "engine")]
pub mod engine;
#[cfg(feature = "engine")]
pub mod fetch;
#[cfg(feature = "engine")]
pub mod ffmpeg;
#[cfg(feature = "engine")]
pub mod hardware;
#[cfg(feature = "engine")]
pub mod output;
#[cfg(feature = "engine")]
pub mod thumbs;
#[cfg(feature = "engine")]
pub mod watch;

#[cfg(feature = "engine")]
pub use engine::{Engine, EventSink};
pub use error::{Error, Result};
pub use job::{EncodeOptions, JobEvent, JobId, JobResult, JobSpec, Trim};
pub use probe::MediaInfo;
