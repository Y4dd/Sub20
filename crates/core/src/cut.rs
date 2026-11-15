//! Cutting a finished result shorter without encoding it again.
//!
//! The packets are copied, so a cut is quick, loses nothing and can only make
//! the file smaller. A copy has to begin on a keyframe. In MP4 and MOV that
//! doesn't move the start: the packets from the keyframe before it are kept
//! and an edit list tells players to begin at the start itself (Chromium,
//! and so Discord's desktop player and Sub20's window, do). Matroska has no
//! edit list, so there the start moves back to the keyframe at or before it
//! ([`snap_start`]), and the window snaps its handle by the same rule, so the
//! cut is what was shown. The end lands within a frame or two (B-frames are
//! reordered past it).
//! Results get a keyframe every [`crate::command::KEYFRAME_SECONDS`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The shortest cut, the same as the shortest trim in the window.
pub const MIN_KEEP: f64 = 0.5;

/// A finished cut.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cut {
    /// The cut file: the result's own name, or a free name beside it when
    /// the uncut file could not go to the bin (then both are kept).
    pub output: PathBuf,
    pub size_bytes: u64,
    /// Length of the cut file in seconds.
    pub duration: f64,
    /// Where the cut starts in the uncut result, after snapping.
    pub start: f64,
    /// Why both files were kept, when they were.
    pub note: Option<String>,
}

/// Whether a copy of this file can begin between keyframes: MP4 and MOV keep
/// the packets before the start and hide them with an edit list.
pub fn starts_anywhere(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ["mp4", "m4v", "m4a", "mov"].iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// The keyframe at or before `start`. With no keyframes (a sound-only
/// result, where every packet can start) the start stays where it is.
pub fn snap_start(keyframes: &[f64], start: f64) -> f64 {
    if keyframes.is_empty() {
        return start.max(0.0);
    }
    keyframes.iter().copied().filter(|k| *k <= start + 0.001).fold(0.0, f64::max)
}

/// The part to keep, snapped, or why there is nothing to cut.
pub fn window(keyframes: &[f64], start: f64, end: f64, duration: f64) -> Result<(f64, f64)> {
    let end = end.min(duration);
    let mut start = snap_start(keyframes, start);
    if start < 0.02 {
        start = 0.0;
    }
    if start == 0.0 && duration - end < 0.02 {
        return Err(Error::Invalid("That keeps the whole result, so there is nothing to cut.".into()));
    }
    if end - start < MIN_KEEP {
        return Err(Error::Invalid("That would keep less than half a second.".into()));
    }
    Ok((start, end))
}

/// Keyframe times from `ffprobe -show_entries packet=pts_time,flags -of csv=p=0`,
/// one packet a line (`1.016667,K__`), in order.
pub fn parse_keyframes(csv: &str) -> Vec<f64> {
    let mut times: Vec<f64> = csv
        .lines()
        .filter_map(|line| {
            let (time, flags) = line.trim().split_once(',')?;
            flags.contains('K').then(|| time.parse::<f64>().ok()).flatten()
        })
        .collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    times
}

/// FFmpeg's arguments for copying `start..end` of `input` into `output`.
///
/// In MP4 the seek is the start itself, and the muxer writes the edit list
/// that begins playback there. Elsewhere the start is a keyframe, and the
/// seek lands half a second after it (less on a very short cut): Matroska's
/// seek goes to the keyframe before the time asked less the B-frame delay,
/// so a seek a millisecond past the keyframe went back to the one before it
/// (a cut from 2 s started at 0). The demuxer starts on the keyframe, and
/// the copy keeps everything from there.
pub fn args(input: &Path, start: f64, end: f64, output: &Path) -> Vec<OsString> {
    let mut a: Vec<OsString> = vec!["-y".into()];
    let seek = match (start > 0.0, starts_anywhere(output)) {
        (false, _) => 0.0,
        (true, true) => start,
        (true, false) => start + (0.5f64).min((end - start) / 2.0),
    };
    if seek > 0.0 {
        a.extend(["-ss".into(), format!("{seek:.3}").into()]);
    }
    a.extend(["-i".into(), input.as_os_str().to_owned()]);
    a.extend(["-t".into(), format!("{:.3}", end - seek).into()]);
    a.extend(["-map", "0", "-c", "copy", "-sn", "-dn"].map(OsString::from));
    if starts_anywhere(output) {
        a.extend(["-movflags", "+faststart"].map(OsString::from));
    }
    a.push(output.as_os_str().to_owned());
    a
}

/// The result's keyframe times. Empty for a file without video.
#[cfg(feature = "engine")]
pub async fn keyframes(tools: &crate::ffmpeg::Tools, path: &Path) -> Result<Vec<f64>> {
    let mut cmd = tools.ffprobe();
    cmd.args(["-v", "error", "-select_streams", "v:0", "-show_entries", "packet=pts_time,flags", "-of", "csv=p=0"])
        .arg(path);
    let out = crate::ffmpeg::run(&mut cmd, std::time::Duration::from_secs(120)).await?;
    if !out.ok {
        return Err(Error::Unreadable(path.to_path_buf(), crate::ffmpeg::tail(&out.stderr, 1)));
    }
    Ok(parse_keyframes(&out.stdout))
}

/// Copies `start..end` of `input` into `output`.
#[cfg(feature = "engine")]
pub async fn copy(tools: &crate::ffmpeg::Tools, input: &Path, start: f64, end: f64, output: &Path) -> Result<()> {
    let mut cmd = tools.ffmpeg();
    cmd.args(["-loglevel", "error"]).args(args(input, start, end, output));
    let out = crate::ffmpeg::run(&mut cmd, std::time::Duration::from_secs(600)).await?;
    if out.ok {
        Ok(())
    } else {
        Err(Error::Encode(crate::ffmpeg::tail(&out.stderr, 1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_moves_back_to_the_keyframe_before_it() {
        let keys = [0.0, 2.0, 4.0, 6.0];
        assert_eq!(snap_start(&keys, 3.9), 2.0);
        assert_eq!(snap_start(&keys, 4.0), 4.0);
        assert_eq!(snap_start(&keys, 3.9995), 4.0, "a keyframe printed a hair late still counts");
        assert_eq!(snap_start(&keys, 0.7), 0.0);
        assert_eq!(snap_start(&[], 3.3), 3.3, "sound alone can start anywhere");
    }

    #[test]
    fn a_cut_must_cut_something_and_keep_something() {
        let keys = [0.0, 2.0, 4.0];
        assert_eq!(window(&keys, 2.5, 5.0, 6.0).unwrap(), (2.0, 5.0));
        assert_eq!(
            window(&keys, 0.0, 9.0, 6.0).map_err(|e| e.to_string()),
            Err("That keeps the whole result, so there is nothing to cut.".into())
        );
        assert_eq!(
            window(&keys, 1.0, 5.995, 6.0).map_err(|_| ()),
            Err(()),
            "snapped back to the start, it keeps it all"
        );
        assert!(window(&keys, 4.1, 4.55, 6.0).is_ok(), "snapped to 4.0, so 0.45 s becomes 0.55 s");
        assert!(window(&[], 4.1, 4.55, 6.0).is_err(), "sound alone isn't snapped, so 0.45 s is too short");
    }

    #[test]
    fn keyframes_are_read_from_ffprobes_packets() {
        let csv = "0.000000,K__\n0.016667,___\nN/A,K__\n2.000000,K_\n1.983333,___\n4.000000,K__\n\n";
        assert_eq!(parse_keyframes(csv), [0.0, 2.0, 4.0]);
    }

    #[test]
    fn mp4_starts_anywhere_and_matroska_on_a_keyframe() {
        assert!(starts_anywhere(Path::new("/r/clip-20mb.mp4")));
        assert!(starts_anywhere(Path::new("/r/clip.MOV")));
        assert!(!starts_anywhere(Path::new("/r/clip-20mb.mkv")));
        assert!(!starts_anywhere(Path::new("/r/clip")));
    }

    #[test]
    fn the_copy_seeks_to_the_start_in_mp4_and_just_past_the_keyframe_elsewhere() {
        let text = |a: Vec<OsString>| a.iter().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ");
        assert_eq!(
            text(args(Path::new("/r/clip-20mb.mp4"), 3.25, 9.5, Path::new("/r/.clip.1.part.mp4"))),
            "-y -ss 3.250 -i /r/clip-20mb.mp4 -t 6.250 -map 0 -c copy -sn -dn -movflags +faststart /r/.clip.1.part.mp4"
        );
        assert_eq!(
            text(args(Path::new("/r/clip-20mb.mkv"), 4.0, 9.5, Path::new("/r/.clip.1.part.mkv"))),
            "-y -ss 4.500 -i /r/clip-20mb.mkv -t 5.000 -map 0 -c copy -sn -dn /r/.clip.1.part.mkv"
        );
        let from_start = text(args(Path::new("/r/a.mkv"), 0.0, 3.0, Path::new("/r/b.mkv")));
        assert!(from_start.starts_with("-y -i /r/a.mkv -t 3.000 "), "{from_start}");
        assert!(!from_start.contains("movflags"), "{from_start}");
    }
}
