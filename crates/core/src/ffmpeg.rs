//! Finding FFmpeg/FFprobe and starting them without a console window.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

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
    let ffmpeg = dir.join(exe("ffmpeg"));
    let ffprobe = dir.join(exe("ffprobe"));
    (ffmpeg.is_file() && ffprobe.is_file()).then_some(Tools { ffmpeg, ffprobe })
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
            if let Some(found) = pair_in(dir) {
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

fn quiet(program: impl AsRef<OsStr>) -> Command {
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
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(Output {
            ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
        Ok(Err(e)) => Err(Error::io("FFmpeg did not finish", e)),
        // `kill_on_drop` stops the process when the future is dropped here.
        Err(_) => Err(Error::Encode(format!("timed out after {}s", limit.as_secs()))),
    }
}

/// Same as [`run`] but keeps stdout as raw bytes (for images).
pub async fn run_bytes(cmd: &mut Command, limit: Duration) -> Result<(bool, Vec<u8>, String)> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(|e| Error::io("could not start FFmpeg", e))?;
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok((out.status.success(), out.stdout, String::from_utf8_lossy(&out.stderr).into_owned())),
        Ok(Err(e)) => Err(Error::io("FFmpeg did not finish", e)),
        Err(_) => Err(Error::Encode(format!("timed out after {}s", limit.as_secs()))),
    }
}

/// The last meaningful lines of FFmpeg's stderr, for error messages.
pub fn tail(stderr: &str, lines: usize) -> String {
    let kept: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let start = kept.len().saturating_sub(lines);
    kept[start..].join(" | ")
}
