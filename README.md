<p align="center"><img src="public/icon.svg" width="96" alt="Sub20 icon"></p>

# Sub20

Trim a clip and get it under 20 MB, ready for Discord.

I kept cutting clips and re-encoding them by hand to fit under Discord's upload
limit, so I made one app that does both. Drop videos in, trim each one with two
handles, pick a size, and every result comes out under it. Encoding runs on the
GPU when there is one (NVIDIA, Intel, AMD or Apple), and each result is
measured: one that lands even a byte over the limit is encoded again until it
fits. Hence the name: whatever you send comes out sub 20.

Sub20 is a native desktop app: a Rust engine that drives FFmpeg, in a
[Tauri](https://tauri.app) window. Its bitrate and resolution rules and its
FFmpeg settings for each encoder come from
[8mb.local](https://github.com/JMS1717/8mb.local) by JMS1717, a self-hosted web
app (Python, Celery, Redis and SvelteKit in Docker) that inspired it.

## What it does

- **Fits a size.** Bitrates are worked out from the size limit and the length
  of the kept part. Each result is measured, and one that lands over the limit
  is encoded again at a lower bitrate (up to two more times) until it fits.
- **Trim before compressing.** Step through the videos; each has a filmstrip
  timeline with a left and a right handle. Only the part between them is
  compressed, and the bitrate budget is spent on that part alone.
- **H.264, HEVC and AV1** on NVIDIA NVENC, Intel Quick Sync, AMD AMF, Windows
  Media Foundation, Apple VideoToolbox, Linux VAAPI, or the CPU (x264, x265,
  SVT-AV1). Every encoder is tested with a real encode at startup. Jobs use
  the best one that works and fall back to the CPU if a GPU encoder fails
  mid-job.
- **Automatic resolution.** When the limit is too tight for the source
  resolution, the picture is scaled down a step or two rather than smeared.
  Portrait videos are treated the same as landscape ones.
- **A queue** with live progress, time left, per-video cancel, and the FFmpeg
  log one click away.
- **Options**: resolution, frame-rate cap, Opus/AAC/no sound, speed against
  quality, MP4 or MKV, fixed bitrate, sound only, saved profiles.
- **Watch a folder** and compress whatever appears in it.
- **History** of what was compressed and what it shrank to.

## Requirements

- [FFmpeg](https://ffmpeg.org) 6 or newer, with `ffprobe`. The app looks on
  `PATH`, in the usual install folders, and in a folder you choose in
  Settings. To ship FFmpeg inside the app, see [Bundling FFmpeg](#bundling-ffmpeg).
- To build: [Rust](https://rustup.rs) 1.80+, Node.js 20+, and the
  [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system
  (WebView2 on Windows, Xcode command line tools on macOS, WebKitGTK 4.1 on
  Linux).

## Build and run

```bash
npm install
npm run tauri dev      # run with live reload
npm run tauri build    # make an installer for this system
```

`npm run tauri build` writes installers to `target/release/bundle/` (`.msi`
and `.exe` on Windows, `.dmg` and `.app` on macOS, `.deb`, `.rpm` and
`.AppImage` on Linux).

To work on the interface alone, `npm run dev` opens it in a browser at
`http://localhost:1420` with invented clips and simulated encodes.

### Bundling FFmpeg

Put `ffmpeg` and `ffprobe` (with `.exe` on Windows) in `src-tauri/ffmpeg/` and
add this to the `bundle` section of `src-tauri/tauri.conf.json`:

```json
"resources": ["ffmpeg/*"]
```

The app prefers a bundled FFmpeg over one on `PATH`. Check the licence of the
FFmpeg build you ship: builds with x264 and x265 are GPL.

## Using it

1. Drop videos on the window, or click to choose them. You can also start the
   app with files: `sub20 clip1.mp4 clip2.mov`.
2. For each video, drag the yellow handles to keep the part you want. The line
   under the timeline says what the result will be.
3. Click the size at the bottom left to change it.
4. Press **Compress**.

Results are saved next to the originals as `name-10mb.mp4` (or in one folder,
if you set that in Settings). Originals are never changed.

| Key | Does |
| --- | --- |
| Space | Play or pause the kept part |
| ← → | Move the playhead 0.1 s (with Shift, 1 s) |
| I, O (or [ and ]) | Set the start or end at the playhead |
| ↑ ↓ | Previous or next video |
| Delete | Remove the video from the list |
| Ctrl/⌘ + Enter | Compress |

With a trim handle focused, the arrow keys move that handle.

## How it is put together

```
crates/core     sub20-core: the engine. No GUI code.
  probe.rs        read a file's facts with ffprobe
  plan.rs         size limit → bitrates, resolution, frame rate
  encoders.rs     the 21 encoders the app knows
  hardware.rs     test which of them work here
  command.rs      build the FFmpeg command line
  engine.rs       the queue: run, report, cancel, fall back, retry
  watch.rs        folder watch
  thumbs.rs       filmstrip and preview frames
  settings.rs     settings and profiles (JSON)
  history.rs      history (JSON)
src-tauri       the desktop shell: commands, events, file access
src             the interface (Svelte 5, TypeScript)
```

The engine talks to the world through one trait, `EventSink`, so it can be
used from a command-line tool or a server as easily as from the window.

## Tests

```bash
cargo test --workspace
```

Unit tests cover the planner, command builder, probe parser, settings, history
and folder scan. `crates/core/tests/e2e.rs` runs real FFmpeg encodes: fitting a
target, trimming, HEVC, AV1, two-pass, the over-the-limit retry, cancel,
CPU fallback, queue order, and the folder watch. Those are skipped when FFmpeg
is not installed.

## How it differs from 8mb.local

- **A desktop app, not a server.** No upload step, no Docker, no Redis, no
  login. Files are read and written where they are.
- **Trimming is visual.** The original takes start and end times as text.
- **Results are held to the limit.** The original accepts a file up to 2% over
  and suggests sizes like 9.7 MB to leave room. Here any overshoot triggers
  another encode, so the quick sizes are round numbers. (1 MB is 1024 × 1024
  bytes, as in the original.)
- **"Best" effort keeps the limit.** The original's "extra quality" preset
  switches to constant-quality mode, which ignores the size. Here the top
  setting uses the slowest preset and, on x264 and x265, two passes, and still
  aims at the size.
- **Audio gives way on tight budgets.** If the sound would take more than a
  third of the bitrate, it is lowered; if the limit is impossible for the
  length, the app says so instead of producing a file over the limit.
- **Hardware decoding** is a single switch that asks FFmpeg for any available
  decoder and retries on the CPU if it fails. The original has
  NVIDIA-specific and QSV-specific decode paths.
- **Not carried over:** the Android app, the login, memory-backed upload
  storage, and file retention (there are no uploaded copies to clean up; the
  equivalent setting here is how long history is kept).

## What has and has not been tested

The engine was first built and tested on Linux x86-64 with FFmpeg 6.1: all unit
and end-to-end tests, and the app itself launched, trimmed and compressed real
files. On Windows 11 with FFmpeg 9.0 the unit tests pass and the checks are
clean, but on an NVIDIA GPU the NVENC encoder can overshoot small size limits
and the result is kept anyway: a known bug, being fixed. The other hardware
encoder paths (Quick Sync, AMF, Media Foundation, VideoToolbox, VAAPI) follow
8mb.local's FFmpeg flags but have only been exercised through their
failure-and-fallback path so far. The app window has not been run on Windows
or macOS yet.

## Coming next

- FFmpeg bundled in the app, so it works straight after install.
- A cleaner interface.
- Downloads on the [Releases](../../releases) page, with and without FFmpeg
  bundled.

## Licence

8mb.local is published under CC BY-NC 4.0 (attribution, non-commercial). Sub20
follows its bitrate and resolution rules and its FFmpeg flags for each encoder,
so it carries the same terms for now. Read that licence before distributing
this or using it commercially, and keep the credit to JMS1717.
