//! The tray icon, on Windows only (plan 7). It is there while the watch
//! keeps going after the window closes (`WatchSettings::keep_watching`, with
//! a watch on), so closing never leaves a process nobody can see or quit.
//! A left click or Open brings the window back, built again if it was
//! closed; Pause watching stops looking at the folder until Resume or the
//! next start; Quit asks first while videos are compressing, as closing does.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

const ID: &str = "sub20";

/// Shows the tray icon when `wanted`, and takes it away otherwise.
pub fn follow(app: &AppHandle, wanted: bool, paused: bool) {
    let shown = shown(app);
    if wanted && !shown {
        if let Err(e) = build(app, paused) {
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

/// The menu and the tooltip say whether the watch is paused.
pub fn paused(app: &AppHandle, paused: bool) {
    let Some(tray) = app.tray_by_id(ID) else { return };
    match menu(app, paused) {
        Ok(menu) => drop(tray.set_menu(Some(menu))),
        Err(e) => log::warn!("could not update the tray menu: {e}"),
    }
    drop(tray.set_tooltip(Some(tooltip(paused))));
}

fn tooltip(paused: bool) -> &'static str {
    if paused {
        "Sub20, watch paused"
    } else {
        "Sub20"
    }
}

fn menu(app: &AppHandle, paused: bool) -> tauri::Result<Menu<Wry>> {
    let open = MenuItem::with_id(app, "open", "Open Sub20", true, None::<&str>)?;
    let pause = if paused { "Resume watching" } else { "Pause watching" };
    let pause = MenuItem::with_id(app, "pause", pause, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(app, &[&open, &pause, &quit])
}

fn build(app: &AppHandle, paused: bool) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id(ID)
        .tooltip(tooltip(paused))
        .menu(&menu(app, paused)?)
        // A left click opens the window; the menu is on the right button.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => crate::show_window(app),
            // Reloading the watch spawns its scan, so it runs on the runtime.
            "pause" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { crate::toggle_pause(&app) });
            }
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
