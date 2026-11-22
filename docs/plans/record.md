# The record: built and measured so far

What was done before the plans were split (13 November 2026), with the
numbers worth keeping. `CLAUDE.md` says how each thing works; this file
says that it was done, how it was checked, and what was measured. New
measurements from the plans are added at the end of their section.

## Settled first

- **Licence: MIT** (12 November 2026). It used to declare CC BY-NC 4.0,
  which isn't an open-source licence and keeps package managers away. No
  permission was needed: what Sub20 shares with the project that inspired
  it is functional only, and an 8-word overlap check found no shared prose
  or code. The bundled FFmpeg (GPL) ships as a separate program beside the
  app, with its licence and its source.
- **Discord's free limit: 20 MiB** (20,971,520 bytes). On 12 November 2026
  a free account uploaded a file of 20,971,000 bytes and refused one of
  20,972,000. The Nitro figures have not been checked the same way.
- **AV1 plays in Discord's chat** (owner's test, November 2026). HEVC
  didn't, in that test. A later test (15 November 2026) on a Windows
  computer played all three, and on a phone H.264 and HEVC but not AV1.
- **The updater key** was made on 12 November 2026, backed up by the
  owner, and added to GitHub as `TAURI_SIGNING_PRIVATE_KEY`.
- **No code signing, no Intel Macs** (owner, November 2026).

## The desktop app

Built, and how far each was checked:

| What | Checked |
| --- | --- |
| Several sound tracks mixed into one | e2e test (a silent "game" track and a tone "mic" track); not yet on a real OBS or NVIDIA recording |
| HDR tone-mapped to SDR | A generated PQ clip: saturation 87 against the SDR original's 114; untreated it was 23 |
| Watch shortcuts for the NVIDIA app and Xbox Game Bar | Mock backend only |
| The watch made visible (title bar, History, notices, the pile at launch) | Mock backend (`?watch`, `?watch=pile`), both schemes; notices unit-tested only |
| Leave a clip out of Compress | Mock backend, both schemes |
| FFmpeg bundled, built by Sub20 with its source published | Release dry run; installed and used on Windows by the owner |
| "Download FFmpeg" on the download without it | Mock backend only |
| Auto-update, one key per kind of install (ten keys) | Dry run: `latest.json` has all ten, each with its own signature; no real update yet |
| "Open with" and "Compress with Sub20" | Open with, one video, on Windows (owner). 20 clips launched one process each in random order all arrived in name order (release build) |
| Quit question, Copy diagnostics, crash log | Windows release test copy: quit and diagnostics work (5,275 characters, home folder as `~`); a crash not tried |
| Clear messages for a full disk, a read-only folder, a vanished or corrupt file | Tests; disk full by unit test only |
| Accessibility (keyboard, labels, contrast, reduced motion) | Audited on the mock backend, both schemes; no screen reader yet |
| The last options remembered; filmstrips one frame at a time; the About line | Built and checked on the mock backend |

### Real clips

- **NVIDIA app:** nine of the owner's clips over 20 MB (H.264 and AV1, SDR,
  one sound track, up to 1 min 40 s) all came out under 20 MB, at 88 to
  99.7% of the limit. `h264_nvenc` landed 17% and 6% over on two short AV1
  sources and the retry fixed both. A clip recorded at a nominal 240 fps
  averaging 60 kept 60 fps.
- **More NVIDIA app clips, desktop engine (14 November 2026):** an HEVC
  2560x1600 clip (28.6 s) came out at 18.37 MB for 20 and 8.49 MB for 10;
  a 30 s 1080p60 recording of 218 MB at 19.29 MB; Black Myth: Wukong
  recordings of 2 min 20 s (740 MB) at 18.86 MB, scaled to 1080p, and of
  2 min 49 s (902 MB) at 19.37 MB, scaled to 720p, softer but with the
  colours intact. All `h264_nvenc`, first try, 5 to 9 s each.
- **Sub20 Lite on the same machine's clips** (Edge 157 headless, GPU
  encoder, constant bitrate, 20 MB, 14 November 2026): 12 of 12 under on
  the first try, at 65.1 to 99.1% of the limit: eight 4.5 to 21 s clips
  from The Finals (AV1 and H.264, 1080p30 and 60), a 59 s AV1 aim-trainer
  clip, an 82 s AV1 clip and two 30 s 1080p60 recordings of 123 and
  218 MB. 93 s for all twelve. The 10 MB run of the HEVC clip landed at
  96.9%.
- **A generated 2-hour 4K file** (3.5 GB) came out at 491 MB for a 500 MB
  limit (1080p, `h264_nvenc`, 3.9% over on the first try, fixed by the
  retry) in 4.5 minutes. At 20 MB the plan is refused: "it holds 37 minutes
  at most".

### Memory and CPU (Windows 11, release build, 13 November 2026)

Sub20 and all its WebView2 processes summed, 30 s each:

| State | Processes | Working set | Private | CPU (one core) |
| --- | --- | --- | --- | --- |
| Idle, empty bench (from 8 s after launch) | 7 | 388 MB | 232 MB | 0.47% |
| Idle, 20 clips loaded | 8 | 475 MB | 270 MB | 0.05% |
| Minimised, 20 clips loaded | 8 | 473 MB | 268 MB | 0.05% |

Sub20's own process against WebView2's, in a release test copy:

| State | Sub20 CPU | Sub20 wake-ups/s | Sub20 file ops/s | WebView2 CPU |
| --- | --- | --- | --- | --- |
| Idle, watch off | 0.03–0.1% | 34 | 0 | 0.16–0.8% |
| Watch on 2,000 files, 5 s polls, before the fix | 2.58% | 45 | 3,749 | 0.21% |
| The same, after the fix | 0.23–0.26% | 57–100 | 13 | 0.6–3.1% |

- Sub20's own process is about 40 MB; the rest is WebView2. Minimising
  frees none of it, so running without a window means destroying the
  window (plan 7).
- The watch used to open every file on every poll. It now trusts the
  directory listing for files it has already handled.
- During a compress (a 20-minute 720p clip to 20 MB, 25 s on NVENC): Sub20
  0.3–0.5% of one core, FFmpeg about 6 cores, WebView2 53–59%, down to
  32–35% once the progress bars moved by transform instead of width.
- Adding and removing 20 clips five times: private memory 376, 384, 374,
  393, 381 MB. No growth.
- A network share was not measured.

### Closing the window (Windows 11, release test copy, 20 November 2026)

Plan 7's 7.1. A temporary change kept the process when the last window
closed and built the window again from its config on a second launch; it
was removed afterwards. Private bytes summed over the process and every
process under it, sampled every 3 s (30 s unless noted). The window was
closed with `WM_CLOSE`, as the close button or Alt+F4 sends.

| State | Processes | Private |
| --- | --- | --- |
| Open, 20 clips loaded (filmstrips made) | 8 | 261 MB |
| Closed, after the 20 clips | 1 | 8 MB |
| Reopened, empty bench, cycles 1 to 5 (6 s each) | 7 | 213–243 MB |
| Closed after each cycle (6 s each) | 1 | 8–9 MB |
| Closed after five cycles | 1 | 8 MB |
| An empty Tauri window (same Tauri, same machine) | 7 | 156 MB |

- **WebView2 exits with the window:** all its processes are gone within
  15 seconds of closing, and five cycles leave nothing behind.
- **Sub20's own process is 8 MB private.** The "about 40 MB" above was
  not private bytes; with no window the whole of Sub20 is that one
  process, far under plan 7's 100 MB.
- **WebView2's own floor is about 156 MB**, so Sub20's window adds about
  60–90 MB to it on an empty bench and about 105 MB with 20 clips.
- Not measured here: CPU, and a watch running (both 7.10, below).

### In the tray with a watch on (Windows 11, release test copy, 22 November 2026)

Plan 7's 7.10. A release build (`tauri build --no-bundle`) under the test
identifier, started with `--tray` (7.9), watching a folder of 2,000
one-second clips (all already there, so the watch baselined them and then
checked the listing every 5 s), bench empty. Private bytes summed over
the process and every process under it, sampled every 3 s for 60 s; CPU
from the processes' total processor time over the same 60 s, as a share
of one core. The window was opened by a second plain launch and closed
with `WM_CLOSE`, 6 s open each time.

| State | Processes | Private | CPU (one core) |
| --- | --- | --- | --- |
| Started in the tray (from 30 s after start) | 1 | 6–7 MB | 0.03% |
| Window open, six times | 7 | 214–245 MB | not measured |
| Closed, after one open | 1 | 8–9 MB | 0.07% |
| Closed, after six opens | 1 | 9–10 MB | 0.08% |
| The same, four more 30 s windows | 1 | 10 MB | 0.05–0.21% |

- **Within the budget** of 100 MB and 0.5% of one core: a tenth of the
  memory and under half the CPU.
- The watch made about 12 file operations a second (13 after the fix of
  13 November), so it was scanning; no file was opened (0 reads).
- Memory grew by about 1 MB per window cycle (7, then 9, then 10 MB) and
  wasn't watched for longer; 7.1 measured 8 MB after five cycles.
- The copy ran from Claude's side, inside the Claude app's package, where
  file and registry writes can be virtualized; that doesn't change what a
  process uses.

## Releases and CI

- **CI** runs fmt, clippy with warnings as errors, the tests with FFmpeg
  installed, svelte-check, both builds, `npm audit` and `cargo deny`, on
  Windows, macOS and Ubuntu.
- **The release workflow passed a dry run** on 13 November 2026: every
  download, a signed update entry for each, and the checksums. It uses
  plain `tauri build` and Sub20's own scripts, because `tauri-action` can't
  give two kinds of download their own update entries.
- **Timings:** from an empty cache, about 12 to 13 minutes. With the cache
  saved from `main`, about 5 to 6 minutes: compiling falls to 20–80 seconds
  per system, and packaging is most of what's left.
- **Sub20's own FFmpeg** (9.0.2, revision 1) is published with every source
  archive and the build scripts as the pre-release `ffmpeg-9.0.2-1`. The
  archives are 36 MB (Windows), 25 MB (Linux) and 19 MB (macOS).

Download sizes on a dry run:

| File | Sub20 | Without FFmpeg |
| --- | --- | --- |
| Windows installer | 31 MB | 2.8 MB |
| macOS disk image | 32 MB | 3.7 MB |
| Linux deb | 39 MB | 4.9 MB |
| Linux rpm | 39 MB | 4.9 MB |
| Linux AppImage | 110 MB | 79 MB |

The AppImage is large because it carries its own WebKitGTK and GTK; the
deb and rpm use the system's. With general FFmpeg builds the Windows
installer had been about 97 MB.

## Sub20 Lite

Built in November 2026: the shared planner as WebAssembly (about 170 kB),
the backend on WebCodecs and Mediabunny, the capability flags, the
manifest, service worker and Android share target, the browser check, the
phone layout (checked at 375×812, both schemes). The page is about 155 kB
before the first video.

**First size test** (12 November 2026: Windows 11, RTX 5070 Ti, Chromium
156, 20 MB, six generated clips with moving noise: 720p30, 1080p60, a
2-minute 1080p60, 1440p60 HEVC, 4K30, and a 1080×1920 HEVC clip):

| Bitrate mode | Under on the first try | First try lands at |
| --- | --- | --- |
| Constant | 6 of 6 | 97.7–99.3% of the limit |
| Variable | 2 of 6; the rest needed one retry | up to 105% |

GPU encodes ran at 2 to 15 times real time. A 105 MB 1080p60 clip came out
at 19.4 MB in about 11 s.

Two traps found and fixed in `fit.ts`:

- **The frame rate has to be passed** to the encoder; without it, 60 fps
  clips came out at about twice the target.
- **The GPU can refuse a size the browser says it supports.** This GPU's
  H.264 encoder takes 1440p30 but not 1440p60; the browser then quietly
  used its software encoder, which wouldn't go below about 4.7 Mbps (three
  attempts at 26.5 MB). Lite now asks with the real frame rate and steps
  down to a size the GPU takes.

## The website

The home page was built on 13 November 2026 (plan 1 has the details). The
screenshots under `site/public/img` were taken from the app on the mock
backend.
