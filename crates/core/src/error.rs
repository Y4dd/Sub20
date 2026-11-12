use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("FFmpeg was not found. Install FFmpeg or choose its folder in Settings.")]
    FfmpegMissing,

    #[error("{0} is not a video FFmpeg can read: {1}")]
    Unreadable(PathBuf, String),

    #[error("{0} has no video stream")]
    NoVideo(PathBuf),

    #[error("{0} has no audio stream, so there is nothing to extract")]
    NoAudio(PathBuf),

    #[error("the trim range is not valid: {0}")]
    BadTrim(String),

    #[error("{0}")]
    Invalid(String),

    #[error("FFmpeg stopped with an error: {0}")]
    Encode(String),

    #[error("canceled")]
    Canceled,

    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error("could not read {what}: {source}")]
    Json {
        what: String,
        #[source]
        source: serde_json::Error,
    },
}

impl Error {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Error::Io { context: context.into(), source }
    }
}

/// Adds a short description to an `io::Error` on the way up.
pub trait IoContext<T> {
    fn ctx(self, context: impl Into<String>) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn ctx(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| Error::io(context, e))
    }
}
