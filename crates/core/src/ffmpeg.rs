//! Finding FFmpeg/FFprobe and starting them without a console window.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::{Child, Command};

use crate::error::{Error, Result};

/// Paths to the two binaries the app drives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn pair_in(dir: &Path) -> Option<Tools> {
    named_pair_in(dir, "")
}

fn named_pair_in(dir: &Path, prefix: &str) -> Option<Tools> {
    let ffmpeg = dir.join(exe(&format!("{prefix}ffmpeg")));
    let ffprobe = dir.join(exe(&format!("{prefix}ffprobe")));
    (ffmpeg.is_file() && ffprobe.is_file()).then_some(Tools { ffmpeg, ffprobe })
}

/// The release build ships FFmpeg as `sub20-ffmpeg` and `sub20-ffprobe`,
/// because a Linux package puts them in `/usr/bin` beside the executable,
/// where plain names would clash with the system's own FFmpeg package.
/// Plain names are accepted too, for a hand-made bundle.
fn bundled_pair_in(dir: &Path) -> Option<Tools> {
    named_pair_in(dir, "sub20-").or_else(|| pair_in(dir))
}

impl Tools {
    /// Looks for FFmpeg in this order:
    ///
    /// 1. `custom`: a folder (or the ffmpeg binary itself) the user chose,
    /// 2. `bundled`: folders shipped with the app (next to the executable,
    ///    the resource folder),
    /// 3. every folder on `PATH`,
    /// 4. a few well-known install locations.
    pub fn locate(custom: Option<&Path>, bundled: &[PathBuf]) -> Result<Tools> {
        if let Some(custom) = custom.filter(|p| !p.as_os_str().is_empty()) {
            let dir = if custom.is_file() { custom.parent().unwrap_or(custom) } else { custom };
            if let Some(found) = pair_in(dir).or_else(|| pair_in(&dir.join("bin"))) {
                return Ok(found);
            }
        }
        for dir in bundled {
            if let Some(found) = bundled_pair_in(dir) {
                return Ok(found);
            }
        }
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                if let Some(found) = pair_in(&dir) {
                    return Ok(found);
                }
            }
        }
        let mut usual: Vec<PathBuf> = Vec::new();
        if cfg!(target_os = "macos") {
            usual.extend(["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"].map(PathBuf::from));
        } else if cfg!(windows) {
            usual.extend(
                ["C:\\ffmpeg\\bin", "C:\\Program Files\\ffmpeg\\bin", "C:\\ProgramData\\chocolatey\\bin"]
                    .map(PathBuf::from),
            );
            if let Some(home) = std::env::var_os("USERPROFILE") {
                let home = PathBuf::from(home);
                usual.push(home.join("scoop").join("shims"));
                usual.push(home.join("AppData\\Local\\Microsoft\\WinGet\\Links"));
            }
        } else {
            usual.extend(["/usr/bin", "/usr/local/bin", "/snap/bin"].map(PathBuf::from));
        }
        usual.iter().find_map(|dir| pair_in(dir)).ok_or(Error::FfmpegMissing)
    }

    /// A ready-to-fill `ffmpeg` command: no banner, no stdin, no console.
    pub fn ffmpeg(&self) -> Command {
        let mut cmd = quiet(&self.ffmpeg);
        cmd.arg("-hide_banner").arg("-nostdin");
        cmd
    }

    pub fn ffprobe(&self) -> Command {
        quiet(&self.ffprobe)
    }

    /// First line of `ffmpeg -version`, e.g. `ffmpeg version 7.1`.
    pub async fn version(&self) -> Option<String> {
        let out = run(self.ffmpeg().arg("-version"), Duration::from_secs(10)).await.ok()?;
        out.stdout.lines().next().map(|line| line.split(" Copyright").next().unwrap_or(line).trim().to_string())
    }
}

/// Stops a running FFmpeg and anything it started.
///
/// On Windows `ffmpeg.exe` is often a launcher: Chocolatey's ShimGen shims
/// start the real FFmpeg as a child, and killing the shim leaves that child
/// running, still writing its output. So the whole tree goes first, while the
/// shim is alive and still the real FFmpeg's parent.
pub async fn kill_tree(child: &mut Child) {
    end_tree(child.id()).await;
    let _ = child.kill().await;
}

#[cfg(windows)]
async fn end_tree(pid: Option<u32>) {
    let Some(pid) = pid else { return };
    // By full path, so a `taskkill` earlier on PATH is never the one run.
    let taskkill = std::env::var_os("SystemRoot")
        .map(|root| PathBuf::from(root).join("System32").join("taskkill.exe"))
        .unwrap_or_else(|| PathBuf::from("taskkill.exe"));
    let mut cmd = quiet(taskkill);
    cmd.args(["/T", "/F", "/PID", &pid.to_string()]).stdout(Stdio::null()).stderr(Stdio::null());
    let _ = tokio::time::timeout(Duration::from_secs(5), cmd.status()).await;
}

/// On macOS and Linux, package managers link FFmpeg directly or `exec` it
/// from a wrapper, so the process killed is FFmpeg itself.
#[cfg(not(windows))]
async fn end_tree(_pid: Option<u32>) {}

pub(crate) fn quiet(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: do not flash a console for every FFmpeg call.
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Captured result of a short helper run.
pub struct Output {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a command to completion with a time limit and captures its output.
pub async fn run(cmd: &mut Command, limit: Duration) -> Result<Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(|e| Error::io("could not start FFmpeg", e))?;
    let pid = child.id();
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(Output {
            ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
        Ok(Err(e)) => Err(Error::io("FFmpeg did not finish", e)),
        // `kill_on_drop` stops the process itself when the future is dropped
        // here; `end_tree` stops whatever a launcher started.
        Err(_) => {
            end_tree(pid).await;
            Err(Error::Encode(format!("timed out after {}s", limit.as_secs())))
        }
    }
}

/// Same as [`run`] but keeps stdout as raw bytes (for images).
pub async fn run_bytes(cmd: &mut Command, limit: Duration) -> Result<(bool, Vec<u8>, String)> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(|e| Error::io("could not start FFmpeg", e))?;
    let pid = child.id();
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok((out.status.success(), out.stdout, String::from_utf8_lossy(&out.stderr).into_owned())),
        Ok(Err(e)) => Err(Error::io("FFmpeg did not finish", e)),
        Err(_) => {
            end_tree(pid).await;
            Err(Error::Encode(format!("timed out after {}s", limit.as_secs())))
        }
    }
}

/// The last meaningful lines of FFmpeg's stderr, for error messages.
pub fn tail(stderr: &str, lines: usize) -> String {
    let kept: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let start = kept.len().saturating_sub(lines);
    kept[start..].join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(exe(name));
        std::fs::write(&path, b"").unwrap();
        path
    }

    #[test]
    fn the_bundled_copy_is_found_by_its_own_names_first() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "ffmpeg");
        touch(dir.path(), "ffprobe");
        let ffmpeg = touch(dir.path(), "sub20-ffmpeg");
        let ffprobe = touch(dir.path(), "sub20-ffprobe");

        let found = Tools::locate(None, &[dir.path().to_path_buf()]).unwrap();
        assert_eq!(found, Tools { ffmpeg, ffprobe });
    }

    #[test]
    fn a_hand_made_bundle_with_plain_names_still_works() {
        let dir = tempfile::tempdir().unwrap();
        let ffmpeg = touch(dir.path(), "ffmpeg");
        let ffprobe = touch(dir.path(), "ffprobe");
        touch(dir.path(), "sub20-ffmpeg"); // half a pair is not a bundle

        let found = Tools::locate(None, &[dir.path().to_path_buf()]).unwrap();
        assert_eq!(found, Tools { ffmpeg, ffprobe });
    }
}
