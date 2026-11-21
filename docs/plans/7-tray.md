# Plan 7: the watch without a window

**Goal.** On Windows, Sub20 can keep watching a folder after its window is
closed, from the tray, and can start that way when you sign in, at a cost
small enough to leave running while you play: under 100 MB and under 0.5%
of one core with a watch on and no window. Off by default.

**Why it moved** (owner, 20 November 2026): this was plan 6's 6.3, after
1.0. The owner wants it sooner, because the watch is what gamers recording
with the NVIDIA app or Game Bar use, and today the watch stops when the
window closes. It was checked against the code that day (ECC's planner,
then by hand): the watch, its notices and the history already live in the
shell, so the idea holds, but closing the window today would also lose
videos opened from Explorer, notice clicks and the question about a pile
of clips, and nothing would check for updates. Those come first (7.2).

**The cut line.** It ships off by default in 0.9.x, so testers can try
it. If 7.1's measurement or 7.10's gate fails, the option stays hidden,
the plan goes back to plan 6, and 1.0 launches without it. The launch
does not wait on this plan.

**Status.** Started 20 November 2026: 7.1 passed (8 MB with no window).

**Needs first:** nothing. **Runs beside:** plans 3 and 4. **Unblocks:**
nothing; plan 5 includes it only if it is finished.

**Windows first.** Linux keeps today's behaviour, with the option hidden.
A tray on Linux needs `libayatana-appindicator`, GNOME shows no tray, and
starting an AppImage at sign-in needs its path: those wait in plan 6
until after the owner's Arch sitting.

## Tasks

- [x] **7.1 Destroy and measure, first.** Done 20 November 2026, in a
      release test copy (numbers in [record.md](record.md), "Closing the
      window"): WebView2's processes exit with the window, and what stays
      is Sub20's own process at 8 MB private, also after five
      open-and-close cycles. An empty Tauri window is 156 MB. The plan
      goes on. Today's numbers
      ([record.md](record.md)) were taken with a window alive, and "about
      40 MB" for Sub20's own process doesn't say whether it is working set
      or private bytes. In a release-style test copy with its own
      identifier: close the window while the process stays (prevent the
      exit when the last window closes, `ExitRequested` with no `code`),
      build it again from its config, and measure private bytes summed
      over `sub20.exe` and every process under it, 30 seconds each: after
      closing with 20 clips loaded, after five open-and-close cycles, and
      an empty Tauri window for WebView2's own floor.
      *Done when:* the numbers are in the record and say whether
      WebView2's processes exit with the window. If the process tree stays
      over 100 MB, stop here: the plan goes back to plan 6.
- [ ] **7.2 A window that can come back.** The main window is no longer
      made at startup (`create: false`, in `tauri.conf.json` and
      `tauri.windows.conf.json`, since each declares the whole window). One
      shell function shows it, building it if it is gone, and everything
      that brings the window forward uses it:
      1. `hand_over`, also with no paths, so a plain second launch brings
         the window forward (today it does nothing).
      2. `WINDOW_READY` is cleared when the window is destroyed, so opened
         videos wait in `OPENED` again.
      3. A notice click with no window is held until `bootstrap` takes it,
         like `OPENED`.
      4. `bootstrap` returns this session's watch jobs, kept in the shell,
         so a rebuilt window's History and `openNotice` still know them
         (today `app.watched` is built only from that window's events).
      *Done when:* a second launch brings the window forward, the
      watch-job list has a test, and fmt, clippy, the tests and
      svelte-check are clean. This ships on its own, before the tray.
- [ ] **7.3 The setting.** `WatchSettings::keep_watching`, false by
      default, through `tidy()`, with a test and its mirror in `types.ts`.
      Settings offers "Keep watching when the window is closed" under the
      watch, only when a watch is set up, and only where a capability flag
      says the tray exists (Windows).
      *Done when:* old settings files load, and the option is hidden in
      Lite and on Linux.
- [ ] **7.4 Close to the tray.** Tauri's `tray-icon` feature, for the
      Windows target only. The tray menu has Open (also a left click),
      Pause watching and Quit. With the setting on, closing the window
      destroys it and keeps the process, without asking when only watched
      jobs are running. Quit asks through `may_quit` as today.
      *Done when:* in `npx tauri build --debug --no-bundle`, closing leaves
      the tray icon, Open brings the window back with its own frame, and
      Quit during a watched encode asks.
- [ ] **7.5 Pause watching, in the core.** `Watcher::pause` and `resume`,
      for this session only, and `WatchStatus::paused` with its mirror.
      Clips that arrived during a pause follow the pile rule
      (`watch::PILE`) on resume.
      *Done when:* tests in `watch.rs`, and the title bar says the watch
      is paused.
- [ ] **7.6 The pile with no window.** When more than `PILE` clips are
      held and there is no window to ask, a notice says so ("7 clips
      arrived while Sub20 was closed. Open Sub20 to compress or skip
      them.") and clicking it opens the window on the question.
      *Done when:* tried on the mock and in a test copy started without a
      window.
- [ ] **7.7 Updates with no window.** The shell checks at start when
      `check_updates` is on, since today only the window checks, and the
      tray offers "Update to x.y.z". It never installs by itself: 3.6
      recorded FFmpeg failing after an updater restart.
      *Done when:* a test copy updated from the tray comes back in the
      tray.
- [ ] **7.8 Watched encodes at lower priority.** A job's origin reaches the
      spawn of FFmpeg (`ffmpeg::quiet`). Watched jobs run at
      `BELOW_NORMAL_PRIORITY_CLASS` on Windows, which a launcher's child
      inherits, and nice 10 on Linux. Engine logic, so it lives in
      `crates/core`.
      *Done when:* an e2e test reads the priority of a running FFmpeg
      back, on both CI systems.
- [ ] **7.9 Start at sign-in.** `WatchSettings::start_at_sign_in`, which
      `tidy()` clears unless `keep_watching` is on. A Run value under
      HKCU, `"…\sub20.exe" --tray`, written or removed when the setting is
      saved and checked again at startup, so `settings.json` stays its one
      home. `--tray` starts without a window, and a second launch with
      `--tray` shows nothing. The NSIS uninstall hook removes the value,
      inside its `$UpdateMode` guard. Written directly (a few dozen lines,
      `windows-sys` is already a dependency) rather than through
      `tauri-plugin-autostart`, so the arguments stay Sub20's.
      *Done when:* the Run value appears and goes as the setting changes,
      and an uninstall leaves none.
- [ ] **7.10 The budget, as a gate.** 7.1's method on a start from the
      tray, after a close, and after five cycles, with the watch on 2,000
      files.
      *Done when:* under 100 MB private and under 0.5% of one core, with
      the numbers in the record. If not, the option stays hidden and the
      plan goes back to plan 6.
- [ ] **7.11 Words.** `CLAUDE.md`, a section in `docs/testing.md`, the
      README, the changelog, and any site guide that describes the watch.

## Needs the owner

- [ ] **What closing does while a manual compress runs,** with the
      setting on. Until the owner says otherwise: it asks, as today, and
      the clips on the bench are lost with the window.
- [ ] **A game, twice:** a full-screen and a borderless game while a
      watched clip encodes, with an overlay showing frame times, once at
      normal priority and once below normal. Watched jobs use NVENC first,
      and CPU priority does nothing about sharing the GPU, so "doesn't
      cost frames" is unproven until this.
- [ ] **Sign out and in** with start at sign-in on: the tray appears, the
      watch runs, the pile notice shows.
- [ ] **An update from the tray** on the owner's machine.
- [ ] **Task Manager's Startup toggle:** what turning Sub20 off there does
      to the setting.

## Exit criteria

- [ ] 7.1 to 7.11 are ticked, or the plan has gone back to plan 6 with
      the reason written here.
- [ ] Every line under "Needs the owner" is ticked.
- [ ] At least one tester ran it for a few days with a watch on.
