//! Starting Sub20 in the tray when the user signs in (Windows; plan 7's
//! 7.9): a value under the user's Run key, `"…\sub20.exe" --tray`, named
//! after the app's identifier so a test copy never touches the real one.
//! `settings.json` is the setting's one home; the value is written or
//! removed to match it whenever settings are saved, and checked again at
//! every start, which also follows the program if it has moved. The
//! uninstaller removes it (`windows/hooks.nsh`).

use std::io;

use crate::TRAY_ARG;

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

/// The programs Windows starts at sign-in, for this user only.
pub const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Where Task Manager keeps an entry switched off on its Startup page. Left
/// alone while the value is wanted, so the user's choice there holds;
/// removed with the value, so switching Sub20's setting off and on again
/// starts afresh.
pub const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// Writes the value when it is `wanted` and differs from what is there,
/// and removes it, with Task Manager's note on it, when it isn't.
pub fn follow(name: &str, wanted: bool) -> io::Result<()> {
    let command = if wanted { Some(command(&std::env::current_exe()?)) } else { None };
    follow_in(RUN, APPROVED, name, command.as_deref())
}

/// What the value says: the program, quoted, then `--tray`.
fn command(exe: &std::path::Path) -> String {
    format!("\"{}\" {TRAY_ARG}", exe.display())
}

fn follow_in(run: &str, approved: &str, name: &str, command: Option<&str>) -> io::Result<()> {
    match command {
        Some(command) => {
            if read(run, name)?.as_deref() != Some(command) {
                write(run, name, command)?;
            }
            Ok(())
        }
        None => {
            remove(run, name)?;
            remove(approved, name)
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn check(code: u32) -> io::Result<()> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(code as i32))
    }
}

/// The value's text, or `None` when there is no such value (or key).
fn read(key: &str, name: &str) -> io::Result<Option<String>> {
    let (key, name) = (wide(key), wide(name));
    let mut buffer = vec![0u16; 512];
    loop {
        let mut bytes = (buffer.len() * 2) as u32;
        // SAFETY: both names are NUL-terminated, and `bytes` is the
        // buffer's size in bytes, which the call never writes past.
        let code = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        match code {
            ERROR_FILE_NOT_FOUND => return Ok(None),
            ERROR_MORE_DATA => buffer.resize((bytes as usize).div_ceil(2) + 1, 0),
            code => {
                check(code)?;
                // `bytes` counts the terminating NUL, which RRF_RT_REG_SZ
                // guarantees.
                let units = &buffer[..(bytes as usize / 2).saturating_sub(1)];
                return Ok(Some(String::from_utf16_lossy(units)));
            }
        }
    }
}

/// Writes a text value, making the key if it isn't there.
fn write(key: &str, name: &str, value: &str) -> io::Result<()> {
    let (key, name, value) = (wide(key), wide(name), wide(value));
    // SAFETY: NUL-terminated names; the size is the value's in bytes,
    // terminating NUL included, as REG_SZ requires.
    check(unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            REG_SZ,
            value.as_ptr().cast(),
            (value.len() * 2) as u32,
        )
    })
}

/// Removes a value; one that isn't there is already removed.
fn remove(key: &str, name: &str) -> io::Result<()> {
    let (key, name) = (wide(key), wide(name));
    // SAFETY: NUL-terminated names.
    match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) } {
        ERROR_FILE_NOT_FOUND => Ok(()),
        code => check(code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Registry::RegDeleteKeyW;

    /// A key of the test's own under HKCU, never the real Run key, deleted
    /// when the test ends however it ends.
    struct Scratch(String);

    impl Drop for Scratch {
        fn drop(&mut self) {
            for sub in ["approved", "run"] {
                let key = wide(&format!(r"{}\{sub}", self.0));
                // SAFETY: a NUL-terminated name.
                unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, key.as_ptr()) };
            }
            let key = wide(&self.0);
            // SAFETY: a NUL-terminated name.
            unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, key.as_ptr()) };
        }
    }

    #[test]
    fn the_value_follows_the_setting() {
        let scratch = Scratch(format!(r"Software\io.github.y4dd.sub20.test-sign-in-{}", std::process::id()));
        let (run, approved) = (format!(r"{}\run", scratch.0), format!(r"{}\approved", scratch.0));
        let name = "io.github.y4dd.sub20.test";
        let exe = std::path::Path::new(r"C:\Users\demo\AppData\Local\Sub20\sub20.exe");
        let wanted = command(exe);
        assert_eq!(wanted, r#""C:\Users\demo\AppData\Local\Sub20\sub20.exe" --tray"#);

        assert_eq!(read(&run, name).unwrap(), None, "nothing there yet");
        follow_in(&run, &approved, name, Some(&wanted)).unwrap();
        assert_eq!(read(&run, name).unwrap().as_deref(), Some(wanted.as_str()));
        // Again, unchanged; and a program that moved is followed.
        follow_in(&run, &approved, name, Some(&wanted)).unwrap();
        let moved = command(std::path::Path::new(r"D:\Apps\Sub20 Test\sub20.exe"));
        follow_in(&run, &approved, name, Some(&moved)).unwrap();
        assert_eq!(read(&run, name).unwrap().as_deref(), Some(moved.as_str()));

        // Task Manager's note goes with the value, so switching the setting
        // on again isn't still switched off there.
        write(&approved, name, "disabled").unwrap();
        follow_in(&run, &approved, name, None).unwrap();
        assert_eq!(read(&run, name).unwrap(), None);
        assert_eq!(read(&approved, name).unwrap(), None);
        // Removing what isn't there is fine.
        follow_in(&run, &approved, name, None).unwrap();
    }

    #[test]
    fn a_long_value_is_read_whole() {
        let scratch = Scratch(format!(r"Software\io.github.y4dd.sub20.test-long-{}", std::process::id()));
        let run = format!(r"{}\run", scratch.0);
        let long = format!("\"C:\\{}\\sub20.exe\" --tray", "deep folder\\".repeat(80));
        write(&run, "x", &long).unwrap();
        assert_eq!(read(&run, "x").unwrap().as_deref(), Some(long.as_str()));
    }
}
