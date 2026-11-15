# Plan 3: the desktop beta

**Goal.** The desktop app has been used by hand, in the real window, on
Windows, macOS and Linux; a small beta (three to five people, at least one
not on NVIDIA and one on a Mac) has used it for a week or more; and nothing
they found that loses data or crashes is still open. After this plan the
desktop app is ready to be called 1.0.

**Why a small beta is enough** (owner, 15 November 2026, replacing ten
testers on three GPU makers for two weeks, which the launch was needed to
find): a file over the limit is never saved, and a hardware encoder that
fails hands the job to the CPU. So an untested Quick Sync, AMF,
VideoToolbox or VAAPI path can cost speed, not a file that doesn't fit.
What that doesn't cover is picture and colour on those encoders: until a
tester has seen them, they are called unverified, never safe.

**Status.** Started 14 November 2026: the test sheet is written, real
clips have run through the engine, and the first beta reports are fixed
(3.6). The features for 1.0 are built
([record.md](record.md)). Most were checked on the mock backend, from the
command line, or on one Windows machine with an NVIDIA card. What has not
been tried by hand is listed in `CLAUDE.md` under "Verified, and not".

**Needs first:** plan 2 (a release to install). **Runs beside:** plan 4.
**Unblocks:** plan 5.

Most of this plan is the owner and a few other people trying things. The
AI's part is to make that easy (3.1), to test what can be tested from here
(3.2, 3.3), and to fix what comes back (3.6). While this plan waits on
people, the AI works on plan 4 or on plan 5's drafts.

## Tasks

- [ ] **3.1 A test sheet.** `docs/testing.md`: the hands-on pass as numbered
      steps a tester can follow in twenty minutes, each with what should
      happen, and how to report (Settings → Copy diagnostics, pasted into
      the bug template). Sections: install and first launch, compress and
      send to Discord, the window, the folder watch, opening videos from
      the system, updating, a screen reader. It covers every line under
      "By hand" below, so a filled-in sheet ticks them.
      *Done when:* the sheet exists and the owner has followed it once on
      Windows without asking what a step means.
      Written 14 November 2026, every label taken from the components
      (parts 1 to 3 are the twenty minutes; 4 to 8 cover the rest of "By
      hand"). Waits for the owner's run on Windows.
- [ ] **3.2 Real clips through the engine.** With files the owner
      provides, run `cargo run -p sub20-core --example compress` on: an OBS
      recording (MKV, game and microphone on separate tracks), an NVIDIA
      clip with the microphone on its own track, Xbox Game Bar, a Steam
      export, Medal, an iPhone video (HEVC, HDR, rotated) and an Android
      video (variable frame rate). Record size against the limit, the
      encoder and the attempts in [record.md](record.md). For HDR, compare
      the result's colour with the source.
      *Done when:* each kind the owner could provide came out under the
      limit and looks and sounds right, or has an issue.
      Started 14 November 2026: the NVIDIA app clips on this machine,
      including HEVC and 2-3 minute recordings, all under on the first try
      ([record.md](record.md)). Left: the other kinds, which need files.
- [ ] **3.3 A long real recording** (an hour or more) in a release test
      copy: memory while it loads and compresses, the time estimate against
      the real time, Page Up and Page Down on the trim handles.
      *Done when:* the numbers are in the record and nothing grew without
      bound.
- [ ] **3.4 Open the result from a watch notice.** Clicking "clip.mp4 is
      ready" should put that clip on the bench as a finished clip, with
      Copy ready. The notification plugin reports no clicks on desktop, so
      Windows needs a toast activation handler of Sub20's own. If that
      costs more than a day or needs a packaged app identity, write down
      why and move it to plan 6.
      *Done when:* a click opens the result in a release build on Windows,
      or the task has moved with its reason.
      Built 14 November 2026 without an activation handler:
      tauri-plugin-notification 2.8 reports clicks on desktop
      (`on_action`), so `notify::listen` brings the window forward and opens
      what the notice names (one clip on the bench, several in History).
      On Windows that works while the notice is on screen, not from the
      notification centre; that part would need a COM activator and a
      registered app identity, and goes to plan 6 if testers miss it.
      Checked on the mock (`?watch=notice`); waits for a click in a release
      build on Windows.
- [ ] **3.5 The first update.** When the first fix is ready, release 0.9.1
      the way plan 2 released 0.9.0. Then, on every kind someone can reach:
      0.9.0 says an update is out, installs it, restarts as 0.9.1, and
      stays the kind it was (with or without FFmpeg; deb stays deb, rpm
      stays rpm).
      *Done when:* seen on Windows and on at least one deb or rpm install.
      *Released 15 November 2026:* v0.9.1 published at 07:21 UTC as latest
      (CI and the release build green on all three systems, 24 files, ten
      signed feed entries pointing at v0.9.1, the site's links follow it).
      Seen on Windows the same morning (owner): 0.9.0 offered it, installed
      it and came back as 0.9.1, and FFmpeg worked after that restart, so
      the failure after the 0.9.0 update did not repeat (still unexplained).
      Left: a deb or rpm install by a tester. The `github-pages`
      environment now allows `v*` tags beside `main` (owner), so the next
      published release deploys the site by itself.
- [ ] **3.6 Fix what the beta finds.** Every report becomes an issue. A bug
      that loses a file, saves one over the limit, or crashes is fixed
      before anything else, with a test where one can be written. Fixes go
      out as 0.9.x releases, a few at a time. Requests for new features go
      to plan 6.
      *Done when:* the exit criteria hold.
      Reports are written down here, under the date and who reported
      them; a tester's report also becomes a GitHub issue, so they can
      follow it. Releases (owner, 15 November 2026, after four in one day):
      a fix for a lost file, a file over the limit or a crash goes out at
      once; everything else waits for one release a week.
      First reports (owner, 14 November 2026):
      - Adding many videos is slow. Cause: every file was read one after
        another and nothing showed until the last, and every video's
        filmstrip started at once (four FFmpeg processes each, so 120 for
        30 videos). Fix: read six at a time, show each in the list as it
        is read (in name order), and make filmstrips two at a time, the
        selected video's first.
      - A folder should be crawled, not opened one level deep. Fix: a
        dropped or chosen folder is searched with its subfolders for
        videos, skipping Sub20's own results, hidden folders and
        unfinished files, up to 1,000 videos; "Choose a folder" beside
        "Choose videos".
      - The result can't be sought, plays in another player, and people
        trimmed it after compressing. The owner chose to cut the result
        itself rather than compress again. Fix: the Result view plays in
        Sub20's own player, over a timeline of the result with its own
        handles, and **Cut** shortens the file by copying its packets (quick,
        no loss, always smaller). A copy can only start on a keyframe, so
        the start handle snaps to the result's keyframes and the cut is what
        was shown; results now get a keyframe every 2 seconds (`-g`), which
        also makes them seek better in Discord. The uncut file goes to the
        bin, the history entry follows the cut, and the original's trim
        moves with it, so Compress again starts from what is kept. Not in
        Lite yet (`can.cut`).

      Report after the 0.9.0 update (owner, 15 November 2026, Windows,
      Sub20 without FFmpeg, FFmpeg 9.0.2 from Scoop):
      - Every dropped video "can't be read", and Copy diagnostics says
        "FFmpeg not found" with every encoder "not in this FFmpeg", though
        `ffmpeg` works in a terminal. What was measured: Sub20 found Scoop's
        shim (`~\scoop\shims`, on the running Sub20's PATH, and the
        engine's encoder test ran), but `ffmpeg -version` gave it no answer.
        The same shim answers from a small windowed test program started
        the way Sub20 starts FFmpeg. The Sub20 that failed was the one the
        update installer restarted (its parent was the installer). Not yet
        known: whether a Sub20 opened by hand runs the shim, which tells an
        updater-restart problem from a Scoop one.
      - Fix (0.9.1): FFmpeg counts as found only when `ffmpeg -version`
        answers (`Tools::check`). One that doesn't puts the window on the
        FFmpeg screen with where it is and why (the error, the exit code,
        or FFmpeg's last line), offers the download, and Look again starts
        afresh; Settings and Copy diagnostics say the same, and an encoder
        report made while FFmpeg didn't answer isn't kept.
      - Answer: after quitting and opening Sub20 from the Start menu, FFmpeg
        works. So only the copy the update installer restarted couldn't run
        it. That restart is Tauri's NSIS template (`RunAsUser`, which for a
        per-user install is a plain `ShellExecute` from `$INSTDIR`); nothing
        in it explains the failure, and the failing process is gone. 3.5
        (0.9.0 to 0.9.1) repeats the update; if FFmpeg fails after that
        restart, 0.9.1's FFmpeg screen and Copy diagnostics now name why.
      - The Cut start handle moved two seconds at a time while the end
        handle moved smoothly. Cause: the start snaps to the result's
        keyframes (every 2 s) on every pointer move. Fix (0.9.1): while
        dragging, the handle follows the pointer and a thin mark shows the
        keyframe the cut will start on (the readout gives its time); on
        release the handle settles on the mark. Checked on the mock: the
        handle tracked the pointer in 2% steps, the mark stepped by
        keyframes, and the release landed on the mark.
      - After 0.9.1 (owner): the drag is smooth but the cut still starts on
        the 2-second keyframe; the end lands where it's put. Fix (0.9.2):
        an MP4 cut starts exactly at the handle, still copied. The copy
        keeps the packets from the keyframe before and the muxer's edit
        list starts playback at the start; checked in Chromium (Discord's
        desktop player and Sub20's window): a cut at 3.0 s showed the
        source's 3.0 s frame first (difference 0, against 12.7 for the 2 s
        keyframe). Matroska has no edit list, so an MKV result still snaps,
        and its seek went one keyframe too far back (a cut from 2 s kept
        from 0); it now seeks half a second past the keyframe. Both are e2e
        tests with real FFmpeg. Not yet seen: Discord's phone apps playing
        an edit-listed cut (iOS and Android players honour edit lists).
        Released as 0.9.2 (15 November 2026, 07:52 UTC): CI and the release
        build green on all three systems, ten signed feed entries, and the
        site deployed from the release on its own for the first time (the
        `v*` tag rule). The owner confirmed it in the real app on Windows
        the same day: the cut starts where the handle is. Not yet seen: the
        cut played in Discord on a phone.

      Report on the formats (owner, 15 November 2026): on a Windows
      computer Discord played H.264, HEVC and AV1 in the chat; on a phone
      H.264 and HEVC, not AV1. So "Discord won't play HEVC" was wrong.
      Fix (0.9.3): Options lists all three with where each plays, H.264
      still the default; the dock, the guides and the FAQ say "may not
      play". Released as 0.9.3 the same day.
- [ ] **3.7 Words in one place.** Moved to plan 6 (6.5, with the
      translations) on 15 November 2026, not done: the beta runs long, as
      foreseen below. Move the window's visible strings into
      one module, so a translation later is one file. No behaviour changes,
      and no translation yet. Do it after the beta's fixes have settled, so
      it doesn't collide with them; if the beta runs long, it moves to
      plan 6 with the translations.
      *Done when:* svelte-check is clean and both colour schemes look the
      same as before in screenshots.

## Needs the owner and testers

### The owner's Windows sitting

Most of what is left needs no tester, only the owner's Windows machine and
an hour or two. In one sitting, with `docs/testing.md` open:

- Follow the sheet once from the start, noting any step that needed
  asking about (3.1).
- Load and compress a recording an hour or longer, with Task Manager open
  for memory, and time it against the estimate (3.3).
- Turn on a watch, save a clip while watching, click the notice while it
  is on screen (3.4), and play a full-screen and a borderless game while a
  clip is saved (the folder watch line below).
- Turn on NVDA and go through the window (the screen reader line, Windows
  half).
- If a game records in HDR, compress one such clip (the HDR line).

From friends, without joining the beta: an OBS recording with the
microphone on its own track, and a video from an iPhone and from an
Android phone (3.2 and plan 4's 4.1).

### By hand, in an installed build

Each line is ticked with who tried it, on what, and when.

- [x] **Windows, the full pass:** Copy then paste into Discord, drag-out,
      the title bar (snap, maximise, a second monitor), Replace the
      original, renaming a finished file, cancel, quit.
      The owner, Windows 11 with an RTX 5070 Ti, 15 November 2026: works.
- [ ] **macOS (Apple Silicon):** the same pass, plus the first launch
      through Open Anyway, video playback in the preview, and
      VideoToolbox doing the encode.
- [ ] **Linux:** the same pass on Ubuntu (deb) and on Fedora (rpm) or Arch
      (AppImage), plus preview playback and VAAPI.
- [ ] **An Intel and an AMD card:** Quick Sync and AMF encode a clip under
      the limit; Copy diagnostics from each goes into the record. Wanted,
      not required: without a tester on one, its encoder stays
      "unverified" in `CLAUDE.md` and the CPU fallback covers it. The
      same holds for VideoToolbox in the macOS line and VAAPI in the Linux
      line: the rest of those lines is required, the encoder is not.
- [ ] **The folder watch on Windows:** "Watch NVIDIA app clips" and "Watch
      Xbox Game Bar clips" in Settings, a clip saved while watching, the
      title bar's status, History's list, and the notice: shown on the
      desktop, held during a full-screen and a borderless game, then one
      summary afterwards.
- [ ] **A real HDR clip** (an iPhone, or a game recorded in HDR): ordinary
      colour in the result, and the stage says it was HDR.
- [ ] **Opening videos from the system:** Open with; "Compress with Sub20"
      on several selected videos in Explorer (they arrive together, in name
      order); a video opened while Sub20 is running; Finder's Open With
      with several files; a file manager on Linux.
      Windows Open with: works (the owner, 15 November 2026). Left: macOS
      and Linux.
- [ ] **Quit, diagnostics, crash:** the quit question while compressing and
      Copy diagnostics on macOS and Linux (Windows: done 13 November 2026);
      a crash leaves `crash.log` and the diagnostics include it.
- [ ] **A screen reader:** NVDA on Windows and VoiceOver on macOS reach and
      name everything; focus stays inside an open sheet; the size picker
      and the segmented choices work by keyboard.
- [x] **The download without FFmpeg, on a machine with none:** carried over
      from plan 2 if it wasn't done there.
      A friend of the owner, Windows, 15 November 2026: the first screen
      offered Download FFmpeg, it installed, and clips compressed.

### The beta

- [ ] **Three to five testers,** at least one not on NVIDIA and one on a
      Mac, for a week or more. Find them with a short beta post in one or
      two places (a Discord server the owner is in, and a small subreddit),
      drafted from plan 5's 5.7 and read by the owner first. Send them the
      release page and `docs/testing.md`.
      15 November 2026: one tester so far. Was ten testers on three GPU
      makers for two weeks, lowered by the owner the same day.
- [ ] Ask each for the size before and after of a clip they really sent,
      and whether their words may be quoted. Plan 5 uses them.
- [ ] **Nitro's limits in bytes,** if someone has Nitro: the largest file
      that uploads at 50 MB, 500 MB and 1 GB, the way the free limit was
      found. The 500 MB figure is the unconfirmed one.

## Exit criteria

- [ ] Every line under "By hand" is ticked, or has an issue that is closed,
      except the hardware encoders other than NVENC (the Intel and AMD
      line, VideoToolbox, VAAPI), which may stay "unverified".
- [ ] Three to five testers, at least one not on NVIDIA and one on a Mac,
      over a week or more, with no open bug that loses data and no open
      crash.
- [ ] A fresh Windows machine goes from the download to a clip in Discord
      without reading anything beyond the SmartScreen step.
- [ ] An update from one release to the next worked on Windows and on a
      deb or rpm install.
