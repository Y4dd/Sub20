//! Getting FFmpeg for the lean download.
//!
//! The lean build finds FFmpeg on the system. When there is none, the window
//! offers to download the same FFmpeg the full download ships: the archives
//! pinned in `scripts/ffmpeg.json`, checked against the same SHA-256. They
//! are fetched from Sub20's own release, where the release workflow puts
//! them unchanged, because the builders' links don't last (BtbN deletes its
//! daily builds after a few weeks).
//!
//! The shell does the HTTP. This module says what to fetch and from where,
//! checks what arrived, unpacks it and installs it as `sub20-ffmpeg` and
//! `sub20-ffprobe` in a folder the shell adds to the bundled folders.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{Error, IoContext, Result};
use crate::ffmpeg;

const PINS: &str = include_str!("../../../scripts/ffmpeg.json");
const LICENCE: &str = include_str!("../../../licenses/FFmpeg-GPL-3.0.txt");

/// Where the release workflow uploads the archives, per app version.
const RELEASES: &str = "https://github.com/Y4dd/Sub20/releases/download";

/// What this system can download.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    /// FFmpeg's version, "9.0.2".
    pub version: String,
    /// Who built it, with a link.
    pub builder: String,
    /// Where FFmpeg's source for this build is.
    pub source: String,
    pub files: Vec<Download>,
}

/// One archive to fetch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    /// The name on Sub20's release page.
    pub name: String,
    /// Where the builder published it.
    pub upstream: String,
    pub sha256: String,
}

impl Offer {
    /// The address of `file` on the release of Sub20 `app_version`.
    pub fn url(&self, app_version: &str, file: &Download) -> String {
        format!("{RELEASES}/v{app_version}/{}", file.name)
    }
}

/// The Rust target this program was built for, when FFmpeg is pinned for
/// it.
fn this_target() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else {
        None
    }
}

/// The download for this system, or `None` where no FFmpeg is pinned.
pub fn offer() -> Option<Offer> {
    offer_for(PINS, this_target()?)
}

fn offer_for(pins: &str, target: &str) -> Option<Offer> {
    let pins: serde_json::Value = serde_json::from_str(pins).ok()?;
    let version = pins["version"].as_str()?.to_string();
    let pin = &pins["targets"][target];
    let text = |key: &str| pin[key].as_str().map(str::to_string);
    let file = |entry: &serde_json::Value, tool: Option<&str>| -> Option<Download> {
        let upstream = entry["url"].as_str()?.to_string();
        Some(Download {
            name: asset_name(&version, target, tool, &upstream),
            sha256: entry["sha256"].as_str()?.to_string(),
            upstream,
        })
    };
    let files = if pin["archive"].is_object() {
        vec![file(&pin["archive"], None)?]
    } else {
        vec![file(&pin["ffmpeg"], Some("ffmpeg"))?, file(&pin["ffprobe"], Some("ffprobe"))?]
    };
    Some(Offer { builder: text("builder")?, source: text("source")?, version, files })
}

/// The name an archive is uploaded under. `scripts/fetch-ffmpeg.mjs` gives
/// it the same name; change both together.
fn asset_name(version: &str, target: &str, tool: Option<&str>, upstream: &str) -> String {
    let ext = if upstream.ends_with(".tar.xz") { ".tar.xz" } else { ".zip" };
    match tool {
        Some(tool) => format!("FFmpeg-{version}-{target}-{tool}{ext}"),
        None => format!("FFmpeg-{version}-{target}{ext}"),
    }
}

/// The file's SHA-256 as lowercase hex.
pub fn sha256(file: &Path) -> Result<String> {
    let mut reader = std::fs::File::open(file).ctx(format!("could not open {}", file.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = reader.read(&mut buffer).ctx(format!("could not read {}", file.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Refuses a download whose SHA-256 isn't the pinned one, and deletes it.
pub fn check(file: &Path, expected: &str) -> Result<()> {
    if sha256(file)?.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    let _ = std::fs::remove_file(file);
    Err(Error::Invalid(
        "The FFmpeg download didn't match its pinned fingerprint, so it was thrown away. Try again; if it happens again, install FFmpeg yourself.".into(),
    ))
}

/// Unpacks a zip or tar.xz archive with the system's bsdtar. On Windows
/// that is the `tar.exe` Windows ships: a GNU tar earlier on `PATH` (Git's)
/// can't read zip.
pub async fn unpack(archive: &Path, into: &Path) -> Result<()> {
    std::fs::create_dir_all(into).ctx(format!("could not create {}", into.display()))?;
    let tar = if cfg!(windows) {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("System32").join("tar.exe")
    } else {
        PathBuf::from("tar")
    };
    let mut cmd = ffmpeg::quiet(&tar);
    cmd.arg("-xf").arg(archive).arg("-C").arg(into);
    let out = ffmpeg::run(&mut cmd, Duration::from_secs(300))
        .await
        .map_err(|e| Error::Invalid(format!("Could not unpack the FFmpeg download: {e}")))?;
    if !out.ok {
        return Err(Error::Invalid(format!("Could not unpack the FFmpeg download: {}", ffmpeg::tail(&out.stderr, 2))));
    }
    Ok(())
}

/// Finds `ffmpeg` and `ffprobe` anywhere under `unpacked` and installs them
/// into `dest` as `sub20-ffmpeg` and `sub20-ffprobe`, with FFmpeg's licence
/// and where it came from. Whatever `dest` held is replaced. Returns the
/// notice file.
pub fn install(unpacked: &Path, dest: &Path, offer: &Offer) -> Result<PathBuf> {
    let suffix = std::env::consts::EXE_SUFFIX;
    let staging = dest.with_extension("new");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).ctx(format!("could not create {}", staging.display()))?;

    for tool in ["ffmpeg", "ffprobe"] {
        let name = format!("{tool}{suffix}");
        let found =
            find(unpacked, &name).ok_or_else(|| Error::Invalid(format!("The FFmpeg download has no {name} in it.")))?;
        let to = staging.join(format!("sub20-{name}"));
        std::fs::copy(&found, &to).ctx(format!("could not copy {name}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o755))
                .ctx(format!("could not make {name} runnable"))?;
        }
    }

    let mut notice = vec![
        format!("Sub20 downloaded FFmpeg {} (the ffmpeg and ffprobe programs) into", offer.version),
        "this folder. FFmpeg is free software, licensed under the GNU General".into(),
        "Public License version 3 (LICENSE-GPL-3.0.txt beside this file). Sub20".into(),
        "runs them as separate programs; Sub20 itself is MIT licensed.".into(),
        String::new(),
        format!("Built by: {}", offer.builder),
        format!("FFmpeg source: {}", offer.source),
        String::new(),
    ];
    for file in &offer.files {
        notice.push(format!("Built from: {}", file.upstream));
        notice.push(format!("SHA-256: {}", file.sha256));
    }
    notice.push(String::new());
    std::fs::write(staging.join("FFMPEG.txt"), notice.join("\n")).ctx("could not write FFmpeg's notice")?;
    std::fs::write(staging.join("LICENSE-GPL-3.0.txt"), LICENCE).ctx("could not write FFmpeg's licence")?;

    if dest.exists() {
        std::fs::remove_dir_all(dest).ctx(format!("could not replace {}", dest.display()))?;
    }
    std::fs::rename(&staging, dest).ctx(format!("could not move FFmpeg into {}", dest.display()))?;
    Ok(dest.join("FFMPEG.txt"))
}

fn find(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in &entries {
        let path = entry.path();
        if path.is_file() && entry.file_name() == name {
            return Some(path);
        }
    }
    entries.iter().filter(|entry| entry.path().is_dir()).find_map(|entry| find(&entry.path(), name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pinned_system_gets_an_offer_named_for_the_release_page() {
        let windows = offer_for(PINS, "x86_64-pc-windows-msvc").expect("windows is pinned");
        assert_eq!(windows.files.len(), 1);
        assert!(windows.files[0].name.starts_with(&format!("FFmpeg-{}-x86_64-pc-windows-msvc", windows.version)));
        assert!(windows.files[0].name.ends_with(".zip"));
        assert_eq!(windows.files[0].sha256.len(), 64);

        let linux = offer_for(PINS, "x86_64-unknown-linux-gnu").expect("linux is pinned");
        assert!(linux.files[0].name.ends_with("-x86_64-unknown-linux-gnu.tar.xz"));

        for mac in ["aarch64-apple-darwin", "x86_64-apple-darwin"] {
            let offer = offer_for(PINS, mac).expect("mac is pinned");
            let names: Vec<_> = offer.files.iter().map(|f| f.name.as_str()).collect();
            assert_eq!(
                names,
                [
                    format!("FFmpeg-{}-{mac}-ffmpeg.zip", offer.version),
                    format!("FFmpeg-{}-{mac}-ffprobe.zip", offer.version)
                ]
            );
        }

        assert_eq!(offer_for(PINS, "riscv64gc-unknown-linux-gnu"), None);
        assert_eq!(
            windows.url("1.2.3", &windows.files[0]),
            format!("https://github.com/Y4dd/Sub20/releases/download/v1.2.3/{}", windows.files[0].name)
        );
        if this_target().is_some() {
            assert!(offer().is_some());
        }
    }

    #[test]
    fn a_download_must_match_its_pin() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("archive");
        std::fs::write(&file, b"abc").unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(sha256(&file).unwrap(), abc);
        check(&file, &abc.to_uppercase()).unwrap();

        let error = check(&file, &"0".repeat(64)).unwrap_err().to_string();
        assert!(error.contains("didn't match"), "{error}");
        assert!(!file.exists(), "a download that doesn't match is deleted");
    }

    #[tokio::test]
    async fn an_archive_is_unpacked_and_installed_under_sub20_names() {
        let dir = tempfile::tempdir().unwrap();
        let suffix = std::env::consts::EXE_SUFFIX;
        let tree = dir.path().join("tree").join("ffmpeg-9.0.2").join("bin");
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::write(tree.join(format!("ffmpeg{suffix}")), b"ffmpeg").unwrap();
        std::fs::write(tree.join(format!("ffprobe{suffix}")), b"ffprobe").unwrap();

        // A plain tar, which bsdtar and GNU tar both make and read.
        let archive = dir.path().join("ffmpeg.tar");
        let tar = if cfg!(windows) {
            PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32").join("tar.exe")
        } else {
            PathBuf::from("tar")
        };
        let made = std::process::Command::new(tar)
            .arg("-cf")
            .arg(&archive)
            .arg("-C")
            .arg(dir.path().join("tree"))
            .arg("ffmpeg-9.0.2")
            .status()
            .unwrap();
        assert!(made.success());

        let unpacked = dir.path().join("unpacked");
        unpack(&archive, &unpacked).await.unwrap();

        let dest = dir.path().join("ffmpeg");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("old"), b"replaced").unwrap();
        let offer = offer_for(PINS, "x86_64-pc-windows-msvc").unwrap();
        let notice = install(&unpacked, &dest, &offer).unwrap();

        assert_eq!(std::fs::read(dest.join(format!("sub20-ffmpeg{suffix}"))).unwrap(), b"ffmpeg");
        assert_eq!(std::fs::read(dest.join(format!("sub20-ffprobe{suffix}"))).unwrap(), b"ffprobe");
        assert!(!dest.join("old").exists(), "the folder is replaced, not added to");
        assert!(std::fs::read_to_string(&notice).unwrap().contains(&offer.files[0].sha256));
        assert!(dest.join("LICENSE-GPL-3.0.txt").is_file());
        assert!(!dest.with_extension("new").exists());
        let found = ffmpeg::Tools::locate(None, std::slice::from_ref(&dest)).unwrap();
        assert_eq!(found.ffmpeg, dest.join(format!("sub20-ffmpeg{suffix}")));
    }

    #[test]
    fn an_archive_without_ffprobe_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let suffix = std::env::consts::EXE_SUFFIX;
        std::fs::write(dir.path().join(format!("ffmpeg{suffix}")), b"ffmpeg").unwrap();
        let offer = offer_for(PINS, "x86_64-pc-windows-msvc").unwrap();
        let error = install(dir.path(), &dir.path().join("out"), &offer).unwrap_err().to_string();
        assert!(error.contains(&format!("no ffprobe{suffix}")), "{error}");
    }
}
