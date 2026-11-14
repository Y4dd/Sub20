# Plan 3: the desktop beta

**Goal.** The desktop app has been used by hand, in the real window, on
Windows, macOS and Linux and on NVIDIA, Intel and AMD cards; ten or more
people have used it for two weeks; and nothing they found that loses data
or crashes is still open. After this plan the desktop app is ready to be
called 1.0.

**Status.** Not started. The features for 1.0 are built
([record.md](record.md)). Most were checked on the mock backend, from the
command line, or on one Windows machine with an NVIDIA card. What has not
been tried by hand is listed in `CLAUDE.md` under "Verified, and not".

**Needs first:** plan 2 (a release to install). **Runs beside:** plan 4.
**Unblocks:** plan 5.

Most of this plan is other people trying things. The AI's part is to make
that easy (3.1), to test what can be tested from here (3.2, 3.3), and to
fix what comes back (3.6).

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
- [ ] **3.5 The first update.** When the first fix is ready, release 0.9.1
      the way plan 2 released 0.9.0. Then, on every kind someone can reach:
      0.9.0 says an update is out, installs it, restarts as 0.9.1, and
      stays the kind it was (with or without FFmpeg; deb stays deb, rpm
      stays rpm).
      *Done when:* seen on Windows and on at least one deb or rpm install.
- [ ] **3.6 Fix what the beta finds.** Every report becomes an issue. A bug
      that loses a file, saves one over the limit, or crashes is fixed
      before anything else, with a test where one can be written. Fixes go
      out as 0.9.x releases, a few at a time. Requests for new features go
      to plan 6.
      *Done when:* the exit criteria hold.
- [ ] **3.7 Words in one place.** Move the window's visible strings into
      one module, so a translation later is one file. No behaviour changes,
      and no translation yet. Do it after the beta's fixes have settled, so
      it doesn't collide with them; if the beta runs long, it moves to
      plan 6 with the translations.
      *Done when:* svelte-check is clean and both colour schemes look the
      same as before in screenshots.

## Needs the owner and testers

### By hand, in an installed build

Each line is ticked with who tried it, on what, and when.

- [ ] **Windows, the full pass:** Copy then paste into Discord, drag-out,
      the title bar (snap, maximise, a second monitor), Replace the
      original, renaming a finished file, cancel, quit.
- [ ] **macOS (Apple Silicon):** the same pass, plus the first launch
      through Open Anyway, video playback in the preview, and
      VideoToolbox doing the encode.
- [ ] **Linux:** the same pass on Ubuntu (deb) and on Fedora (rpm) or Arch
      (AppImage), plus preview playback and VAAPI.
- [ ] **An Intel and an AMD card:** Quick Sync and AMF encode a clip under
      the limit; Copy diagnostics from each goes into the record.
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
- [ ] **Quit, diagnostics, crash:** the quit question while compressing and
      Copy diagnostics on macOS and Linux (Windows: done 13 November 2026);
      a crash leaves `crash.log` and the diagnostics include it.
- [ ] **A screen reader:** NVDA on Windows and VoiceOver on macOS reach and
      name everything; focus stays inside an open sheet; the size picker
      and the segmented choices work by keyboard.
- [ ] **The download without FFmpeg, on a machine with none:** carried over
      from plan 2 if it wasn't done there.

### The beta

- [ ] **Ten or more testers**, from Discord servers the owner is already
      in, with NVIDIA, Intel and AMD cards among them, for two weeks or
      more. Send them the release page and `docs/testing.md`.
- [ ] Ask each for the size before and after of a clip they really sent,
      and whether their words may be quoted. Plan 5 uses them.
- [ ] **Nitro's limits in bytes,** if someone has Nitro: the largest file
      that uploads at 50 MB, 500 MB and 1 GB, the way the free limit was
      found. The 500 MB figure is the unconfirmed one.

## Exit criteria

- [ ] Every line under "By hand" is ticked, or has an issue that is closed.
- [ ] Ten or more testers on at least three GPU makers, over two weeks or
      more, with no open bug that loses data and no open crash.
- [ ] A fresh Windows machine goes from the download to a clip in Discord
      without reading anything beyond the SmartScreen step.
- [ ] An update from one release to the next worked on Windows and on a
      deb or rpm install.
