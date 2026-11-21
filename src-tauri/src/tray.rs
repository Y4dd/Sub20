//! The tray icon, on Windows only (plan 7). It is there while the watch
//! keeps going after the window closes (`WatchSettings::keep_watching`, with
//! a watch on), so closing never leaves a process nobody can see or quit.
//! A left click or Open brings the window back, built again if it was
//! closed; Quit asks first while videos are compressing, as closing does.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

const ID: &str = "sub20";

/// Shows the tray icon when `wanted`, and takes it away otherwise.
pub fn follow(app: &AppHandle, wanted: bool) {
    let shown = shown(app);
    if wanted && !shown {
        if let Err(e) = build(app) {
            log::warn!("could not add the tray icon: {e}");
        }
    } else if !wanted && shown {
        // Dropping the last handle takes the icon off the taskbar.
        drop(app.remove_tray_by_id(ID));
    }
}

/// The icon is in the tray.
pub fn shown(app: &AppHandle) -> bool {
    app.tray_by_id(ID).is_some()
}

fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Sub20", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut builder = TrayIconBuilder::with_id(ID)
        .tooltip("Sub20")
        .menu(&menu)
        // A left click opens the window; the menu is on the right button.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => crate::show_window(app),
            // `ExitRequested` asks through `may_quit` while jobs run.
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Down and Up both arrive; one click opens once.
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
