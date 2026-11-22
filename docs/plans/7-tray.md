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

**Status.** Started 20 November 2026: 7.1 passed (8 MB with no window); 7.2 to 7.4, 7.12 and 7.13 done the same day; 7.5 on 22 November 2026.

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
- [x] **7.2 A window that can come back.** Done 20 November 2026: in a
      release test copy, a minimised window came forward on a plain second
      launch and that launch quit, a second launch with two clips added
      them, and closing still quits (until 7.4). The watch-job list is
      `sub20_core::watched` with three tests; fmt, clippy, the tests and
      svelte-check are clean. The owner looked at the window the shell
      now builds at startup the same day: its own title bar, dragging,
      snapping, maximise and close behave as before. A window built again
      after closing is 7.4's to look at. The main window is no longer
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
- [x] **7.3 The setting.** Done 20 November 2026:
      `keep_watching_is_off_in_old_files_and_kept_once_on` loads a file
      from before it (off) and keeps it once on; `Capabilities::tray` is
      true in the desktop app on Windows only (`tauri.ts`, from the user
      agent like the frame), false in Lite, and on the mock false under
      `?frame=native`. Tried on the mock in both schemes: the toggle sits
      under the watch's other toggles, the watch's note changes to "Keeps
      running from the tray when the window is closed.", and it is absent
      with Linux's frame. fmt, clippy (with and without the engine), the
      settings tests and svelte-check are clean. It does nothing until
      7.4, so don't release it without 7.4.
      `WatchSettings::keep_watching`, false by
      default, through `tidy()`, with a test and its mirror in `types.ts`.
      Settings offers "Keep watching when the window is closed" under the
      watch, only when a watch is set up, and only where a capability flag
      says the tray exists (Windows).
      *Done when:* old settings files load, and the option is hidden in
      Lite and on Linux.
- [x] **7.4 Close to the tray.** Done 20 November 2026. The owner tried
      it in the debug test copy the same day: the icon comes and goes with
      the setting, closing asks nothing and leaves the icon, a left click
      and Open bring the window back with its own title bar, and Quit
      during a watched encode (the long clip, with "Compress new videos
      straight away" on) asked; "Keep compressing" kept Sub20 running and
      "Stop and quit" quit. Checked by script before that, in a debug test copy with
      the setting on: closing the window left one process at 8–9 MB with
      no WebView2 processes, a clip dropped in the watched folder was
      compressed with no window, and a plain second launch built the
      window again (one process). Six launches at once with six new clips,
      as Explorer starts them, made one process and one window and added
      all six, both cold and from the tray (the window is built on a
      thread of its own, one build at a time). `Engine::active_manual` is tested in
      `cancel_stops_the_encode_and_cleans_up`. Decided while building:
      the tray needs the watch on as well as `keep_watching` (the setting
      outlives switching the watch off), and is there while both are on,
      window or not; the process stays only while the icon is there.
      Pause watching joins the menu in 7.5, with the thing it calls.
      The owner's first try: Quit during a watched encode quit without
      asking, most likely because the clip had not been picked up yet
      (nothing in `watch.json`, no partial file; Tauri does let `exit(0)`
      be prevented, though a job canceled by the quit would leave the same
      traces); tried again once the clip was being compressed, it asked.
      Tauri's `tray-icon` feature, for the
      Windows target only. The tray menu has Open (also a left click),
      Pause watching and Quit. With the setting on, closing the window
      destroys it and keeps the process, without asking when only watched
      jobs are running. Quit asks through `may_quit` as today.
      *Done when:* in `npx tauri build --debug --no-bundle`, closing leaves
      the tray icon, Open brings the window back with its own frame, and
      Quit during a watched encode asks.
- [x] **7.12 The watch puts clips on the bench** Done 20 November 2026:
      `new_clips_go_on_the_bench_once_and_are_not_compressed` (seven at
      once, no pile question, not handed over again after a relaunch); the
      pile tests and the e2e watch test now set `compress`. On the mock in
      both schemes: the section reads "Folder watch", the compressing rows
      show only with "Compress new videos straight away" on, and a clip the
      watch finds joins the bench without moving the selection. In a debug
      test copy a clip dropped in the folder joined the bench with the
      window open and with only the tray. The owner tried both by hand the
      same day, with the Settings section, a clip benched while closed,
      and the bench kept across closing (trim, name, leave out).
      (owner, 20 November 2026,
      after trying 7.4: people want to trim before compressing). Taken
      after 7.4, before 7.5. Settings' toggle becomes "Watch a folder", and
      under it "Compress new videos straight away"
      (`WatchSettings::compress`), off by default, also for watches set up
      before (owner's choice; the changelog says so). Off, a clip that has
      settled is handed to the shell instead of the engine, marked
      `benched` in `watch.json`, and joins the bench like an opened video,
      without taking the selection; the pile question is only for
      compressing. On, the watch works as today. Profile, Results,
      Originals and the notice toggle apply only to compressing, so they
      show only when it is on.
      *Done when:* a test in `watch.rs` shows benched clips handed over
      once and never compressed, the mock shows the new settings in both
      schemes, and in a test copy a clip dropped in the folder lands on the
      bench.
- [x] **7.13 The bench is kept.** Done 20 November 2026: three tests in
      `bench.rs`; in a debug test copy a hand-written bench (a clip
      trimmed to 1.5–6 s, named, left out), a clip the watch found at
      launch, one dropped with the window open and one with only the tray
      were all in `bench.json`, and all four were there again, trim, name
      and "leave out" kept, after the window was built again and after the
      process was stopped and started; a clip opened with Sub20 reached
      the file, so the window's own save works. A clip whose read throws
      at launch (FFmpeg missing) stays in every save, so a bad start
      doesn't empty the file: built, not tried, since this machine's
      FFmpeg can't be hidden from the lookup. Known limits: a bench of
      more than 1,000 clips comes back as its first 1,000 (`crawl::MOST`),
      and "Put those on the bench too" on a folder of more than 1,000 marks
      them all benched while the window takes the first 1,000. The clips on the bench that aren't
      finished (ready, failed, canceled, left out, and running ones, which
      quitting stops) are saved with their trim, typed name and "leave
      out" to `bench.json` (`sub20_core::bench`), by the window a moment
      after they change, and come back at the next launch or when the
      window is built again; files that are gone are dropped. Clips the
      watch benches while there is no window are added to that file by the
      shell, so they are waiting when the window opens. Desktop only: Lite's
      files can't be opened again.
      *Done when:* tests in `bench.rs` (round trip, missing files dropped,
      adding without duplicates), and in a test copy a trimmed clip and a
      clip benched from the tray are on the bench after quitting and
      starting again.
- [x] **7.5 Pause watching, in the core.** Done 22 November 2026:
      `a_paused_watch_looks_again_on_resume_and_asks_about_a_pile` (seven
      clips settle during a pause and nothing is sent; on resume they are
      held as a pile; a settings reload keeps the pause) and
      `a_pause_ends_when_the_watch_is_switched_off`. On the mock in both
      schemes: History's Pause link, "Watch paused" in the title bar with a
      ring for its dot, Resume back to "Watching". The tray's Pause
      watching / Resume watching item (`watch_pause`, `set_paused`) is
      built and clippy-clean, not tried by hand yet. Running jobs finish.
      `Watcher::pause` and `resume`,
      for this session only, and `WatchStatus::paused` with its mirror.
      Clips that arrived during a pause follow the pile rule
      (`watch::PILE`) on resume.
      *Done when:* tests in `watch.rs`, and the title bar says the watch
      is paused.
- [ ] **7.6 The pile with no window.** When more than `PILE` clips are
      held and there is no window to ask, a notice says so ("7 clips
      arrived while Sub20 was closed. Open Sub20 to compress or skip
      them.") and clicking it opens the window on the question. Since
      7.12 that question is only for "Compress straight away"; with it off
      (the default), the notice says a clip is on the bench ("clip.mp4 is
      on the bench", or "7 clips are on the bench"), held during games like
      the others, and clicking it opens the window. Since 7.5, resuming
      from the tray with no window can hold a pile too, and needs the
      same notice.
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
      Started 22 November 2026, for the release with 7.2 to 7.5: the
      README's feature line, `docs/testing.md` §4 (bench, compress, tray,
      pause, Quit), the changelog, and the NVIDIA guide's watch section
      (read against the app, `checked` moved). Left: 7.6 to 7.9's words
      as they land. The home page's watch card ("picks up each new clip
      as it lands") is still true; its screenshot shows a compressed clip
      in History, the compressing mode.

## Needs the owner

- [ ] **What closing does while a manual compress runs,** with the
      setting on. Until the owner says otherwise: it asks, as today. Since
      7.13 the clips on the bench are kept either way.
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
