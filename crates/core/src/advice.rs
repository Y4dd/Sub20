//! Turning a failed job into a message that names the cause and the fix.
//!
//! FFmpeg's last words ("moov atom not found", "No space left on device")
//! are accurate but don't say what to do. The common failures that aren't
//! Sub20's fault get a plain sentence; anything else keeps FFmpeg's line.

use std::path::Path;

use crate::error::Error;

/// What was known about the job when it failed.
pub struct Situation<'a> {
    pub input: &'a Path,
    /// The folder the result was being written to.
    pub out_dir: &'a Path,
}

/// The message the window shows for a failed job.
pub fn explain(error: &Error, at: &Situation) -> String {
    let name = at
        .input
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| at.input.display().to_string());
    let dir = at.out_dir.display();
    let text = error.to_string();
    let lower = text.to_lowercase();

    // Checked first: a missing file explains every error that follows from it.
    if !at.input.exists() {
        return format!("{name} was moved or deleted while it was being compressed. Put it back and compress again.");
    }
    if !at.out_dir.is_dir() {
        return format!(
            "The folder for results, {dir}, is no longer there. Put it back, or choose another folder in Settings."
        );
    }
    if disk_full(error, &lower) {
        return format!(
            "The disk is full: there was no room for the result in {dir}. Free some space, or choose a folder on another disk in Settings."
        );
    }
    if not_allowed(error, &lower) {
        // FFmpeg names the file it couldn't open. The partial result is the
        // only thing it writes; anything else it was refused is the source.
        let source_refused = matches!(error, Error::Encode(_) | Error::Unreadable(..)) && !lower.contains(".part.");
        return if source_refused {
            format!("Sub20 isn't allowed to read {name}. Check the file's permissions, or copy it to a folder you own.")
        } else {
            format!(
                "Sub20 isn't allowed to save in {dir}. Choose another folder for results in Settings, or check that the folder isn't read-only."
            )
        };
    }
    if damaged(error, &lower) {
        return format!(
            "{name} couldn't be read as a video ({}). It may be damaged, or still being recorded or copied: if so, wait until that's done and try again.",
            detail(error)
        );
    }
    text
}

fn os_error(error: &Error) -> Option<i32> {
    match error {
        Error::Io { source, .. } => source.raw_os_error(),
        _ => None,
    }
}

fn disk_full(error: &Error, lower: &str) -> bool {
    // ENOSPC on Linux and macOS; ERROR_HANDLE_DISK_FULL and ERROR_DISK_FULL on Windows.
    let codes: &[i32] = if cfg!(windows) { &[39, 112] } else { &[28] };
    os_error(error).is_some_and(|code| codes.contains(&code))
        || lower.contains("no space left on device")
        || lower.contains("not enough space on the disk")
        || lower.contains("disk quota exceeded")
}

fn not_allowed(error: &Error, lower: &str) -> bool {
    // EROFS on Linux and macOS.
    let read_only = !cfg!(windows) && os_error(error) == Some(30);
    matches!(error, Error::Io { source, .. } if source.kind() == std::io::ErrorKind::PermissionDenied)
        || read_only
        || ["permission denied", "access is denied", "read-only file system", "operation not permitted"]
            .iter()
            .any(|words| lower.contains(words))
}

fn damaged(error: &Error, lower: &str) -> bool {
    matches!(error, Error::Unreadable(..))
        || [
            "invalid data found",
            "moov atom not found",
            "corrupt",
            "truncat",
            "partial file",
            "end of file",
            "error while decoding",
            "invalid nal",
            "ebml header parsing failed",
        ]
        .iter()
        .any(|words| lower.contains(words))
}

/// FFmpeg's own words, without the path in front of them.
fn detail(error: &Error) -> String {
    let raw = match error {
        Error::Unreadable(_, detail) | Error::Encode(detail) => detail.clone(),
        other => other.to_string(),
    };
    // "C:\clips\a.mp4: Invalid data found…" → "Invalid data found…"
    match raw.rsplit_once(": ") {
        Some((_, words)) if !words.is_empty() => words.trim().to_string(),
        _ => raw.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Bench {
        _dir: tempfile::TempDir,
        input: PathBuf,
        out: PathBuf,
    }

    fn bench() -> Bench {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, b"x").unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        Bench { input, out, _dir: dir }
    }

    fn say(error: Error, b: &Bench) -> String {
        explain(&error, &Situation { input: &b.input, out_dir: &b.out })
    }

    #[test]
    fn a_source_that_vanished_is_named() {
        let b = bench();
        std::fs::remove_file(&b.input).unwrap();
        let message = say(Error::Encode("clip.mp4: No such file or directory".into()), &b);
        assert!(message.starts_with("clip.mp4 was moved or deleted"), "{message}");
    }

    #[test]
    fn a_results_folder_that_vanished_is_named() {
        let b = bench();
        std::fs::remove_dir(&b.out).unwrap();
        let message = say(Error::Encode("Error opening output files: No such file or directory".into()), &b);
        assert!(message.starts_with("The folder for results"), "{message}");
        assert!(message.contains("choose another folder in Settings"), "{message}");
    }

    #[test]
    fn a_full_disk_is_named_from_ffmpeg_and_from_the_system() {
        let b = bench();
        let from_ffmpeg = say(Error::Encode("Error writing trailer: No space left on device".into()), &b);
        assert!(from_ffmpeg.starts_with("The disk is full"), "{from_ffmpeg}");

        let code = if cfg!(windows) { 112 } else { 28 };
        let from_system = say(Error::io("could not move the result", std::io::Error::from_raw_os_error(code)), &b);
        assert!(from_system.starts_with("The disk is full"), "{from_system}");
    }

    #[test]
    fn a_folder_that_refuses_writing_is_told_apart_from_a_source_that_refuses_reading() {
        let b = bench();
        let write = say(Error::Encode("/out/.clip.1234abcd.part.mp4: Permission denied".into()), &b);
        assert!(write.starts_with("Sub20 isn't allowed to save in"), "{write}");

        let create =
            say(Error::io("could not create /out", std::io::Error::from(std::io::ErrorKind::PermissionDenied)), &b);
        assert!(create.starts_with("Sub20 isn't allowed to save in"), "{create}");

        let read = say(Error::Encode("clip.mp4: Permission denied".into()), &b);
        assert!(read.starts_with("Sub20 isn't allowed to read clip.mp4"), "{read}");
    }

    #[test]
    fn a_damaged_source_says_so_and_keeps_ffmpegs_words() {
        let b = bench();
        let message = say(Error::Unreadable(b.input.clone(), "C:\\clips\\clip.mp4: moov atom not found".into()), &b);
        assert!(message.starts_with("clip.mp4 couldn't be read as a video"), "{message}");
        assert!(message.contains("(moov atom not found)"), "{message}");
        assert!(message.contains("still being recorded or copied"), "{message}");
    }

    #[test]
    fn anything_else_keeps_the_original_message() {
        let b = bench();
        let message = say(Error::Encode("[h264_nvenc @ 0x1] Cannot load libcuda.so.1".into()), &b);
        assert_eq!(message, "FFmpeg stopped with an error: [h264_nvenc @ 0x1] Cannot load libcuda.so.1");
    }
}
