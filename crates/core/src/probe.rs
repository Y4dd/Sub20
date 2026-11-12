//! Reading a file's facts with FFprobe.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::ffmpeg::{self, Tools};

/// What the planner and the UI need to know about a source file.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    /// Length in seconds.
    pub duration: f64,
    /// File size in bytes.
    pub size_bytes: u64,
    pub has_video: bool,
    pub has_audio: bool,
    /// Coded size, before rotation metadata is applied.
    pub width: u32,
    pub height: u32,
    /// Size as a player shows it (rotation applied).
    pub display_width: u32,
    pub display_height: u32,
    /// Clockwise rotation a player applies: 0, 90, 180 or 270.
    pub rotation: u32,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub pixel_format: Option<String>,
    pub video_kbps: Option<f64>,
    pub audio_codec: Option<String>,
    pub audio_kbps: Option<f64>,
    pub audio_channels: Option<u32>,
}

impl MediaInfo {
    /// True for 10-bit and deeper sources.
    pub fn is_high_bit_depth(&self) -> bool {
        self.pixel_format.as_deref().map(|p| ["10", "12", "14", "16"].iter().any(|d| p.contains(d))).unwrap_or(false)
    }
}

/// Runs FFprobe on `path`.
pub async fn probe(tools: &Tools, path: &Path) -> Result<MediaInfo> {
    let mut cmd = tools.ffprobe();
    cmd.args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"]).arg(path);
    let out = ffmpeg::run(&mut cmd, Duration::from_secs(30)).await?;
    if !out.ok {
        return Err(Error::Unreadable(path.to_path_buf(), ffmpeg::tail(&out.stderr, 2)));
    }
    let mut info = parse(&out.stdout).map_err(|e| Error::Unreadable(path.to_path_buf(), e))?;
    if info.size_bytes == 0 {
        info.size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    Ok(info)
}

fn num(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

/// `30000/1001` → 29.97. Rejects `0/0` and absurd values.
pub fn parse_rate(text: &str) -> Option<f64> {
    let fps = match text.split_once('/') {
        Some((n, d)) => {
            let (n, d): (f64, f64) = (n.trim().parse().ok()?, d.trim().parse().ok()?);
            if d == 0.0 {
                return None;
            }
            n / d
        }
        None => text.trim().parse().ok()?,
    };
    (fps.is_finite() && fps > 0.0 && fps <= 1000.0).then_some(fps)
}

fn rotation_of(stream: &Value) -> u32 {
    let mut degrees: Option<f64> = None;
    if let Some(list) = stream.get("side_data_list").and_then(Value::as_array) {
        for item in list {
            if let Some(r) = item.get("rotation").and_then(num) {
                // The display matrix stores counter-clockwise degrees.
                degrees = Some(-r);
            }
        }
    }
    if degrees.is_none() {
        degrees = stream.pointer("/tags/rotate").and_then(num);
    }
    let d = degrees.unwrap_or(0.0).rem_euclid(360.0);
    // Snap to the nearest quarter turn.
    (((d / 90.0).round() as u32) % 4) * 90
}

/// Parses FFprobe's JSON. Kept separate from the process call so it can be
/// tested with canned output.
pub fn parse(json: &str) -> std::result::Result<MediaInfo, String> {
    let root: Value = serde_json::from_str(json).map_err(|e| format!("FFprobe output was not JSON ({e})"))?;
    let empty = Vec::new();
    let streams = root.get("streams").and_then(Value::as_array).unwrap_or(&empty);
    let format = root.get("format").cloned().unwrap_or(Value::Null);

    let is_kind = |s: &Value, kind: &str| s.get("codec_type").and_then(Value::as_str) == Some(kind);
    let is_cover = |s: &Value| s.pointer("/disposition/attached_pic").and_then(Value::as_i64) == Some(1);

    let video = streams.iter().find(|s| is_kind(s, "video") && !is_cover(s));
    let audio = streams.iter().find(|s| is_kind(s, "audio"));

    let mut info = MediaInfo {
        has_video: video.is_some(),
        has_audio: audio.is_some(),
        size_bytes: format.get("size").and_then(num).map(|v| v as u64).unwrap_or(0),
        ..MediaInfo::default()
    };

    info.duration = format
        .get("duration")
        .and_then(num)
        .or_else(|| video.and_then(|v| v.get("duration")).and_then(num))
        .or_else(|| audio.and_then(|a| a.get("duration")).and_then(num))
        .filter(|d| *d > 0.0)
        .unwrap_or(0.0);

    if let Some(a) = audio {
        info.audio_codec = a.get("codec_name").and_then(Value::as_str).map(str::to_string);
        info.audio_kbps = a.get("bit_rate").and_then(num).map(|b| b / 1000.0).filter(|b| *b > 0.0);
        info.audio_channels = a.get("channels").and_then(num).map(|c| c as u32);
    }

    if let Some(v) = video {
        info.width = v.get("width").and_then(num).unwrap_or(0.0) as u32;
        info.height = v.get("height").and_then(num).unwrap_or(0.0) as u32;
        info.rotation = rotation_of(v);
        let turned = info.rotation == 90 || info.rotation == 270;
        info.display_width = if turned { info.height } else { info.width };
        info.display_height = if turned { info.width } else { info.height };
        info.video_codec = v.get("codec_name").and_then(Value::as_str).map(str::to_string);
        info.pixel_format = v.get("pix_fmt").and_then(Value::as_str).map(str::to_string);
        info.fps = ["avg_frame_rate", "r_frame_rate"]
            .iter()
            .filter_map(|k| v.get(*k).and_then(Value::as_str))
            .find_map(parse_rate);
        info.video_kbps = v.get("bit_rate").and_then(num).map(|b| b / 1000.0).filter(|b| *b > 0.0).or_else(|| {
            // Matroska and friends only report a container bitrate.
            let total = format.get("bit_rate").and_then(num)? / 1000.0;
            let rest = total - info.audio_kbps.unwrap_or(0.0);
            (rest > 0.0).then_some(rest)
        });
    }

    if !info.has_video && !info.has_audio {
        return Err("it has no video or audio streams".into());
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates() {
        assert_eq!(parse_rate("30/1"), Some(30.0));
        assert!((parse_rate("30000/1001").unwrap() - 29.97).abs() < 0.01);
        assert_eq!(parse_rate("0/0"), None);
        assert_eq!(parse_rate("90000/1"), None);
    }

    #[test]
    fn phone_video_with_rotation() {
        let json = r#"{
          "streams": [
            {"codec_type":"video","codec_name":"hevc","width":1920,"height":1080,"pix_fmt":"yuv420p10le",
             "avg_frame_rate":"60/1","r_frame_rate":"60/1","bit_rate":"12000000",
             "side_data_list":[{"side_data_type":"Display Matrix","rotation":-90}]},
            {"codec_type":"audio","codec_name":"aac","bit_rate":"192000","channels":2}
          ],
          "format": {"duration":"12.5","size":"19000000","bit_rate":"12200000"}
        }"#;
        let info = parse(json).unwrap();
        assert_eq!(info.rotation, 90);
        assert_eq!((info.display_width, info.display_height), (1080, 1920));
        assert_eq!(info.fps, Some(60.0));
        assert_eq!(info.video_kbps, Some(12000.0));
        assert_eq!(info.audio_kbps, Some(192.0));
        assert!(info.is_high_bit_depth());
        assert_eq!(info.duration, 12.5);
    }

    #[test]
    fn mkv_without_stream_bitrate_and_cover_art() {
        let json = r#"{
          "streams": [
            {"codec_type":"video","codec_name":"mjpeg","width":600,"height":600,"disposition":{"attached_pic":1}},
            {"codec_type":"video","codec_name":"h264","width":1280,"height":720,"pix_fmt":"yuv420p","avg_frame_rate":"0/0","r_frame_rate":"24/1"},
            {"codec_type":"audio","codec_name":"opus","channels":2}
          ],
          "format": {"duration":"100.0","size":"5000000","bit_rate":"400000"}
        }"#;
        let info = parse(json).unwrap();
        assert_eq!(info.video_codec.as_deref(), Some("h264"));
        assert_eq!((info.width, info.height), (1280, 720));
        assert_eq!(info.fps, Some(24.0));
        assert_eq!(info.video_kbps, Some(400.0));
        assert_eq!(info.rotation, 0);
    }

    #[test]
    fn rejects_empty() {
        assert!(parse(r#"{"streams":[],"format":{}}"#).is_err());
        assert!(parse("not json").is_err());
    }
}
