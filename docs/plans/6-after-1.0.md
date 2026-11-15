# Plan 6: after 1.0

**Goal.** Hold everything that 1.0 doesn't need, in the order it should be
picked up, so the plans before this one stay small.

**Status.** Parked until 1.0.0 is stable. Nothing here is started before
plan 5 is finished, unless the owner asks for one item by name.

This is a list, not yet a plan. Before any item is built, it gets tasks
with "Done when" lines here (or a plan file of its own if it is large),
the way plans 1 to 5 are written.

## In this order

### 6.1 Package managers

Each is also a place people find software. One at a time, as each is
accepted:

1. winget
2. Scoop (extras bucket; "Sub20 without FFmpeg" can depend on Scoop's
   `ffmpeg`)
3. Flathub (they build it themselves)
4. AUR

### 6.2 Code signing

Moved to plan 3's task 3.8 on 15 November 2026: the owner wants the
Windows installer signed for 1.0. macOS is no longer built for.

### 6.3 The folder watch without a window

"Keep watching when the window is closed": off by default, offered only
when a watch is set up. Measured in November 2026 ([record.md](record.md)):
the window's WebView2 holds about 350 MB even when minimised, and Sub20's
own process about 40 MB. So closing must destroy the window, not hide it,
and build it again when opened.

- A tray menu: Open, Pause watching, Quit.
- Watched encodes at below-normal priority, so they don't cost frames in a
  game.
- Windows first; trays on Linux are patchy.
- Start with the system (owner, 15 November 2026): an option, under the
  watch in Settings, to open Sub20 into the tray when you sign in, so a
  watch runs without being started by hand. Off by default; it only makes
  sense once the tray exists.
- **Budget, to be met before it ships:** under 100 MB and under 0.5% of one
  core with a watch on and no window.
- About the memory (15 November 2026): the 388 MB is summed working set,
  which counts the libraries the seven processes share once in each; the
  private figure (232 MB) is the fairer one. Most of it is WebView2's own
  floor, not Sub20, and is usual for apps built on a browser engine. Two
  things to measure when this starts: an empty Tauri window on the same
  machine, to split WebView2's floor from Sub20's share, and WebView2's
  `MemoryUsageTargetLevel` (low while the window is in the background),
  which may trim the window's share while it is open.

### 6.4 Smaller things asked for or left over

- Open a watched clip from a click on its notice, if plan 3's task 3.4
  moved here.
- Pick sound tracks per clip, if anyone asks. Options are global and
  tracks differ per file, so it would sit beside the trim.
- A top-level "Compress with Sub20" on Windows 11 (today it is under "Show
  more options"); it needs a packaged `IExplorerCommand` handler.
- A "Send to" shortcut, if Explorer's menu turns out to behave badly with
  many files.
- Lite: VP9 in WebM for a browser that encodes neither H.264, AV1 nor
  HEVC (moved from plan 4's 4.9: it needs a place in the codec picker),
  and HDR closer to the desktop's brightness if a browser ever hands
  over the decoded frames' values.

### 6.5 New features

- Crop and aspect (9:16 for phones).
- Translations. First the window's visible strings move into one module,
  so a translation is one file, with no behaviour change: svelte-check
  clean and both colour schemes the same in screenshots (moved here from
  plan 3's task 3.7 on 15 November 2026).
- Merge several clips into one.
- GIF output for short clips.

## Not planned

By the owner's decision: a login, an Android app, storing uploads.
