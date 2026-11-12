//! Turning "fit this in N MB" into numbers FFmpeg understands.
//!
//! Everything here is pure arithmetic, so the UI can call it on every drag of
//! a trim handle to show what the result will look like.

use serde::{Deserialize, Serialize};

use crate::encoders::Codec;
use crate::error::{Error, Result};
use crate::job::{AudioChoice, Container, EncodeOptions, Trim};
use crate::probe::MediaInfo;

/// Kilobits in one MB of target size. Using 8192 instead of the exact 8388.6
/// keeps about 2.3% back for container overhead.
const KBIT_PER_MB: f64 = 8192.0;

/// Below this the picture falls apart no matter the encoder.
pub const MIN_VIDEO_KBPS: f64 = 24.0;

/// Short-side heights the automatic resolution steps through.
const LADDER: [u32; 7] = [2160, 1440, 1080, 720, 480, 360, 240];

/// Weighted kbps per megapixel (see `density`) that looks good.
const DENSITY_OK: f64 = 550.0;
/// Under this the picture looks rough.
const DENSITY_FLOOR: f64 = 220.0;

/// The audio track that will be written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioCodec {
    Opus,
    Aac,
}

impl AudioCodec {
    pub fn encoder(self) -> &'static str {
        match self {
            AudioCodec::Opus => "libopus",
            AudioCodec::Aac => "aac",
        }
    }
}

/// How the result is expected to look, best first (so `max` is the worse).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Sharp,
    Good,
    Soft,
    Rough,
}

/// The decisions for one encode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// Where the kept part starts, in seconds.
    pub start: f64,
    /// Length of the kept part, in seconds.
    pub duration: f64,
    pub trimmed: bool,
    /// Size the result should come in under. `None` in fixed-bitrate mode.
    pub target_bytes: Option<u64>,
    pub video_kbps: u32,
    pub audio: Option<AudioCodec>,
    pub audio_kbps: u32,
    /// Output picture size, as displayed. Zero when the source size is unknown.
    pub width: u32,
    pub height: u32,
    /// True when the picture is scaled (or nudged to even dimensions).
    pub scaled: bool,
    /// Output frame rate limit, only set when the source is faster.
    pub fps_cap: Option<f64>,
    pub quality: Quality,
    /// Size the result is expected to have, in bytes.
    pub expected_bytes: u64,
    /// Plain-language notes about adjustments, shown in the job log.
    pub notes: Vec<String>,
}

/// `(total, video)` bitrate in kbps for a target size.
pub fn bitrates(target_mb: f64, duration: f64, audio_kbps: f64) -> (f64, f64) {
    if duration <= 0.0 {
        return (0.0, 0.0);
    }
    let total = target_mb * KBIT_PER_MB / duration;
    (total, (total - audio_kbps).max(0.0))
}

/// Validates a trim against the source and returns `(start, duration)`.
pub fn trim_window(source_duration: f64, trim: Option<Trim>) -> Result<(f64, f64, bool)> {
    let Some(trim) = trim else {
        return Ok((0.0, source_duration, false));
    };
    if !trim.start.is_finite() || trim.start < 0.0 {
        return Err(Error::BadTrim("the start must be zero or later".into()));
    }
    if !trim.end.is_finite() || trim.end <= trim.start {
        return Err(Error::BadTrim("the end must come after the start".into()));
    }
    if source_duration > 0.0 && trim.start >= source_duration {
        return Err(Error::BadTrim("the start is past the end of the video".into()));
    }
    let end = if source_duration > 0.0 { trim.end.min(source_duration) } else { trim.end };
    let duration = end - trim.start;
    // A trim that keeps everything (within a frame or so) is no trim.
    let whole = trim.start < 0.02 && source_duration > 0.0 && (source_duration - end) < 0.02;
    Ok((trim.start, duration, !whole))
}

fn even(value: f64) -> u32 {
    (((value / 2.0).floor() as u32) * 2).max(2)
}

/// kbps per megapixel, weighted the way the eye sees it: newer codecs do
/// more with the same bits, and higher frame rates need more. The resolution
/// choice and the quality verdict both use this, so they cannot disagree.
fn density(codec: Codec, video_kbps: f64, width: u32, height: u32, fps: f64) -> f64 {
    let efficiency = match codec {
        Codec::H264 => 1.0,
        Codec::Hevc => 1.5,
        Codec::Av1 => 1.8,
    };
    let motion = (fps / 30.0).clamp(0.8, 2.0).sqrt();
    let megapixels = f64::from(width) * f64::from(height) / 1_000_000.0;
    video_kbps * efficiency / (megapixels * motion)
}

/// Picks the short-side height to encode at, or `None` to keep the source
/// size: the size whose result will look best, judged by `rate` (which
/// weighs both how well the bits feed the pixels and how far the picture
/// was shrunk). Ties go to the larger picture, except when every choice
/// looks rough: then fewer pixels at least hold together.
///
/// Because each size's verdict only improves with more bitrate, so does the
/// best one: a shorter trim or a larger limit never looks worse (sharp may
/// give way to good as the picture grows)
/// (`shorter_trims_never_look_worse`).
pub fn auto_short_side(width: u32, height: u32, video_kbps: f64, codec: Codec, fps: f64, min_side: u32) -> Option<u32> {
    if width == 0 || height == 0 {
        return None;
    }
    let short = width.min(height);
    let floor = LADDER.iter().rev().copied().find(|r| *r >= min_side).unwrap_or(LADDER[0]);
    // The source, then the rungs below it and not under the floor, largest first.
    let mut sizes = vec![short];
    sizes.extend(LADDER.iter().copied().filter(|r| *r < short && *r >= floor.min(short)));
    let verdict = |side: u32| {
        let (w, h) = if side == short { (width, height) } else { scale_to_short_side(width, height, side) };
        rate(codec, video_kbps, w, h, fps, side != short)
    };
    // Good is good enough: a larger good picture beats a smaller sharp one.
    let verdict = |side: u32| verdict(side).max(Quality::Good);
    let best = sizes.iter().map(|s| verdict(*s)).min()?;
    let mut matching = sizes.into_iter().filter(|s| verdict(*s) == best);
    let side = if best == Quality::Rough { matching.next_back() } else { matching.next() }?;
    (side < short).then_some(side)
}

fn scale_to_short_side(width: u32, height: u32, side: u32) -> (u32, u32) {
    let ratio = f64::from(side) / f64::from(width.min(height));
    (even(f64::from(width) * ratio), even(f64::from(height) * ratio))
}

fn scale_to_limits(width: u32, height: u32, max_w: Option<u32>, max_h: Option<u32>) -> (u32, u32) {
    let mut ratio: f64 = 1.0;
    if let Some(w) = max_w.filter(|w| *w > 0) {
        ratio = ratio.min(f64::from(w) / f64::from(width));
    }
    if let Some(h) = max_h.filter(|h| *h > 0) {
        ratio = ratio.min(f64::from(h) / f64::from(height));
    }
    if ratio >= 1.0 {
        (width, height)
    } else {
        (even(f64::from(width) * ratio), even(f64::from(height) * ratio))
    }
}

/// How the result will look. `scaled` says the picture was shrunk to fit:
/// below 720p it looks small however well fed, so the verdict is capped.
fn rate(codec: Codec, video_kbps: f64, width: u32, height: u32, fps: f64, scaled: bool) -> Quality {
    if width == 0 || height == 0 {
        return Quality::Good;
    }
    let density = density(codec, video_kbps, width, height, fps);
    let by_bits = if density >= 1400.0 {
        Quality::Sharp
    } else if density >= DENSITY_OK {
        Quality::Good
    } else if density >= DENSITY_FLOOR {
        Quality::Soft
    } else {
        Quality::Rough
    };
    let short = width.min(height);
    let by_size = match (scaled, short) {
        (true, s) if s < 480 => Quality::Rough,
        (true, s) if s < 720 => Quality::Soft,
        _ => Quality::Sharp,
    };
    by_bits.max(by_size)
}

/// Lowest bitrate each audio codec is still listenable at.
fn audio_floor(codec: AudioCodec) -> u32 {
    match codec {
        AudioCodec::Opus => 24,
        AudioCodec::Aac => 48,
    }
}

/// Works out bitrates, resolution and frame rate for one job.
pub fn plan(info: &MediaInfo, trim: Option<Trim>, options: &EncodeOptions) -> Result<Plan> {
    let (start, duration, trimmed) = trim_window(info.duration, trim)?;
    if duration <= 0.0 {
        return Err(Error::Invalid("the video has no readable length".into()));
    }
    let mut notes = Vec::new();

    // Audio: what codec, if any.
    let audio = match options.audio {
        _ if !info.has_audio => None,
        AudioChoice::None if !options.audio_only => None,
        AudioChoice::Opus if options.audio_only => Some(AudioCodec::Opus),
        AudioChoice::Opus if options.container == Container::Mp4 => {
            notes.push("MP4 plays most reliably with AAC, so the audio is AAC instead of Opus.".into());
            Some(AudioCodec::Aac)
        }
        AudioChoice::Opus => Some(AudioCodec::Opus),
        AudioChoice::Aac | AudioChoice::None => Some(AudioCodec::Aac),
    };
    if audio.is_some() && info.audio_tracks > 1 {
        notes.push(if options.mix_audio {
            format!("The {} sound tracks are mixed into one.", info.audio_tracks)
        } else {
            format!("Only the first of {} sound tracks is kept.", info.audio_tracks)
        });
    }

    if options.audio_only {
        let Some(codec) = audio else {
            return Err(Error::Invalid("this video has no audio track to extract".into()));
        };
        let audio_kbps = options.audio_kbps.clamp(audio_floor(codec).max(64), 512);
        return Ok(Plan {
            start,
            duration,
            trimmed,
            target_bytes: None,
            video_kbps: 0,
            audio: Some(codec),
            audio_kbps,
            width: 0,
            height: 0,
            scaled: false,
            fps_cap: None,
            quality: Quality::Good,
            expected_bytes: (f64::from(audio_kbps) * 1000.0 / 8.0 * duration) as u64,
            notes,
        });
    }
    if !info.has_video {
        return Err(Error::Invalid("this file has no video track".into()));
    }

    // Bitrates.
    let fixed = options.video_kbps.filter(|k| *k > 0.0);
    let mut audio_kbps = if audio.is_some() { options.audio_kbps.clamp(16, 512) } else { 0 };
    let (video_kbps, target_bytes) = match fixed {
        Some(kbps) => (kbps, None),
        None => {
            if !(options.target_mb.is_finite() && options.target_mb > 0.0) {
                return Err(Error::Invalid("the target size must be more than zero".into()));
            }
            let (total, _) = bitrates(options.target_mb, duration, 0.0);
            if let Some(codec) = audio {
                // Audio may take at most a third of a tight budget.
                let ceiling = total * 0.34;
                if f64::from(audio_kbps) > ceiling {
                    let lowered = [256, 192, 160, 128, 96, 64, 48, 32, 24]
                        .into_iter()
                        .find(|step| f64::from(*step) <= ceiling && *step < audio_kbps)
                        .unwrap_or(0)
                        .max(audio_floor(codec));
                    if lowered < audio_kbps {
                        notes.push(format!(
                            "Audio lowered from {audio_kbps} to {lowered} kbps to leave room for the picture."
                        ));
                        audio_kbps = lowered;
                    }
                }
            }
            let video = total - f64::from(audio_kbps);
            if video < MIN_VIDEO_KBPS {
                return Err(Error::Invalid(format!(
                    "{} MB is too small for {:.0} seconds of video. Trim it shorter or raise the size.",
                    trim_number(options.target_mb),
                    duration
                )));
            }
            (video, Some((options.target_mb * 1024.0 * 1024.0) as u64))
        }
    };

    // Frame rate. Before the picture size, which depends on it.
    let fps_cap = match (options.max_fps.filter(|f| *f > 0.0), info.fps) {
        (Some(cap), Some(source)) if source > cap + 0.01 => {
            notes.push(format!("Frame rate capped at {cap} fps (source is {source:.2})."));
            Some(cap)
        }
        _ => None,
    };
    let fps = fps_cap.or(info.fps).unwrap_or(30.0);

    // Whole kbps from here on, so the size is chosen with the same number
    // the verdict is given with.
    let video_kbps = video_kbps.round().max(1.0) as u32;

    // Picture size.
    let (src_w, src_h) = (info.display_width, info.display_height);
    let (mut width, mut height) = (src_w, src_h);
    if src_w > 0 && src_h > 0 {
        if options.auto_resolution {
            let kbps = f64::from(video_kbps);
            if let Some(side) = auto_short_side(src_w, src_h, kbps, options.codec, fps, options.min_auto_height) {
                (width, height) = scale_to_short_side(src_w, src_h, side);
                notes.push(format!("Resolution lowered to {width}×{height} so the bitrate is not spread too thin."));
            }
        } else {
            if let Some(side) = options.max_short_side.filter(|s| *s >= 16 && *s < src_w.min(src_h)) {
                (width, height) = scale_to_short_side(src_w, src_h, side);
            }
            (width, height) = scale_to_limits(width, height, options.max_width, options.max_height);
        }
        // 4:2:0 encoders need even dimensions.
        if (width, height) == (src_w, src_h) && (src_w % 2 == 1 || src_h % 2 == 1) {
            (width, height) = (even(f64::from(src_w)), even(f64::from(src_h)));
        }
    }
    let scaled = (width, height) != (src_w, src_h);

    let expected_bytes = match target_bytes {
        Some(bytes) => bytes,
        None => (f64::from(video_kbps + audio_kbps) * 1000.0 / 8.0 * duration) as u64,
    };
    Ok(Plan {
        start,
        duration,
        trimmed,
        target_bytes,
        video_kbps,
        audio,
        audio_kbps,
        width,
        height,
        scaled,
        fps_cap,
        quality: rate(options.codec, f64::from(video_kbps), width, height, fps, scaled),
        expected_bytes,
        notes,
    })
}

/// `9.7` → "9.7", `10.0` → "10".
pub fn trim_number(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(w: u32, h: u32, duration: f64) -> MediaInfo {
        MediaInfo {
            duration,
            has_video: true,
            has_audio: true,
            width: w,
            height: h,
            display_width: w,
            display_height: h,
            fps: Some(30.0),
            ..MediaInfo::default()
        }
    }

    #[test]
    fn bitrate_math_matches_the_original() {
        // 9.7 MB over 60 s with 128 kbps audio.
        let (total, video) = bitrates(9.7, 60.0, 128.0);
        assert!((total - 1324.37).abs() < 0.01);
        assert!((video - 1196.37).abs() < 0.01);
    }

    #[test]
    fn trim_shortens_the_budget_window() {
        let info = source(1920, 1080, 120.0);
        let options = EncodeOptions { target_mb: 10.0, ..Default::default() };
        let whole = plan(&info, None, &options).unwrap();
        let cut = plan(&info, Some(Trim { start: 30.0, end: 60.0 }), &options).unwrap();
        assert!(!whole.trimmed && cut.trimmed);
        assert_eq!(cut.start, 30.0);
        assert_eq!(cut.duration, 30.0);
        // A quarter of the length gets roughly four times the bitrate.
        assert!(cut.video_kbps > whole.video_kbps * 3);
    }

    #[test]
    fn trim_validation() {
        assert!(trim_window(60.0, Some(Trim { start: 10.0, end: 5.0 })).is_err());
        assert!(trim_window(60.0, Some(Trim { start: 70.0, end: 80.0 })).is_err());
        assert!(trim_window(60.0, Some(Trim { start: -1.0, end: 5.0 })).is_err());
        // The end is clamped to the source.
        assert_eq!(trim_window(60.0, Some(Trim { start: 50.0, end: 90.0 })).unwrap(), (50.0, 10.0, true));
        // Keeping everything is not a trim.
        assert!(!trim_window(60.0, Some(Trim { start: 0.0, end: 60.0 })).unwrap().2);
    }

    #[test]
    fn auto_resolution_keeps_well_fed_sources() {
        assert_eq!(auto_short_side(1920, 1080, 4000.0, Codec::H264, 30.0, 240), None);
        assert_eq!(auto_short_side(1280, 720, 600.0, Codec::H264, 30.0, 240), None);
    }

    #[test]
    fn auto_resolution_steps_down_when_starved() {
        // 1080p at 900 kbps is 434 kbps/MP: tight, so drop to 720p.
        assert_eq!(auto_short_side(1920, 1080, 900.0, Codec::H264, 30.0, 240), Some(720));
        // 4K at 2 Mbps holds 1080p.
        assert_eq!(auto_short_side(3840, 2160, 2000.0, Codec::H264, 30.0, 240), Some(1080));
        // 1080p at 150 kbps is hopeless at 720p. 360p would be well fed but
        // looks small; 480p is a little soft, the best on offer.
        assert_eq!(auto_short_side(1920, 1080, 150.0, Codec::H264, 30.0, 240), Some(480));
        // The floor is respected.
        assert_eq!(auto_short_side(1920, 1080, 30.0, Codec::H264, 30.0, 480), Some(480));
        // Never upscale a small source.
        assert_eq!(auto_short_side(320, 240, 20.0, Codec::H264, 30.0, 240), None);
    }

    /// Trimming shorter gives every second more bits, so the verdict may
    /// only stay or improve. It once went from "720p, good" to "1080p, a
    /// little soft" when a 4K 60 fps clip was trimmed from 48 s to 40 s.
    /// Sharp may give way to good when the picture grows; that is a larger
    /// picture, not a worse one.
    #[test]
    fn shorter_trims_never_look_worse() {
        let sources = [
            (3840, 2160, 60.0, Codec::H264),
            (1920, 1080, 30.0, Codec::H264),
            (1080, 1920, 30.0, Codec::H264),
            (2560, 1440, 60.0, Codec::Hevc),
            (1280, 720, 30.0, Codec::Av1),
        ];
        for (w, h, fps, codec) in sources {
            for target_mb in [10.0, 20.0, 50.0] {
                let info = MediaInfo { fps: Some(fps), ..source(w, h, 900.0) };
                // No sound: its bitrate rises in steps, which can take a few
                // kbps from the picture as the trim shortens. The claim is
                // about the picture getting more bits.
                let options = EncodeOptions { target_mb, codec, audio: AudioChoice::None, ..Default::default() };
                let mut previous: Option<(f64, Quality)> = None;
                for tenths in (20..=6000).rev().step_by(7) {
                    let end = f64::from(tenths) / 10.0;
                    let Ok(p) = plan(&info, Some(Trim { start: 0.0, end }), &options) else { continue };
                    if let Some((longer, before)) = previous {
                        assert!(
                            p.quality.max(Quality::Good) <= before.max(Quality::Good),
                            "{w}x{h}@{fps} {codec:?} {target_mb} MB: {longer} s was {before:?}, {end} s is {:?} at {}x{}",
                            p.quality,
                            p.width,
                            p.height
                        );
                    }
                    previous = Some((end, p.quality));
                }
            }
        }
    }

    #[test]
    fn the_reported_case_keeps_its_verdict() {
        let info = MediaInfo { fps: Some(60.0), ..source(3840, 2160, 72.4) };
        let options = EncodeOptions { target_mb: 10.0, ..Default::default() };
        let longer = plan(&info, Some(Trim { start: 15.8, end: 64.3 }), &options).unwrap();
        let shorter = plan(&info, Some(Trim { start: 15.8, end: 55.4 }), &options).unwrap();
        assert!(shorter.quality <= longer.quality, "{:?} then {:?}", longer.quality, shorter.quality);
        assert!(shorter.height >= longer.height);
    }

    #[test]
    fn auto_resolution_treats_portrait_like_landscape() {
        assert_eq!(auto_short_side(1080, 1920, 900.0, Codec::H264, 30.0, 240), Some(720));
        let info = source(1080, 1920, 60.0);
        let options = EncodeOptions { target_mb: 8.0, ..Default::default() };
        let p = plan(&info, None, &options).unwrap();
        assert_eq!((p.width, p.height), (720, 1280));
        assert!(p.scaled);
    }

    #[test]
    fn manual_limits_and_even_dimensions() {
        let info = source(1920, 1080, 10.0);
        let options = EncodeOptions { auto_resolution: false, max_height: Some(480), ..Default::default() };
        let p = plan(&info, None, &options).unwrap();
        assert_eq!((p.width, p.height), (852, 480));

        // "720p" means the short side, whichever way the video is turned.
        let options = EncodeOptions { auto_resolution: false, max_short_side: Some(720), ..Default::default() };
        let p = plan(&source(1080, 1920, 10.0), None, &options).unwrap();
        assert_eq!((p.width, p.height), (720, 1280));
        let p = plan(&source(1920, 1080, 10.0), None, &options).unwrap();
        assert_eq!((p.width, p.height), (1280, 720));
        let p = plan(&source(640, 360, 10.0), None, &options).unwrap();
        assert_eq!((p.width, p.height), (640, 360));

        let odd = source(853, 479, 10.0);
        let options = EncodeOptions { auto_resolution: false, ..Default::default() };
        let p = plan(&odd, None, &options).unwrap();
        assert_eq!((p.width, p.height), (852, 478));
        assert!(p.scaled);
    }

    #[test]
    fn audio_yields_when_the_budget_is_tight() {
        // 4 MB over 5 minutes is 109 kbps in total.
        let info = source(1280, 720, 300.0);
        let options =
            EncodeOptions { target_mb: 4.0, container: Container::Mkv, audio: AudioChoice::Opus, ..Default::default() };
        let p = plan(&info, None, &options).unwrap();
        assert_eq!(p.audio, Some(AudioCodec::Opus));
        assert_eq!(p.audio_kbps, 32);
        assert!(p.video_kbps >= 70);
    }

    #[test]
    fn impossible_targets_are_refused_with_advice() {
        let info = source(1920, 1080, 3600.0);
        let options = EncodeOptions { target_mb: 4.0, ..Default::default() };
        let err = plan(&info, None, &options).unwrap_err().to_string();
        assert!(err.contains("too small"), "{err}");
    }

    #[test]
    fn mp4_swaps_opus_for_aac_and_silent_sources_get_no_audio() {
        let info = source(1280, 720, 30.0);
        let p = plan(&info, None, &EncodeOptions::default()).unwrap();
        assert_eq!(p.audio, Some(AudioCodec::Aac));

        let silent = MediaInfo { has_audio: false, ..source(1280, 720, 30.0) };
        let p = plan(&silent, None, &EncodeOptions::default()).unwrap();
        assert_eq!((p.audio, p.audio_kbps), (None, 0));
    }

    #[test]
    fn the_log_says_what_happens_to_several_sound_tracks() {
        let recording = MediaInfo { audio_tracks: 2, ..source(1920, 1080, 30.0) };
        let p = plan(&recording, None, &EncodeOptions::default()).unwrap();
        assert!(p.notes.iter().any(|n| n == "The 2 sound tracks are mixed into one."), "{:?}", p.notes);

        let first = EncodeOptions { mix_audio: false, ..Default::default() };
        let p = plan(&recording, None, &first).unwrap();
        assert!(p.notes.iter().any(|n| n == "Only the first of 2 sound tracks is kept."), "{:?}", p.notes);

        let muted = EncodeOptions { audio: AudioChoice::None, ..Default::default() };
        let p = plan(&recording, None, &muted).unwrap();
        assert!(!p.notes.iter().any(|n| n.contains("sound tracks")), "{:?}", p.notes);
    }

    #[test]
    fn frame_rate_cap_only_applies_to_faster_sources() {
        let mut info = source(1280, 720, 30.0);
        info.fps = Some(60.0);
        let options = EncodeOptions { max_fps: Some(30.0), ..Default::default() };
        assert_eq!(plan(&info, None, &options).unwrap().fps_cap, Some(30.0));
        info.fps = Some(24.0);
        assert_eq!(plan(&info, None, &options).unwrap().fps_cap, None);
    }

    #[test]
    fn fixed_bitrate_mode_ignores_the_size() {
        let info = source(1280, 720, 30.0);
        let options = EncodeOptions { video_kbps: Some(2500.0), ..Default::default() };
        let p = plan(&info, None, &options).unwrap();
        assert_eq!(p.video_kbps, 2500);
        assert_eq!(p.target_bytes, None);
    }
}
