//! The catalog of encoders the app can drive, and how they relate.

use serde::{Deserialize, Serialize};

/// The video format written into the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    H264,
    Hevc,
    Av1,
}

impl Codec {
    pub const ALL: [Codec; 3] = [Codec::H264, Codec::Hevc, Codec::Av1];

    pub fn id(self) -> &'static str {
        match self {
            Codec::H264 => "h264",
            Codec::Hevc => "hevc",
            Codec::Av1 => "av1",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::Hevc => "HEVC",
            Codec::Av1 => "AV1",
        }
    }

    pub fn from_id(id: &str) -> Option<Codec> {
        match id {
            "h264" => Some(Codec::H264),
            "hevc" | "h265" => Some(Codec::Hevc),
            "av1" => Some(Codec::Av1),
            _ => None,
        }
    }

    /// The software encoder used when no hardware encoder works.
    pub fn cpu_encoder(self) -> &'static str {
        match self {
            Codec::H264 => "libx264",
            Codec::Hevc => "libx265",
            Codec::Av1 => "libsvtav1",
        }
    }
}

/// Which piece of hardware (or none) does the encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Nvenc,
    Qsv,
    Amf,
    /// Windows Media Foundation.
    Mf,
    VideoToolbox,
    Vaapi,
    Cpu,
}

impl Family {
    pub fn label(self) -> &'static str {
        match self {
            Family::Nvenc => "NVIDIA NVENC",
            Family::Qsv => "Intel Quick Sync",
            Family::Amf => "AMD AMF",
            Family::Mf => "Windows Media Foundation",
            Family::VideoToolbox => "Apple VideoToolbox",
            Family::Vaapi => "VAAPI",
            Family::Cpu => "CPU",
        }
    }

    pub fn is_hardware(self) -> bool {
        self != Family::Cpu
    }

    /// Whether this family can exist on the operating system we run on.
    pub fn applies_here(self) -> bool {
        match self {
            Family::Cpu => true,
            Family::Nvenc | Family::Qsv => cfg!(any(windows, target_os = "linux")),
            Family::Amf | Family::Mf => cfg!(windows),
            Family::VideoToolbox => cfg!(target_os = "macos"),
            Family::Vaapi => cfg!(target_os = "linux"),
        }
    }
}

/// One FFmpeg encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoder {
    /// FFmpeg's name, e.g. `hevc_nvenc`.
    pub name: &'static str,
    pub codec: Codec,
    pub family: Family,
}

const fn enc(name: &'static str, codec: Codec, family: Family) -> Encoder {
    Encoder { name, codec, family }
}

/// Every encoder the app knows, in order of preference within each codec:
/// hardware first, CPU last.
pub const CATALOG: &[Encoder] = &[
    enc("h264_nvenc", Codec::H264, Family::Nvenc),
    enc("h264_qsv", Codec::H264, Family::Qsv),
    enc("h264_amf", Codec::H264, Family::Amf),
    enc("h264_mf", Codec::H264, Family::Mf),
    enc("h264_videotoolbox", Codec::H264, Family::VideoToolbox),
    enc("h264_vaapi", Codec::H264, Family::Vaapi),
    enc("libx264", Codec::H264, Family::Cpu),
    enc("hevc_nvenc", Codec::Hevc, Family::Nvenc),
    enc("hevc_qsv", Codec::Hevc, Family::Qsv),
    enc("hevc_amf", Codec::Hevc, Family::Amf),
    enc("hevc_mf", Codec::Hevc, Family::Mf),
    enc("hevc_videotoolbox", Codec::Hevc, Family::VideoToolbox),
    enc("hevc_vaapi", Codec::Hevc, Family::Vaapi),
    enc("libx265", Codec::Hevc, Family::Cpu),
    enc("av1_nvenc", Codec::Av1, Family::Nvenc),
    enc("av1_qsv", Codec::Av1, Family::Qsv),
    enc("av1_amf", Codec::Av1, Family::Amf),
    enc("av1_mf", Codec::Av1, Family::Mf),
    enc("av1_vaapi", Codec::Av1, Family::Vaapi),
    enc("libsvtav1", Codec::Av1, Family::Cpu),
    // Kept so old profiles still run. Never picked automatically: it is far
    // slower than SVT-AV1.
    enc("libaom-av1", Codec::Av1, Family::Cpu),
];

/// Encoders that are never chosen unless the user names them.
pub const NEVER_AUTOMATIC: &[&str] = &["libaom-av1"];

pub fn find(name: &str) -> Option<Encoder> {
    CATALOG.iter().copied().find(|e| e.name == name)
}

/// Encoders for one codec, best first.
pub fn for_codec(codec: Codec) -> impl Iterator<Item = Encoder> {
    CATALOG.iter().copied().filter(move |e| e.codec == codec)
}

/// A human name such as `HEVC, NVIDIA NVENC`.
pub fn describe(name: &str) -> String {
    match find(name) {
        Some(e) if e.name == "libsvtav1" => "AV1, CPU (SVT-AV1)".into(),
        Some(e) if e.name == "libaom-av1" => "AV1, CPU (libaom)".into(),
        Some(e) => format!("{}, {}", e.codec.label(), e.family.label()),
        None => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        assert_eq!(CATALOG.len(), 21);
        for codec in Codec::ALL {
            let list: Vec<_> = for_codec(codec).collect();
            // Hardware comes before CPU within a codec.
            let first_cpu = list.iter().position(|e| e.family == Family::Cpu).unwrap();
            assert!(list[..first_cpu].iter().all(|e| e.family.is_hardware()));
            assert_eq!(list[first_cpu].name, codec.cpu_encoder());
        }
        let mut names: Vec<_> = CATALOG.iter().map(|e| e.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CATALOG.len());
    }
}
