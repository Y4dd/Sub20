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
        if cfg!(windows) {
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
        self.check().await.ok()
    }

    /// Runs `ffmpeg -version` and returns its first line, or why FFmpeg did
    /// not answer. A file that exists is not yet an FFmpeg that runs: a
    /// launcher whose target is gone, a damaged download, or a system that
    /// refuses to start it all look found until this is asked.
    pub async fn check(&self) -> std::result::Result<String, String> {
        let mut cmd = self.ffmpeg();
        cmd.arg("-version");
        let out = match run(&mut cmd, Duration::from_secs(10)).await {
            Ok(out) => out,
            Err(Error::Io { source, .. }) => return Err(format!("it could not be started ({source})")),
            Err(Error::Encode(_)) => return Err("it did not answer within 10 seconds".to_string()),
            Err(other) => return Err(other.to_string()),
        };
        let first = out.stdout.lines().next().map(|line| line.split(" Copyright").next().unwrap_or(line).trim());
        match first {
            Some(line) if out.ok && line.starts_with("ffmpeg version") => Ok(line.to_string()),
            _ => {
                let said = tail(&out.stderr, 1);
                let code = out.code.map(exit_code).unwrap_or_else(|| "no exit code".to_string());
                Err(if said.is_empty() {
                    format!("it stopped with {code}")
                } else {
                    format!("it stopped with {code}: {said}")
                })
            }
        }
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

/// On Linux, package managers link FFmpeg directly or `exec` it
/// from a wrapper, so the process killed is FFmpeg itself.
#[cfg(not(windows))]
async fn end_tree(_pid: Option<u32>) {}

/// Do not flash a console for every FFmpeg call.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// A priority class given at creation, inherited by what the process starts.
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

/// How far down a watched encode runs on Linux: the scheduler gives a
/// normal process (nice 0) about nine times its share.
#[cfg(unix)]
pub const WATCHED_NICE: i32 = 10;

/// Runs FFmpeg below normal priority, so a watched clip compressing while a
/// game runs gives way to it (plan 7's 7.8). On Windows it is the priority
/// class, given at creation, which a launcher's child (Chocolatey's shims)
/// inherits; on Linux, nice [`WATCHED_NICE`], set in the child before FFmpeg
/// starts, so every thread FFmpeg makes has it. A failure to lower it is
/// ignored: the encode runs as before.
pub(crate) fn lower_priority(cmd: &mut Command) {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    #[cfg(unix)]
    // SAFETY: `nice` is a plain system call, safe between fork and exec; it
    // touches no memory of the parent's and its result is ignored.
    unsafe {
        cmd.pre_exec(|| {
            libc::nice(WATCHED_NICE);
            Ok(())
        });
    }
}

pub(crate) fn quiet(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Captured result of a short helper run.
pub struct Output {
    pub ok: bool,
    /// The exit code, when the process gave one.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// An exit code as people search for it: Windows' start-up failures are
/// NTSTATUS values, written in hex (0xC0000142), others in decimal.
fn exit_code(code: i32) -> String {
    if cfg!(windows) && code < 0 {
        format!("exit code {:#010X}", code as u32).replace("0X", "0x")
    } else {
        format!("exit code {code}")
    }
}

/// Runs a command to completion with a time limit and captures its output.
pub async fn run(cmd: &mut Command, limit: Duration) -> Result<Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(|e| Error::io("could not start FFmpeg", e))?;
    let pid = child.id();
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(Output {
            ok: out.status.success(),
            code: out.status.code(),
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

    #[tokio::test]
    async fn an_ffmpeg_that_will_not_start_says_why() {
        // Found, because the files are there, but not a program at all.
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "ffmpeg");
        touch(dir.path(), "ffprobe");
        let found = Tools::locate(None, &[dir.path().to_path_buf()]).unwrap();

        let why = found.check().await.unwrap_err();
        assert!(why.starts_with("it could not be started ("), "{why}");
        assert_eq!(found.version().await, None);
    }

    #[test]
    fn windows_start_up_failures_read_as_hex() {
        if cfg!(windows) {
            assert_eq!(exit_code(0xC000_0142_u32 as i32), "exit code 0xC0000142");
        }
        assert_eq!(exit_code(1), "exit code 1");
    }
}
