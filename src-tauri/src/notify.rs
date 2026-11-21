//! Telling the user that a watched clip is ready, without interrupting a
//! game. Most people who watch a clip folder are playing while it fills, so
//! a notice waits while Windows says the user is busy (a full-screen game,
//! presentation mode, quiet hours) or while another app's full-screen window
//! has the focus, and what piled up is said in one notice afterwards. It
//! never flashes the taskbar or takes the focus.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sub20_core::job::{JobEvent, Origin};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// The key under which a notice carries the results it names.
const RESULTS: &str = "results";

/// How often a held notice checks whether the user is back.
const RECHECK: Duration = Duration::from_secs(10);

/// What became of one watched clip.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Ready { name: String, bytes: u64, output: PathBuf },
    Failed { name: String },
}

#[derive(Default)]
pub struct Notices {
    /// Watch jobs in progress: id → source.
    watched: Mutex<HashMap<String, PathBuf>>,
    /// Outcomes not shown yet because the user was busy.
    held: Mutex<Vec<Outcome>>,
    waiting: AtomicBool,
}

impl Notices {
    /// Looks at every job event; tells the user about watched clips that
    /// finished, when `wanted` says the settings allow it.
    pub fn see(self: &Arc<Self>, app: &AppHandle, event: &JobEvent, wanted: impl Fn() -> bool) {
        let finished = match event {
            JobEvent::Queued { id, input, origin: Origin::Watch } => {
                self.watched.lock().expect("watched lock").insert(id.to_string(), input.clone());
                return;
            }
            JobEvent::Done { id, result } => self.forget(&id.to_string()).map(|_| Outcome::Ready {
                name: file_name(&result.output),
                bytes: result.size_bytes,
                output: result.output.clone(),
            }),
            JobEvent::Failed { id, .. } => {
                self.forget(&id.to_string()).map(|source| Outcome::Failed { name: file_name(&source) })
            }
            JobEvent::Canceled { id } => {
                self.forget(&id.to_string());
                None
            }
            _ => None,
        };
        let Some(outcome) = finished else { return };
        if !wanted() || window_in_front(app) {
            return;
        }
        self.held.lock().expect("held lock").push(outcome);
        self.flush_when_free(app);
    }

    fn forget(&self, id: &str) -> Option<PathBuf> {
        self.watched.lock().expect("watched lock").remove(id)
    }

    /// Shows what is held now if the user isn't busy; otherwise checks again
    /// every few seconds until they are back.
    fn flush_when_free(self: &Arc<Self>, app: &AppHandle) {
        if !user_busy() {
            self.flush(app);
            return;
        }
        if self.waiting.swap(true, Ordering::SeqCst) {
            return;
        }
        let (notices, app) = (self.clone(), app.clone());
        std::thread::spawn(move || {
            while user_busy() {
                std::thread::sleep(RECHECK);
            }
            notices.waiting.store(false, Ordering::SeqCst);
            notices.flush(&app);
        });
    }

    fn flush(&self, app: &AppHandle) {
        let outcomes = std::mem::take(&mut *self.held.lock().expect("held lock"));
        let Some((title, body)) = summary(&outcomes) else { return };
        // A click on the notice opens what it names (see `listen`).
        let results: Vec<&PathBuf> = outcomes
            .iter()
            .filter_map(|o| match o {
                Outcome::Ready { output, .. } => Some(output),
                Outcome::Failed { .. } => None,
            })
            .collect();
        if let Err(e) = app.notification().builder().title(title).body(body).extra(RESULTS, results).show() {
            log::warn!("could not show a notification: {e}");
        }
    }
}

/// Makes a click on a notice open what it names: the window comes forward
/// (built again if it was closed) and hears `notice` with the paths of the
/// results (none for a notice about failures), or finds them in
/// `bootstrap`, then opens one result on the bench or several in History. Call before the first notice: one shown with no handler reports
/// nothing. On Windows a click is heard while the notice is on screen, not
/// once it has gone to the notification centre.
pub fn listen(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.notification().on_action(move |performed| {
        if performed.action_id() != "tap" {
            return;
        }
        let results: Vec<PathBuf> = performed
            .notification()
            .and_then(|n| n.extra().get(RESULTS))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        crate::notice_clicked(&handle, results);
    });
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Sub20's own window has the focus: the title bar already says it.
fn window_in_front(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_focused().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

/// Megabytes the way the window writes them.
fn size(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1000.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else if mb >= 100.0 {
        format!("{mb:.0} MB")
    } else if mb >= 10.0 {
        format!("{mb:.1} MB")
    } else {
        format!("{mb:.2} MB")
    }
}

/// "a.mp4, b.mp4 and 2 more".
fn names(list: &[&str]) -> String {
    match list {
        [] => String::new(),
        [one] => one.to_string(),
        [first, second] => format!("{first} and {second}"),
        [first, second, rest @ ..] if rest.len() > 1 => format!("{first}, {second} and {} more", rest.len()),
        [first, second, third] => format!("{first}, {second} and {third}"),
        _ => unreachable!(),
    }
}

/// The title and text of one notice for everything that finished.
pub fn summary(outcomes: &[Outcome]) -> Option<(String, String)> {
    let ready: Vec<_> = outcomes
        .iter()
        .filter_map(|o| match o {
            Outcome::Ready { name, bytes, .. } => Some((name.as_str(), *bytes)),
            Outcome::Failed { .. } => None,
        })
        .collect();
    let failed: Vec<_> = outcomes
        .iter()
        .filter_map(|o| match o {
            Outcome::Failed { name } => Some(name.as_str()),
            Outcome::Ready { .. } => None,
        })
        .collect();
    let clips = |n: usize| if n == 1 { "1 clip".to_string() } else { format!("{n} clips") };
    Some(match (ready.as_slice(), failed.as_slice()) {
        ([], []) => return None,
        ([(name, bytes)], []) => {
            (format!("{name} is ready"), format!("{}, ready to send. Open Sub20 to copy it.", size(*bytes)))
        }
        (ready, []) => (
            format!("{} are ready", clips(ready.len())),
            format!("{}. Open Sub20 to copy them.", names(&ready.iter().map(|r| r.0).collect::<Vec<_>>())),
        ),
        ([], [name]) => (format!("{name} couldn't be compressed"), "Open Sub20 to see why.".into()),
        ([], failed) => (
            format!("{} couldn't be compressed", clips(failed.len())),
            format!("{}. Open Sub20 to see why.", names(failed)),
        ),
        (ready, failed) => (
            format!(
                "{} {} ready, {} couldn't be compressed",
                clips(ready.len()),
                if ready.len() == 1 { "is" } else { "are" },
                failed.len()
            ),
            "Open Sub20 to see them.".into(),
        ),
    })
}

/// The user is in a game, presenting, in quiet hours or away, or another
/// app fills the screen. Windows only; elsewhere the system's own Focus and
/// Do Not Disturb settings decide.
#[cfg(windows)]
fn user_busy() -> bool {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_ACCEPTS_NOTIFICATIONS, QUNS_APP};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    };

    // SAFETY: plain Win32 queries with valid out-pointers; a null window
    // handle is checked before use.
    unsafe {
        let mut state = 0;
        if SHQueryUserNotificationState(&mut state) == 0 && state != QUNS_ACCEPTS_NOTIFICATIONS && state != QUNS_APP {
            return true;
        }
        // Borderless windowed games don't always count as busy above, so a
        // full-screen window that isn't Sub20's or the desktop's counts too.
        let window = GetForegroundWindow();
        if window.is_null() {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);
        if pid == std::process::id() {
            return false;
        }
        let mut class = [0u16; 64];
        let length = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..length]);
        if class == "Progman" || class == "WorkerW" || class == "Shell_TrayWnd" {
            return false;
        }
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..std::mem::zeroed() };
        if GetWindowRect(window, &mut rect) == 0
            || GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &mut info) == 0
        {
            return false;
        }
        let screen = info.rcMonitor;
        rect.left <= screen.left && rect.top <= screen.top && rect.right >= screen.right && rect.bottom >= screen.bottom
    }
}

#[cfg(not(windows))]
fn user_busy() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(name: &str, mb: f64) -> Outcome {
        Outcome::Ready { name: name.into(), bytes: (mb * 1024.0 * 1024.0) as u64, output: PathBuf::from(name) }
    }

    fn failed(name: &str) -> Outcome {
        Outcome::Failed { name: name.into() }
    }

    fn said(outcomes: &[Outcome]) -> (String, String) {
        summary(outcomes).unwrap()
    }

    #[test]
    fn one_ready_clip_names_itself_and_its_size() {
        assert_eq!(
            said(&[ready("clip.mp4", 18.4)]),
            ("clip.mp4 is ready".into(), "18.4 MB, ready to send. Open Sub20 to copy it.".into())
        );
    }

    #[test]
    fn what_piled_up_during_a_game_is_one_notice() {
        let (title, body) =
            said(&[ready("a.mp4", 9.0), ready("b.mp4", 12.0), ready("c.mp4", 3.0), ready("d.mp4", 1.0)]);
        assert_eq!(title, "4 clips are ready");
        assert_eq!(body, "a.mp4, b.mp4 and 2 more. Open Sub20 to copy them.");
        assert_eq!(
            said(&[ready("a.mp4", 1.0), ready("b.mp4", 1.0), ready("c.mp4", 1.0)]).1,
            "a.mp4, b.mp4 and c.mp4. Open Sub20 to copy them."
        );
    }

    #[test]
    fn failures_say_so() {
        assert_eq!(said(&[failed("x.mp4")]).0, "x.mp4 couldn't be compressed");
        assert_eq!(said(&[failed("x.mp4"), failed("y.mp4")]).1, "x.mp4 and y.mp4. Open Sub20 to see why.");
        assert_eq!(said(&[ready("a.mp4", 1.0), failed("x.mp4")]).0, "1 clip is ready, 1 couldn't be compressed");
        assert_eq!(summary(&[]), None);
    }
}
