//! Building FFmpeg argument lists.

use std::ffi::OsString;
use std::path::Path;

use crate::encoders::{Codec, Encoder, Family};
use crate::job::{Container, Effort, EncodeOptions};
use crate::plan::Plan;
use crate::probe::MediaInfo;

/// Which pass of the encode this command is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    Single,
    /// Analysis pass: writes statistics, no output file.
    First,
    Second,
}

/// Everything needed to write one command line.
#[derive(Clone, Debug)]
pub struct EncodeArgs<'a> {
    pub input: &'a Path,
    pub output: &'a Path,
    pub info: &'a MediaInfo,
    pub plan: &'a Plan,
    pub options: &'a EncodeOptions,
    pub encoder: Encoder,
    /// VAAPI render node for QSV/VAAPI on Linux, e.g. `/dev/dri/renderD128`.
    pub device: Option<&'a str>,
    /// Video bitrate for this attempt. Starts at the plan's and drops when a
    /// result comes out too big.
    pub video_kbps: u32,
    pub hardware_decode: bool,
    pub pass: Pass,
}

struct Args(Vec<OsString>);

impl Args {
    fn push(&mut self, value: impl Into<OsString>) -> &mut Self {
        self.0.push(value.into());
        self
    }
    fn extend<I, S>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.0.extend(values.into_iter().map(Into::into));
        self
    }
}

/// True when this encoder runs a second pass at [`Effort::Max`].
pub fn uses_two_passes(encoder: Encoder, options: &EncodeOptions) -> bool {
    options.effort == Effort::Max && options.video_kbps.is_none() && matches!(encoder.name, "libx264" | "libx265")
}

/// Device setup that must come before the input.
fn device_flags(encoder: Encoder, device: Option<&str>) -> Vec<String> {
    let s = |v: &str| v.to_string();
    match encoder.family {
        Family::Qsv if cfg!(windows) => {
            vec![s("-init_hw_device"), s("qsv=hw"), s("-filter_hw_device"), s("hw")]
        }
        Family::Qsv => {
            let device = device.unwrap_or("/dev/dri/renderD128");
            vec![
                s("-init_hw_device"),
                format!("vaapi=va:{device}"),
                s("-init_hw_device"),
                s("qsv=hw@va"),
                s("-filter_hw_device"),
                s("hw"),
            ]
        }
        Family::Vaapi => {
            let device = device.unwrap_or("/dev/dri/renderD128");
            vec![s("-init_hw_device"), format!("vaapi=va:{device}"), s("-filter_hw_device"), s("va")]
        }
        _ => Vec::new(),
    }
}

/// The frame format handed to QSV/VAAPI. HEVC keeps 10-bit sources 10-bit.
fn surface_format(encoder: Encoder, info: &MediaInfo) -> &'static str {
    let ten_bit_capable = matches!(encoder.name, "hevc_qsv" | "hevc_vaapi");
    if ten_bit_capable && info.is_high_bit_depth() {
        "p010le"
    } else {
        "nv12"
    }
}

/// The `-vf` chain: scale, frame-rate cap, then upload to the GPU if needed.
fn video_filters(args: &EncodeArgs) -> Vec<String> {
    let plan = args.plan;
    let mut filters = Vec::new();
    if plan.scaled && plan.width > 0 && plan.height > 0 {
        filters.push(format!("scale={}:{}:flags=bicubic", plan.width, plan.height));
    }
    if let Some(cap) = plan.fps_cap {
        filters.push(format!("fps=fps={cap}"));
    }
    let format = surface_format(args.encoder, args.info);
    match args.encoder.family {
        Family::Qsv if cfg!(windows) => filters.push(format!("format={format}")),
        Family::Qsv => filters.push(format!("format={format},hwupload=extra_hw_frames=64")),
        Family::Vaapi => filters.push(format!("format={format}|vaapi,hwupload")),
        _ => {}
    }
    filters
}

/// Pixel format and profile flags.
fn format_flags(encoder: Encoder, info: &MediaInfo) -> Vec<&'static str> {
    match encoder.family {
        Family::Qsv | Family::Vaapi => {
            if surface_format(encoder, info) == "p010le" {
                vec!["-profile:v", "main10"]
            } else {
                Vec::new()
            }
        }
        Family::Mf => vec!["-pix_fmt", "yuv420p", "-hw_encoding", "1"],
        Family::VideoToolbox => match encoder.codec {
            Codec::H264 => vec!["-pix_fmt", "yuv420p", "-allow_sw", "0", "-profile:v", "high"],
            _ => vec!["-pix_fmt", "yuv420p", "-allow_sw", "0", "-profile:v", "main"],
        },
        Family::Nvenc | Family::Amf => match encoder.codec {
            Codec::H264 => vec!["-pix_fmt", "yuv420p", "-profile:v", "high"],
            Codec::Hevc => vec!["-pix_fmt", "yuv420p", "-profile:v", "main"],
            Codec::Av1 => vec!["-pix_fmt", "yuv420p"],
        },
        Family::Cpu if encoder.name == "libx264" => vec!["-pix_fmt", "yuv420p", "-profile:v", "high"],
        Family::Cpu => vec!["-pix_fmt", "yuv420p"],
    }
}

/// Speed/quality flags. Each family has its own vocabulary.
fn effort_flags(encoder: Encoder, options: &EncodeOptions) -> Vec<String> {
    let i = options.effort.index();
    let pair = |flag: &str, value: &str| vec![flag.to_string(), value.to_string()];
    match (encoder.family, encoder.name) {
        (Family::Nvenc, _) => {
            let preset = ["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p7"][i];
            let mut flags = pair("-preset", preset);
            flags.extend(pair("-tune", options.tune.id()));
            flags
        }
        (Family::Qsv, _) => {
            let preset = ["veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow", "veryslow"][i];
            pair("-preset", preset)
        }
        // These take driver-specific controls; FFmpeg's generic preset does
        // not travel well, so rate control alone steers them.
        (Family::Vaapi | Family::Amf | Family::Mf | Family::VideoToolbox, _) => Vec::new(),
        (_, "libx264") => {
            let preset = ["ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow", "veryslow"][i];
            let mut flags = pair("-preset", preset);
            flags.extend(pair("-tune", "film"));
            flags
        }
        (_, "libx265") => {
            let preset = ["ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow", "slower"][i];
            pair("-preset", preset)
        }
        (_, "libsvtav1") => {
            // SVT-AV1 presets run from 13 (fastest) to 0 (slowest).
            let preset = ["12", "10", "9", "8", "7", "6", "4", "3"][i];
            pair("-preset", preset)
        }
        (_, "libaom-av1") => {
            let speed = ["8", "7", "6", "5", "4", "4", "2", "1"][i];
            let mut flags = pair("-cpu-used", speed);
            flags.extend(pair("-row-mt", "1"));
            flags
        }
        _ => Vec::new(),
    }
}

/// Average bitrate with a 1.2× peak and a two-second buffer. SVT-AV1 does not
/// accept a peak without CRF, so it gets the average alone.
fn rate_flags(encoder: Encoder, kbps: u32) -> Vec<String> {
    let mut flags = vec!["-b:v".to_string(), format!("{kbps}k")];
    if encoder.name != "libsvtav1" {
        flags.extend([
            "-maxrate".to_string(),
            format!("{}k", (f64::from(kbps) * 1.2) as u32),
            "-bufsize".to_string(),
            format!("{}k", kbps * 2),
        ]);
    }
    flags
}

/// The extension the output file gets.
pub fn extension(plan: &Plan, options: &EncodeOptions) -> &'static str {
    if options.audio_only {
        match plan.audio {
            Some(crate::plan::AudioCodec::Opus) => "opus",
            _ => "m4a",
        }
    } else {
        match options.container {
            Container::Mp4 => "mp4",
            Container::Mkv => "mkv",
        }
    }
}

fn seconds(value: f64) -> String {
    format!("{value:.3}")
}

/// Builds the arguments that follow the `ffmpeg` binary.
///
/// Two-pass encodes write their statistics to the working directory, so the
/// caller must run both passes from the same private folder.
pub fn build(args: &EncodeArgs) -> Vec<OsString> {
    let plan = args.plan;
    let mut a = Args(Vec::new());
    a.push("-y");

    if args.hardware_decode && !args.options.audio_only {
        // Frames come back to system memory, so the filters below still apply.
        a.extend(["-hwaccel", "auto"]);
    }
    if !args.options.audio_only {
        a.extend(device_flags(args.encoder, args.device));
    }
    // Seeking before the input is fast and, because we re-encode, exact.
    if plan.trimmed && plan.start > 0.0 {
        a.push("-ss").push(seconds(plan.start));
    }
    a.push("-i").push(args.input.as_os_str());
    if plan.trimmed {
        a.push("-t").push(seconds(plan.duration));
    }

    if args.options.audio_only {
        a.extend(["-map", "0:a:0", "-vn", "-sn", "-dn"]);
        let codec = plan.audio.map(|c| c.encoder()).unwrap_or("aac");
        a.extend(["-c:a", codec]).push("-b:a").push(format!("{}k", plan.audio_kbps));
        if codec == "aac" {
            a.extend(["-movflags", "+faststart"]);
        }
        a.extend(["-progress", "pipe:1", "-nostats"]);
        a.push(args.output.as_os_str());
        return a.0;
    }

    // One video track (cover art does not count), the first audio track if
    // there is one, nothing else.
    a.extend(["-map", "0:V:0"]);
    let filters = video_filters(args);
    if !filters.is_empty() {
        a.push("-vf").push(filters.join(","));
    }
    a.push("-c:v").push(args.encoder.name);
    a.extend(format_flags(args.encoder, args.info));
    a.extend(rate_flags(args.encoder, args.video_kbps));
    a.extend(effort_flags(args.encoder, args.options));

    match (args.pass, args.encoder.name) {
        (Pass::Single, _) => {}
        (Pass::First, "libx264") => {
            a.extend(["-pass", "1"]);
        }
        (Pass::Second, "libx264") => {
            a.extend(["-pass", "2"]);
        }
        (Pass::First, _) => {
            a.extend(["-x265-params", "pass=1"]);
        }
        (Pass::Second, _) => {
            a.extend(["-x265-params", "pass=2"]);
        }
    }

    if args.pass == Pass::First {
        a.extend(["-an", "-sn", "-dn", "-progress", "pipe:1", "-nostats", "-f", "null", "-"]);
        return a.0;
    }

    match plan.audio {
        Some(codec) => {
            a.extend(["-map", "0:a:0?", "-c:a", codec.encoder()]);
            a.push("-b:a").push(format!("{}k", plan.audio_kbps));
            // Opus tops out at 255 kbps per channel pair; surround needs a layout hint.
            if codec == crate::plan::AudioCodec::Opus && args.info.audio_channels.unwrap_or(2) > 2 {
                a.extend(["-ac", "2"]);
            }
        }
        None => {
            a.push("-an");
        }
    }
    a.extend(["-sn", "-dn"]);

    if args.options.container == Container::Mp4 {
        // Apple players only recognize HEVC in MP4 under this tag.
        if args.encoder.codec == Codec::Hevc {
            a.extend(["-tag:v", "hvc1"]);
        }
        if args.options.fast_finalize {
            a.extend(["-movflags", "+frag_keyframe+empty_moov+default_base_moof"]);
        } else {
            a.extend(["-movflags", "+faststart"]);
        }
    }

    a.extend(["-progress", "pipe:1", "-nostats"]);
    a.push(args.output.as_os_str());
    a.0
}

/// A one-line rendering for the log. Not safe to paste into a shell.
pub fn display(binary: &Path, args: &[OsString]) -> String {
    let mut text = binary.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    for arg in args {
        let arg = arg.to_string_lossy();
        text.push(' ');
        if arg.contains(' ') {
            text.push('"');
            text.push_str(&arg);
            text.push('"');
        } else {
            text.push_str(&arg);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoders::find;
    use crate::job::{AudioChoice, Trim};
    use crate::plan::plan;

    fn source() -> MediaInfo {
        MediaInfo {
            duration: 60.0,
            has_video: true,
            has_audio: true,
            width: 1920,
            height: 1080,
            display_width: 1920,
            display_height: 1080,
            fps: Some(60.0),
            pixel_format: Some("yuv420p".into()),
            audio_channels: Some(2),
            ..MediaInfo::default()
        }
    }

    fn render(info: &MediaInfo, trim: Option<Trim>, options: &EncodeOptions, encoder: &str, pass: Pass) -> String {
        let p = plan(info, trim, options).unwrap();
        let args = EncodeArgs {
            input: Path::new("/in/clip.mov"),
            output: Path::new("/out/clip.mp4"),
            info,
            plan: &p,
            options,
            encoder: find(encoder).unwrap(),
            device: Some("/dev/dri/renderD129"),
            video_kbps: p.video_kbps,
            hardware_decode: options.hardware_decode,
            pass,
        };
        build(&args).iter().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn x264_with_trim_scale_and_fps_cap() {
        let options = EncodeOptions { target_mb: 8.0, max_fps: Some(30.0), ..Default::default() };
        let cmd = render(&source(), Some(Trim { start: 5.0, end: 35.0 }), &options, "libx264", Pass::Single);
        assert!(cmd.starts_with("-y -ss 5.000 -i /in/clip.mov -t 30.000 -map 0:V:0"), "{cmd}");
        assert!(cmd.contains("-vf fps=fps=30 "), "{cmd}");
        assert!(
            cmd.contains("-c:v libx264 -pix_fmt yuv420p -profile:v high -b:v 2057k -maxrate 2468k -bufsize 4114k"),
            "{cmd}"
        );
        assert!(cmd.contains("-preset faster -tune film"), "{cmd}");
        assert!(cmd.contains("-map 0:a:0? -c:a aac -b:a 128k"), "{cmd}");
        assert!(cmd.ends_with("-movflags +faststart -progress pipe:1 -nostats /out/clip.mp4"), "{cmd}");
    }

    #[test]
    fn untrimmed_jobs_do_not_seek() {
        let cmd = render(&source(), None, &EncodeOptions::default(), "libx264", Pass::Single);
        assert!(!cmd.contains("-ss") && !cmd.contains(" -t "), "{cmd}");
    }

    #[test]
    fn hevc_in_mp4_gets_the_apple_tag() {
        let options = EncodeOptions { codec: Codec::Hevc, ..Default::default() };
        let cmd = render(&source(), None, &options, "hevc_nvenc", Pass::Single);
        assert!(cmd.contains("-c:v hevc_nvenc -pix_fmt yuv420p -profile:v main"), "{cmd}");
        assert!(cmd.contains("-preset p4 -tune hq"), "{cmd}");
        assert!(cmd.contains("-tag:v hvc1"), "{cmd}");
    }

    #[test]
    fn svt_av1_gets_no_peak_rate() {
        let options = EncodeOptions {
            codec: Codec::Av1,
            container: Container::Mkv,
            audio: AudioChoice::Opus,
            ..Default::default()
        };
        let cmd = render(&source(), None, &options, "libsvtav1", Pass::Single);
        assert!(!cmd.contains("-maxrate"), "{cmd}");
        assert!(cmd.contains("-preset 8"), "{cmd}");
        assert!(cmd.contains("-c:a libopus"), "{cmd}");
        assert!(!cmd.contains("movflags"), "{cmd}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn vaapi_and_qsv_upload_frames_to_the_chosen_device() {
        let options = EncodeOptions { target_mb: 3.0, ..Default::default() };
        let cmd = render(&source(), None, &options, "h264_vaapi", Pass::Single);
        assert!(cmd.starts_with("-y -init_hw_device vaapi=va:/dev/dri/renderD129 -filter_hw_device va -i"), "{cmd}");
        assert!(
            cmd.contains("-vf scale=1280:720:flags=bicubic,format=nv12|vaapi,hwupload -c:v h264_vaapi -b:v"),
            "{cmd}"
        );

        let ten_bit = MediaInfo { pixel_format: Some("yuv420p10le".into()), ..source() };
        let options = EncodeOptions { codec: Codec::Hevc, ..Default::default() };
        let cmd = render(&ten_bit, None, &options, "hevc_qsv", Pass::Single);
        assert!(cmd.contains("-init_hw_device qsv=hw@va -filter_hw_device hw"), "{cmd}");
        assert!(cmd.contains("format=p010le,hwupload=extra_hw_frames=64 -c:v hevc_qsv -profile:v main10"), "{cmd}");
    }

    #[test]
    fn two_pass_commands() {
        let options = EncodeOptions { effort: Effort::Max, ..Default::default() };
        assert!(uses_two_passes(find("libx264").unwrap(), &options));
        assert!(!uses_two_passes(find("h264_nvenc").unwrap(), &options));
        let first = render(&source(), None, &options, "libx264", Pass::First);
        assert!(first.ends_with("-pass 1 -an -sn -dn -progress pipe:1 -nostats -f null -"), "{first}");
        let second = render(&source(), None, &options, "libx265", Pass::Second);
        assert!(second.contains("-preset slower -x265-params pass=2"), "{second}");
    }

    #[test]
    fn muted_and_audio_only() {
        let options = EncodeOptions { audio: AudioChoice::None, ..Default::default() };
        let cmd = render(&source(), None, &options, "libx264", Pass::Single);
        assert!(cmd.contains(" -an "), "{cmd}");

        let options = EncodeOptions { audio_only: true, audio: AudioChoice::Aac, ..Default::default() };
        let cmd = render(&source(), None, &options, "libx264", Pass::Single);
        assert!(cmd.contains("-map 0:a:0 -vn -sn -dn -c:a aac -b:a 128k"), "{cmd}");
        assert!(!cmd.contains("-c:v"), "{cmd}");
    }
}
