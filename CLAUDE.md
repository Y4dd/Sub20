# Sub20

A desktop app for getting clips onto Discord: drop videos, trim each with two handles, pick a size, compress. Every result comes out under the size limit, by default well under 20 MB, hence the name.

It exists because the owner was forever cutting clips and re-encoding them by hand to fit Discord's upload limit. Sub20 does both in one place: trim, then a GPU encode aimed at a size, checked after encoding and redone if it lands over.

Sub20 is a Tauri 2 desktop app with a Rust engine driving FFmpeg, free and open source under MIT (`LICENSE`).

**Licence and credit.** Sub20 was inspired by 8mb.local, and the README says so in one line at the end of its Licence section. That line is the only mention, by the owner's decision: don't add others, and don't describe Sub20 as following or deriving from another project. Licensing was settled in November 2026. What Sub20 shares with that project is functional only: bitrate arithmetic, resolution and bitrate ladders, FFmpeg flags, encoder option names, file extensions. Ideas, formulas and settings aren't copyrightable. An 8-word overlap check of both codebases found no shared prose or code, so no permission was needed. Write new code and wording from scratch, not from another project's source.

The owner's brief, which every change should still serve: written in Rust; a step to go through one or more videos and trim them with left/right handles before compressing; a modern, simple interface with fewer buttons. Chosen along the way: native desktop app, no login.

## Layers

Dependencies point one way: `crates/core` ← `src-tauri` ← `src`.

- **`crates/core` (the engine, crate `sub20-core`)** has no GUI code and holds all behaviour. It speaks to the outside through one trait, `EventSink`. Put logic here and test it here.
- **`src-tauri` (the shell, crate `sub20`, lib `sub20_lib`)** is thin: Tauri commands, event forwarding, file access. One command per `Backend` method.
- **`src` (the window)** is Svelte 5 with runes. It reaches the app only through the `Backend` interface in `src/lib/types.ts`, which has two implementations: `tauri.ts` (real) and `mock.ts` (invented clips and simulated encodes for browser work).

Adding a capability touches, in order: a core function with a test → a command in `src-tauri/src/lib.rs`, registered in `generate_handler!` → a `Backend` method in `types.ts` → both `tauri.ts` and `mock.ts` → the UI.

`types.ts` mirrors the Rust serde output by hand (camelCase fields, lowercase or camelCase enum variants, `JobEvent` tagged by `type`). Change the Rust type and its mirror in the same edit.

## Commands worth knowing

- `npm run dev` runs the window in a browser on the mock backend. Use it for design work.
- `npm run tauri dev` runs the real app. A plain `cargo build` debug binary also loads `http://localhost:1420`, so it needs the Vite server.
- `npx tauri build --debug --no-bundle` builds the way a release does (UI embedded, CSP enforced). Run this after touching `tauri.conf.json`, the CSP, or anything loaded by URL.
- `cargo test --workspace`: `crates/core/tests/e2e.rs` runs real FFmpeg encodes and skips itself when FFmpeg is missing.
- `cargo run -p sub20-core --example compress -- [--mb 20] [--encoder NAME] [--out DIR] FILE...` compresses real clips with the app's engine and prints size, encoder and attempts. History stays off.
- Before finishing: `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `npx svelte-check`. All three are clean today; keep them clean.
- The repo lives in a OneDrive folder on the owner's machine. Point `CARGO_TARGET_DIR` outside it so build output is not synced.
- CI (`.github/workflows/ci.yml`) runs fmt, `clippy -D warnings` and the tests on Windows, macOS and Ubuntu with FFmpeg installed, plus `svelte-check`, the build, `npm audit --omit=dev` and `cargo deny check advisories`. `deny.toml` fails "unmaintained" only for direct dependencies; the ones inside Tauri's tree (zbus, proc-macro-error) are Tauri's.
- `npm run set-version -- x.y.z` writes the version into `package.json`, `tauri.conf.json` and the workspace `Cargo.toml`, and refreshes both lock files.
- `/lite-lab.html` under `npm run dev` is the Sub20 Lite size test: it encodes chosen files with WebCodecs through `src/lib/web/fit.ts` and reports how close each lands to the limit. Dev only; neither build includes it. `lab.run(files)` runs it from a script.

## Engine decisions

**The limit is strict.** A result one byte over the target is encoded again at a lower bitrate (`lower_bitrate`), up to `SIZE_RETRIES` times. An over-limit file is never saved: `when_over` decides between another retry, a switch to the CPU encoder (hardware encoders only, with fresh retries from the planned bitrate), or an `OverLimit` error that names the fix. A hardware retry that shrinks the file by less than 5% means the encoder is at its floor, and the job moves to the CPU at once. Measured on an RTX 5070 Ti: `h264_nvenc` will not go under about 600 kbps on a busy 720p30 clip whatever the flags (691 kbps even at QP 51) and drifts 6–9% over larger targets (17% on a short clip from the owner's NVIDIA folder; the retry fixed it); `hevc_nvenc` and `av1_nvenc` hit low targets. Because nothing over is ever saved, the default sizes can be round (no 9.7 MB to leave room). 1 MB is 1024 × 1024 bytes. `KBIT_PER_MB = 8192` (size in MiB × 8192 = kilobits) leaves about 2.3% for container overhead.

**Planning is pure** (`plan.rs`), so the window calls it on every handle drag.
- Audio yields on tight budgets: at most 34% of the total, stepped down to a floor (Opus 24, AAC 48 kbps). If video would get under 24 kbps the plan is refused with advice, since the result could not fit; the advice says how long a clip the size holds ("20 MB is too small for 2 hours of video: it holds 37 minutes at most"), rounded down, with lengths in words from `timecode::spoken`.
- Automatic resolution picks the size whose result will look best, judged by `rate`: weighted kbps per megapixel (`density`: codec efficiency, frame rate) with 550 good and 220 rough, and a cap for pictures shrunk below 720p (soft) or 480p (rough). Good counts as good enough, ties go to the larger picture (to the smaller when all are rough), and the size is chosen with the same rounded kbps the verdict is given with. Choice and verdict share one measure, so a shorter trim never looks worse (`shorter_trims_never_look_worse`). This replaced a rule that preferred 1080p and judged by raw kbps, which turned "720p, good" into "1080p, soft" when a clip was trimmed shorter. Portrait behaves like landscape; the source size is a candidate too.
- The scale filter gets explicit even `W:H`, computed in the planner from display dimensions (rotation applied).
- A trim that keeps the whole video is treated as no trim.

**Trim** is `-ss` before `-i` and `-t` after it. Everything is re-encoded, so the cut is exact.

**Encoders.** The user picks a codec; `Report::best` picks the encoder, hardware first. `options.encoder` overrides. Detection encodes two real frames per encoder, because FFmpeg lists encoders whose GPU is absent; families run side by side, encoders within a family take turns (consumer GPUs cap sessions). A hardware failure mid-job falls back to the CPU encoder for that codec, ignoring the user's disabled list so the job finishes. `libaom-av1` is never automatic.

**Effort `Max`** keeps the size target: slowest preset everywhere, plus two passes on libx264 and libx265 only. Pass statistics use the encoder's default file names in the working directory, so each job runs FFmpeg from its own scratch folder; that avoids escaping Windows paths inside `-x265-params`. It never switches to constant quality, which would ignore the size.

**Hardware decode** is `-hwaccel auto` with frames returned to system memory, so the software filter chain is unchanged; on failure the job retries with CPU decoding.

**HDR** (PQ or HLG, read from `color_transfer` into `MediaInfo::hdr`) is tone-mapped to SDR first in the filter chain (`command::TONE_MAP`: `zscale` to linear, `tonemap` Mobius, back to BT.709), encoded 8-bit and tagged BT.709. Untreated, HDR came out grey (saturation 23 against the SDR original's 114 on a generated PQ clip); Mobius kept the most colour (87, against Reinhard 81 and Hable 64). It needs FFmpeg with zscale (`Report::tone_map`, from `ffmpeg -filters`); without it the job runs untreated, logs why, and the stage warns. Audio-only jobs skip it.

**Stream mapping** is `0:V:0` (skips cover art), the sound, `-sn -dn`. Opus in MP4 becomes AAC. HEVC in MP4 gets `-tag:v hvc1`.

**Several audio tracks are mixed** (`EncodeOptions::mix_audio`, on by default), because OBS and the NVIDIA app can record the microphone on a track of its own and keeping only the first lost it. `command::audio_input` makes each track stereo (`aformat`; a mono mic lands in the middle), sums them with `amix normalize=0` (the default divides each input by the track count, so the game came out quieter), and puts `alimiter` after it for the peaks the sum pushes over. One track, or mixing off, maps `0:a:0?` as before. `MediaInfo::audio_tracks` counts them; the other audio facts describe track 1. The e2e test makes a silent "game" track and a tone "mic" track and checks the mic is heard at full volume.

**Output.** FFmpeg writes `.{stem}.{id}.part.{ext}` in the destination folder; a `Cleanup` guard removes the partial file and scratch folder on every exit path. `output::place` moves it to its final name: a name the user typed (`JobSpec::output_name`, cleaned by `output::clean_name` for every system), else `stem-20mb.ext`, `stem-2500k.ext` or `stem-audio.ext`. `looks_generated` recognises the default names so the folder watch skips its own output: change the naming and that function together. A typed name is not recognised, so a manual result saved into a watched folder would be picked up.

**Replace the original** (`OutputMode::Replace`, sent as `JobSpec::replace_original`) saves the result under a free name first, then moves the original to the bin (`trash` crate, injected as a closure so tests use a fake bin), and only then renames the result to the wanted name; if the bin refuses, both stay and the job logs why. `JobResult::replaced_original` tells the window, which then hides Original, Compress again and the trim, since the source is gone. Manual jobs only: the watch treats Replace as `SameFolder` and `tidy()` resets a watch set to it. `Engine::rename_result` renames a finished file and retargets its history entry.

**Queue.** A job takes its place in line synchronously in `Engine::start`, before its task is spawned; spawn order is not run order on a multi-thread runtime, and that was a real bug. Automatic parallelism is `cores / 8`, clamped 1–3.

**Stopping FFmpeg** goes through `ffmpeg::kill_tree`. On Windows `ffmpeg.exe` may be a launcher (Chocolatey's ShimGen shims start the real FFmpeg as a child), and killing the launcher alone leaves FFmpeg running and writing; `taskkill /T /F` on the launcher's PID stops both. Timed-out helper runs do the same. Found on CI, where Chocolatey installs FFmpeg; Scoop's shims pass the kill on.

**Failures name the cause and the fix.** `advice::explain` rewrites a failed job's error before the `Failed` event: the source moved or deleted (checked first, since it explains what follows), the results folder gone, a full disk (FFmpeg's words or the OS error code), a folder Sub20 may not write to versus a source it may not read (FFmpeg names the `.part.` file when it is the output), and a source that can't be read as a video, which also covers a recording still being written. Anything else keeps FFmpeg's telling line. Add a case there, with a unit test, rather than rewording errors where they are raised.

**Progress** comes from `-progress pipe:1`; stderr is the log. `percent` is per attempt and restarts on `Retry`; two-pass splits it 0–50 and 50–100.

**Folder watch polls** on a timer by design: it behaves the same on network shares and synced folders, and "stopped changing for N seconds" answers "has the copy finished?". Its memory (`watch.json`) is keyed by path plus size and mtime; the "already there" baseline is taken once per folder. `watch::clip_folders` finds the NVIDIA app's (`VideosNVIDIA`, a folder per game) and Xbox Game Bar's (`VideosCaptures`) clip folders on Windows; Settings offers each as a one-click watch, new clips only. Steam is left out: its recordings are stream segments, not video files.

**Settings** deserialize with `#[serde(default)]` and pass through `tidy()`, so old or hand-edited files load. A damaged file is renamed to `settings.broken.json`. There is no file retention (no uploaded copies exist); the equivalent is `history_keep_days`. Settings live in the app data folder for the identifier `io.github.y4dd.sub20`.

**FFmpeg lookup order:** the folder in settings, bundled (`resources/ffmpeg`, resources, beside the executable), `PATH`, then usual install folders (GUI apps on macOS do not inherit the shell's `PATH`). In bundled folders `sub20-ffmpeg`/`sub20-ffprobe` come first: the release ships FFmpeg under those names, because a Linux package puts sidecars in `/usr/bin`, where `ffmpeg` would clash with the distribution's package.

## Shell decisions

- Commands that reach `Engine::submit` or `Watcher::reload` are `async`: both call `tokio::spawn` and need the runtime.
- The asset-protocol scope starts empty. Each added video is allowed at runtime in `add_videos`; the thumbnail cache folder is allowed at startup.
- **Linux preview playback** goes through `media.rs`, a loopback HTTP server (127.0.0.1, random port, random token, registered files only, range requests). WebKitGTK hands media to GStreamer, which cannot read the asset protocol. Windows and macOS use the asset protocol. The CSP's `media-src` allows `http://127.0.0.1:*` for this.
- A source the webview cannot decode fires `<video>`'s `onerror`, and the stage switches to still frames from `preview_frame`.
- **Quitting asks** while jobs are queued or running (`may_quit`, on `CloseRequested` and `ExitRequested`, so the custom close button, Alt+F4 and ⌘Q all ask): a native dialog, "Stop and quit" or "Keep compressing". On exit the shell cancels all jobs and waits briefly so no FFmpeg child outlives the app. Installing an update stops jobs without asking; its link says so ("Stop compressing, update and restart").
- **Crashes** are appended to `crash.log` in the app's log folder by a panic hook (`log_crashes`, restarted past 256 KB). **Copy diagnostics** (Settings, About) puts `sub20_core::diagnostics::report` on the clipboard through `clipboard-rs`: versions, build, system (`os_info`), FFmpeg, the encoder report, the crash log's end and the shown job's FFmpeg log, with the home folder written as `~` so it can be pasted in public. There is no telemetry; the update check is the only request the app makes by itself.
- **Filmstrips arrive one by one**: `thumbs::filmstrip` reports each frame through a callback, the `filmstrip` command forwards them over a Tauri `Channel`, and the window fills empty places (`""`) in `Clip::thumbs` as they come.
- Paths passed on the command line open as clips ("Open with Sub20").
- **Two builds.** The lean one (plain `tauri build`) finds FFmpeg on the system. The full one adds `tauri.bundled.conf.json`: FFmpeg as `externalBin` sidecars, fetched by `scripts/fetch-ffmpeg.mjs` from the pins in `scripts/ffmpeg.json` (BtbN for Windows and Linux, Martin Riedl's signed builds for macOS, SHA-256 checked), plus its GPL text and origin under `resources/ffmpeg/`. Never run those downloads on the owner's machine; CI fetches them.
- **Updates** use `tauri-plugin-updater` from Rust (`check_update`, `install_update`), one `latest.json` on the latest GitHub release. Each build asks for its own key, `{os}-{arch}` or `{os}-{arch}-lean` (`update_target`; the full build is compiled with `SUB20_BUNDLED=1`), so an install stays the kind it started as. The window checks at startup when `Settings::check_updates` is on, silently on failure. Installing stops running jobs first. The public key is in `tauri.conf.json`; the private key is `~/.tauri/sub20-updater.key` on the owner's machine and the `TAURI_SIGNING_PRIVATE_KEY` secret. Losing it means existing installs can't update.
- **Releases**: a `v*` tag runs `release.yml`: a draft release named from `CHANGELOG.md`, 8 builds (Windows x64 NSIS, macOS arm64 and x64 app/dmg, Linux AppImage/deb/rpm; each full and lean) with `tauri.release.conf.json` (updater artifacts, ad-hoc macOS signing), renamed by `scripts/collect-release.mjs` (`Sub20-Lean_*` for lean), then `latest.json` and `SHA256SUMS.txt`. Run by hand, it builds without releasing.
- **Results** are remembered in `AppState::results` as job `Done` events pass through the sink. `result_url` (preview) and `copy_file` (clipboard, via `clipboard-rs`; CF_HDROP on Windows) refuse any other path. Drag-out is `tauri-plugin-drag` called from the window, with a filmstrip thumbnail file as the drag image (`tauri.ts` keeps thumbnail URL → file).
- **Window frame** differs by platform: `tauri.windows.conf.json` turns decorations off and the window draws its own buttons (`Frame` "custom"); macOS keeps its traffic lights over an overlay title bar ("overlay", `titleBarStyle: Overlay`); Linux keeps the native frame ("native"). The title bar is `data-tauri-drag-region`; the window permissions it needs are listed in `capabilities/default.json`.
- Out of scope by decision: login, the Android app, upload storage.

## Interface decisions

The brief was "modern, intuitive, fewer buttons, sleek and simple". How that was answered so far:

- **One screen.** A title bar across the top (wordmark, History, Settings, and on Windows the window buttons), clip list, stage (preview, timeline, the plan or result row), dock (size, options summary, one action or the batch progress). Options, Settings and History are sheets that slide over it.
- **The trim bracket is the signature.** Yellow handles are the convention for the kept range in video editors. The bracket is the empty-state drop target, the timeline control, the app icon (with "20" inside), and the progress display (the kept range fills from the left while encoding). `--trim` is reserved for the bracket, the primary action, the selected-clip marker and switched-on toggles.
- **One primary button.** Everything else is a text link (`.link`) where it applies; the exceptions are the finished clip's file chip (drag it out, click to play) and Copy, because getting the file into Discord is the point. Icons are limited to history, settings, close, remove, rename (pencil), copy, the drag grip and the window buttons.
- **Size and options are global** to the list; **trim is per clip**. Editing a finished clip's trim puts it back to ready.
- **Copy:** sentence case, plain verbs, commas between facts. The status line says what will happen, is happening, or happened. Errors name the cause and the fix. An action keeps one name everywhere.
- **Getting it into Discord** is the end of every job: the finished state previews the result (Original/Result switch), offers Copy (Ctrl+C) and a file chip to drag out. The size picker names the Discord limit each preset matches (`discordTier` in `format.ts`; change it when Discord changes its plans). Presets live in settings as plain numbers: Discord's tiers, 20, 50, 500 and 1024 (`DISCORD_SIZES`), default 20 MB (`DEFAULT_TARGET_MB`). A saved list still equal to the old default (20, 50, 100, 500) moves to the new one in `tidy()`; an edited list is left alone. Whole gigabytes show as GB (`sizeParts`, `sizeName`). The picker names them Free, Nitro Basic, Server boost and Nitro (owner, November 2026). HEVC is labelled where it is picked and in the dock summary: Discord won't play it in the chat.
- **The last options** are saved to `Settings::last_options` a second after they stop changing and restored at launch, with the matching profile name when they equal one. `saveSettings` in the window runs one save at a time so two quick changes can't overwrite each other.
- **Tokens** live in `src/app.css`. Dark by default, light through `prefers-color-scheme`. Yellow on the light background is only 1.35:1, so marks drawn in yellow use `--trim-mark` and yellow shapes get a `--trim-edge` outline (both a deep amber in light, plain yellow / nothing in dark). Fonts are bundled (the app works offline): Bricolage Grotesque for display, condensed with `font-stretch`, and Hanken Grotesk for the interface, with tabular figures. Monospace appears only in the FFmpeg log. Corner radii differ by role.
- **Icon source** is `public/icon.svg`. Regenerate the app icons with `npx tauri icon public/icon.svg`, then delete the `android` and `ios` folders it creates.
- **State** lives in `src/lib/app.svelte.ts`. `Clip` is a class with `$state` fields. Job events can arrive before `compress()` returns its ids, hence the `#early` buffer.
- **Accessibility.** Text is at least 4.5:1 against its background in both schemes, so `--faint` is for marks and struck-through choices, not for words that carry meaning (a disabled toggle keeps `--muted` text and dims its switch). Sheets are modal: Tab stays inside, Escape closes, focus returns to the opener. `Segmented` is a radio group with one tab stop and arrow keys. Icon-only buttons carry `aria-label`.
- **Keyboard** handling is in `App.svelte`: arrow keys and vim motions in one interpreter (`vim()`), with counts and two-key commands (`gg`, `dd`, `cw`) that wait a second for their second key. Plain letters are safe because the handler ignores keys typed into fields and keys while a sheet is open. `?` opens `KeysSheet`; keep it, the README table and `vim()` in step.
- **Renaming** happens in the clip list (double-click, F2, `r`, or the pencil). Before compressing it sets `Clip::outputName`; afterwards it renames the file on disk through `rename_result`. A renamed clip shows its new name in the list; the stage keeps the source name.
- Judge interface changes by looking: run `npm run dev` and take screenshots in both colour schemes. `?clips` opens the demo clips at startup and `?frame=overlay|native` shows the other frames, so a headless browser can capture a state without clicking. The built-in browser pane's screenshots crop on a display scaled to 150%; a headless Edge or Chrome at a fixed size works. If a component's styles stop updating in `npm run dev`, restart it: Vite's per-component CSS can go stale.
- The mock planner in `mock.ts` copies the Rust rule for resolution and verdict; change them together. The Lite size test uses it too (`planFor`), and `fit.ts` copies `lower_bitrate` and `SIZE_RETRIES`, until the planner is built to WebAssembly.

## Planned next

`docs/release-plan.md` is the road to a public 1.0: blockers, desktop hardening, Sub20 Lite, the website and the launch, with exit criteria per phase. Keep it current as items land. Highlights:

- **The first release**: the release workflow, both builds and the updater exist but have not run end to end yet.
- **Sub20 Lite**: a third `Backend` on WebCodecs and Mediabunny (`src/lib/web/`), the planner shared through a WebAssembly build of `plan.rs`, capability flags to hide desktop-only features. Gated on the size test in `/lite-lab.html`.
- **Discord facts**: HEVC doesn't play inline in Discord, so H.264 stays the default and HEVC must say so; the free limit is 20 MiB, matching the app's 1024 × 1024 MB (confirmed by upload in November 2026: 20,971,000 bytes went through, 20,972,000 did not).

## Verified, and not

Verified on Linux x86-64 with FFmpeg 6.1 and no GPU (before the rename): 56 tests, and the real app launched in both dev and release-style builds, trimmed with a mouse, previewed, and compressed files under the limit.

Verified on Windows 11 with FFmpeg 9.0 and an RTX 5070 Ti laptop GPU: `cargo fmt`, `clippy` and `svelte-check` clean; 90 tests pass (71 unit, 17 e2e, 2 shell; the read-only-folder e2e test runs on Linux and macOS only), including NVENC jobs that start on the GPU. The interface was checked on the mock backend at 1180×780 and 900×620 in both schemes. The real app window, Copy into Discord, drag-out and the custom title bar have not been tried by hand on Windows yet.

E2e tests that check a particular encoder name it in `options.encoder`, and tests that depend on queue order set `parallel_jobs = 1`; a machine with a GPU or many cores otherwise changes what they observe. `a_hardware_encoder_that_misses_hands_over_to_the_cpu` covers the GPU path and skips itself without one.

Windows keeps a killed FFmpeg's output locked for a moment (error 32), so `Cleanup` removes files `patiently`, retrying for up to about five seconds (a CI runner's Defender held one for over a second).

Unverified, so check these first on a new machine:
- Hardware encoder success paths other than NVENC (Quick Sync, AMF, Media Foundation, VideoToolbox, VAAPI). Their flags are the common FFmpeg settings for each; only fail-and-fall-back has run.
- Windows and macOS app runs, including asset-protocol video playback and the `CREATE_NO_WINDOW` flag on FFmpeg children.

## Loose ends

- The bundled FFmpeg is GPL: the app ships its licence and origin, but the release doesn't carry the corresponding source yet.
- The quit dialog, Copy diagnostics and the crash log have only been built and type-checked, not tried in the real window.
