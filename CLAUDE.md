# Sub20

A desktop app for getting clips onto Discord: drop videos, trim each with two handles, pick a size, compress. Every result comes out under the size limit, by default well under 20 MB, hence the name.

It exists because the owner was forever cutting clips and re-encoding them by hand to fit Discord's upload limit. Sub20 does both in one place: trim, then a GPU encode aimed at a size, checked after encoding and redone if it lands over.

The engine follows [8mb.local](https://github.com/JMS1717/8mb.local) by JMS1717 (Python/Celery/Redis/SvelteKit in Docker): its bitrate and resolution rules and its per-encoder FFmpeg flags. Sub20 is a Tauri 2 desktop app with a Rust engine driving FFmpeg.

The owner's brief, which every change should still serve: written in Rust; a step to go through one or more videos and trim them with left/right handles before compressing; a modern, simple interface with fewer buttons. Chosen along the way: native desktop app, feature parity with the original's desktop side, no login.

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
- Before finishing: `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `npx svelte-check`. All three are clean today; keep them clean.
- The repo lives in a OneDrive folder on the owner's machine. Point `CARGO_TARGET_DIR` outside it so build output is not synced.

## Engine decisions

**The limit is strict.** A result one byte over the target is encoded again at a lower bitrate (`lower_bitrate`), up to `SIZE_RETRIES` times. An over-limit file is never saved: `when_over` decides between another retry, a switch to the CPU encoder (hardware encoders only, with fresh retries from the planned bitrate), or an `OverLimit` error that names the fix. A hardware retry that shrinks the file by less than 5% means the encoder is at its floor, and the job moves to the CPU at once. Measured on an RTX 5070 Ti: `h264_nvenc` will not go under about 600 kbps on a busy 720p30 clip whatever the flags (691 kbps even at QP 51) and drifts 6–9% over larger targets; `hevc_nvenc` and `av1_nvenc` hit low targets. The original accepted 2% over, which is why it suggested 9.7 MB; here the default sizes are round. 1 MB is 1024 × 1024 bytes. `KBIT_PER_MB = 8192` is the original's formula and leaves about 2.3% for container overhead.

**Planning is pure** (`plan.rs`), so the window calls it on every handle drag.
- Audio yields on tight budgets: at most 34% of the total, stepped down to a floor (Opus 24, AAC 48 kbps). If video would get under 24 kbps the plan is refused with advice, since the result could not fit.
- Automatic resolution works on the short side using kbps per megapixel (550 good, 350 fallback, 220 floor) over a 2160…240 ladder. Ported from the original with two fixes: portrait behaves like landscape, and a well-fed source of non-standard size keeps its size.
- The scale filter gets explicit even `W:H`, computed in the planner from display dimensions (rotation applied).
- A trim that keeps the whole video is treated as no trim.

**Trim** is `-ss` before `-i` and `-t` after it. Everything is re-encoded, so the cut is exact.

**Encoders.** The user picks a codec; `Report::best` picks the encoder, hardware first. `options.encoder` overrides. Detection encodes two real frames per encoder, because FFmpeg lists encoders whose GPU is absent; families run side by side, encoders within a family take turns (consumer GPUs cap sessions). A hardware failure mid-job falls back to the CPU encoder for that codec, ignoring the user's disabled list so the job finishes. `libaom-av1` is never automatic.

**Effort `Max`** keeps the size target: slowest preset everywhere, plus two passes on libx264 and libx265 only. Pass statistics use the encoder's default file names in the working directory, so each job runs FFmpeg from its own scratch folder; that avoids escaping Windows paths inside `-x265-params`. (The original's "extra quality" switched to constant quality and ignored the size.)

**Hardware decode** is `-hwaccel auto` with frames returned to system memory, so the software filter chain is unchanged; on failure the job retries with CPU decoding.

**Stream mapping** is `0:V:0` (skips cover art), `0:a:0?`, `-sn -dn`. Opus in MP4 becomes AAC. HEVC in MP4 gets `-tag:v hvc1`.

**Output.** FFmpeg writes `.{stem}.{id}.part.{ext}` in the destination folder, renamed on success; a `Cleanup` guard removes the partial file and scratch folder on every exit path. Names are `stem-10mb.ext`, `stem-2500k.ext`, `stem-audio.ext`. `looks_generated` recognises these so the folder watch skips its own output: change the naming and that function together.

**Queue.** A job takes its place in line synchronously in `Engine::start`, before its task is spawned; spawn order is not run order on a multi-thread runtime, and that was a real bug. Automatic parallelism is `cores / 8`, clamped 1–3.

**Progress** comes from `-progress pipe:1`; stderr is the log. `percent` is per attempt and restarts on `Retry`; two-pass splits it 0–50 and 50–100.

**Folder watch polls** on a timer by design: it behaves the same on network shares and synced folders, and "stopped changing for N seconds" answers "has the copy finished?". Its memory (`watch.json`) is keyed by path plus size and mtime; the "already there" baseline is taken once per folder.

**Settings** deserialize with `#[serde(default)]` and pass through `tidy()`, so old or hand-edited files load. A damaged file is renamed to `settings.broken.json`. There is no file retention (no uploaded copies exist); the equivalent is `history_keep_days`. Settings live in the app data folder for the identifier `io.github.y4dd.sub20`.

**FFmpeg lookup order:** the folder in settings, bundled (`resources/ffmpeg`, resources, beside the executable), `PATH`, then usual install folders (GUI apps on macOS do not inherit the shell's `PATH`).

## Shell decisions

- Commands that reach `Engine::submit` or `Watcher::reload` are `async`: both call `tokio::spawn` and need the runtime.
- The asset-protocol scope starts empty. Each added video is allowed at runtime in `add_videos`; the thumbnail cache folder is allowed at startup.
- **Linux preview playback** goes through `media.rs`, a loopback HTTP server (127.0.0.1, random port, random token, registered files only, range requests). WebKitGTK hands media to GStreamer, which cannot read the asset protocol. Windows and macOS use the asset protocol. The CSP's `media-src` allows `http://127.0.0.1:*` for this.
- A source the webview cannot decode fires `<video>`'s `onerror`, and the stage switches to still frames from `preview_frame`.
- On exit the shell cancels all jobs and waits briefly so no FFmpeg child outlives the app.
- Paths passed on the command line open as clips ("Open with Sub20").
- Out of scope by decision: login, the Android app, upload storage.

## Interface decisions

The brief was "modern, intuitive, fewer buttons, sleek and simple". How that was answered so far:

- **One screen.** Clip list, stage (preview, timeline, one status sentence), dock (size, options summary, one action). Options, Settings and History are sheets that slide over it.
- **The trim bracket is the signature.** Yellow handles are the convention for the kept range in video editors. The bracket is the empty-state drop target, the timeline control, the app icon (with "20" inside), and the progress display (the kept range fills from the left while encoding). `--trim` is reserved for the bracket, the primary action, the selected-clip marker and switched-on toggles.
- **One primary button.** Everything else is a text link inside the status sentence (`.link`), and icons are limited to history, settings, close and remove. A new action goes into the sentence where it applies before it becomes a button.
- **Size and options are global** to the list; **trim is per clip**. Editing a finished clip's trim puts it back to ready.
- **Copy:** sentence case, plain verbs, commas between facts. The status line says what will happen, is happening, or happened. Errors name the cause and the fix. An action keeps one name everywhere.
- **Tokens** live in `src/app.css`. Dark by default, light through `prefers-color-scheme`. Fonts are bundled (the app works offline): Bricolage Grotesque for display, condensed with `font-stretch`, and Hanken Grotesk for the interface, with tabular figures. Monospace appears only in the FFmpeg log. Corner radii differ by role.
- **Icon source** is `public/icon.svg`. Regenerate the app icons with `npx tauri icon public/icon.svg`, then delete the `android` and `ios` folders it creates.
- **State** lives in `src/lib/app.svelte.ts`. `Clip` is a class with `$state` fields. Job events can arrive before `compress()` returns its ids, hence the `#early` buffer.
- **Keyboard** handling is in `App.svelte`; the README has the table.
- Judge interface changes by looking: run `npm run dev` and take screenshots in both colour schemes.

## Planned next

The owner wants these; each is to be discussed before it is built.

- **FFmpeg bundled** in the app, so a fresh install works without a separate download.
- **A more tasteful interface and better UX** than the current one.
- **GitHub releases** with two downloads per platform: one with FFmpeg bundled, one without (uses the system FFmpeg).

## Verified, and not

Verified on Linux x86-64 with FFmpeg 6.1 and no GPU (before the rename): 56 tests, and the real app launched in both dev and release-style builds, trimmed with a mouse, previewed, and compressed files under the limit.

Verified on Windows 11 with FFmpeg 9.0 and an RTX 5070 Ti laptop GPU: `cargo fmt`, `clippy` and `svelte-check` clean; 60 tests pass (46 unit, 12 e2e, 2 shell), including NVENC jobs that start on the GPU. The app window itself has not been launched on Windows yet.

E2e tests that check a particular encoder name it in `options.encoder`, and tests that depend on queue order set `parallel_jobs = 1`; a machine with a GPU or many cores otherwise changes what they observe. `a_hardware_encoder_that_misses_hands_over_to_the_cpu` covers the GPU path and skips itself without one.

Windows keeps a killed FFmpeg's output locked for a moment (error 32), so `Cleanup` removes files `patiently`, retrying for up to a second.

Unverified, so check these first on a new machine:
- Hardware encoder success paths other than NVENC (Quick Sync, AMF, Media Foundation, VideoToolbox, VAAPI). Their flags follow the original; only fail-and-fall-back has run.
- Windows and macOS app runs, including asset-protocol video playback and the `CREATE_NO_WINDOW` flag on FFmpeg children.

## Loose ends

- Quitting while jobs run cancels them without asking.
- Filmstrip thumbnails arrive all at once, not progressively.
- The current options are not remembered between launches except as saved profiles.
- There is no LICENSE file. 8mb.local is CC BY-NC 4.0 and this engine follows its rules and flags; `Cargo.toml` declares the same licence. The owner decides the final terms.
