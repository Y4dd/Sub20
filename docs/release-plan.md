# Sub20 release plan

Goal: a free, open-source 1.0 that anyone can install in one click, a
browser version for people who can't or won't install anything, a website
that ranks for "video too big for Discord", and a launch that gets it in
front of the people who send clips every day.

Each phase has exit criteria. A phase is done when they all hold, not when
its list is ticked.

## Waiting on the owner

Things only the maintainer can do, at their own machine or account.

- [x] **Back up the updater key.** `~/.tauri/sub20-updater.key` (made
      12 November 2026, no password) signs every update. Keep a copy
      somewhere safe and offline, such as a password manager. If it's lost,
      installed copies can never update again; if it leaks, someone else
      can sign updates.
- [x] **Add it to GitHub** so the release workflow can sign:
      `gh secret set TAURI_SIGNING_PRIVATE_KEY --repo Y4dd/Sub20 < ~/.tauri/sub20-updater.key`
- [x] **Dry run** the release workflow: passed on 13 November 2026, all 8
      downloads, a signed update entry for each and the checksums, in
      about 12 minutes.
- [ ] **Try a Windows build with FFmpeg included** from a dry run's
      artifacts: install it, compress a clip, check Settings says FFmpeg is
      included.
- [ ] **Ask a friend to try the lean build** on their machine, ideally one
      without FFmpeg: install `Sub20-Lean`, check the first screen offers
      "Download FFmpeg", let it download, and compress a clip. The download
      only works from a tagged release (it fetches from that version's
      release page), so do this with the first release, not a dry run.
- Set aside by the owner (November 2026): **SignPath** (it asks for a
  website and proof of reputation) and an **Apple Developer account**.
  Releases are unsigned for now; see [Signing](#signing).
- [x] **AV1 plays inline in Discord**: the owner sent an AV1 clip made
      with Sub20 and it played in the chat (November 2026).

## Blockers to clear first

These decide what is allowed later, so they come before any code.

### 1. The licence: settled

Sub20 is **MIT** (`LICENSE`, `Cargo.toml`, `package.json`), as of 12 November
2026. It used to declare CC BY-NC 4.0, which isn't an open-source licence,
rules out free SignPath signing, and keeps package managers away.

No permission was needed. What Sub20 shares with the project that inspired
it is functional only: bitrate arithmetic, resolution and bitrate ladders,
FFmpeg flags, encoder option names and file extensions. Ideas, formulas and
settings aren't protected by copyright. An 8-word overlap check of both
codebases found no shared prose or code.

The README credits the inspiration in one line, and nothing else mentions
it.

**Licence choice:** MIT is the simplest "use it for anything" licence.
Choose GPL-3.0 instead only if you want forks to stay open. The bundled
FFmpeg (GPL, because of x264 and x265) ships as a separate program beside
the app, so it does not decide Sub20's own licence. We still have to ship
its licence text and point to its exact source.

### 2. Discord facts the app depends on

- **HEVC does not play inline in Discord.** Users report a frozen picture
  with sound on desktop; viewers have to download the file. Sub20 offers
  HEVC. H.264 stays the default, and HEVC now says Discord won't play it in
  the chat (done).
- **AV1 plays inline in Discord**, checked by the owner with a clip made
  in Sub20 (November 2026). H.264 stays the default because it plays on
  every device; AV1's note says it needs a recent device to play.
- **The tiers** are 20 MB free, 50 MB Nitro Basic, 500 MB with a server
  boost and 1 GB Nitro, per the owner (November 2026). The presets and the
  picker's names follow them.
- **The limit in bytes: confirmed.** Discord's API reference gives 20 MiB
  (20,971,520 bytes), and Sub20 counts 1 MB as 1024 × 1024 bytes. On
  12 November 2026 a free account uploaded a file of 20,971,000 bytes and
  refused one of 20,972,000. Still to check the same way: the Nitro
  figures, since the 500 MB one is unconfirmed.
- **Brand.** Discord's name can describe what Sub20 is for ("for Discord"),
  but the logo and colours can't be used. Add "Not affiliated with Discord"
  to the README, the site and the About box.

## Phase 1: the desktop app, release-ready

### Must have for 1.0

**Hands-on testing**
- [ ] A full pass on Windows in the real app: Copy then paste into
      Discord, drag-out, the title bar (snap, maximise, multiple
      monitors), Replace the original, renaming on disk, the folder watch,
      cancel, quit.
- [ ] The same pass on macOS (Apple Silicon) and Linux (Ubuntu, plus Fedora
      or Arch).
- [ ] At least one Intel and one AMD GPU, through beta testers.
- [ ] Built and checked only on the mock backend, with generated files or
      from the command line, so far:
  - [ ] The watch shortcuts ("Watch NVIDIA app clips", "Watch Xbox Game Bar
        clips") in the real window, and a clip saved while watching.
  - [ ] A real HDR clip (an iPhone, or a game recorded in HDR): it should
        come out in ordinary colour, and the stage should say it was HDR.
  - [ ] A long real recording (an hour or more): memory use, the time
        estimate, Page Up and Page Down on the trim handles.
  - [ ] A screen reader pass (NVDA on Windows, VoiceOver on macOS), and the
        keyboard changes in the real window: focus kept in sheets, the size
        picker, arrow keys in segmented choices.

**Clip sources gamers actually use**
- [x] **Multiple audio tracks.** OBS and the NVIDIA app often record game
      and microphone as separate tracks, and Sub20 kept only the first, so
      the microphone was lost. Every track is now mixed by default ("Mix all
      sound tracks" in Options → More turns it off), and the stage says
      "2 sound tracks, mixed".
  - [ ] Picking tracks per clip, if anyone asks for it. Options are global
        and tracks differ per file, so this would sit beside the trim.
  - [ ] Sub20 Lite keeps only the first track for now; mixing there needs
        its own audio step.
  - [ ] Try it on a real OBS and NVIDIA app recording.
- [ ] Test files from the NVIDIA app (Instant Replay), Steam Game
      Recording, Xbox Game Bar, OBS (MKV, several tracks), Medal, an iPhone
      (HEVC, HDR, rotated) and an Android phone (variable frame rate).
  - [x] NVIDIA app: nine of the owner's clips over 20 MB (H.264 and AV1,
        SDR, one sound track, up to 1 min 40 s) all came out under 20 MB, at
        88–99.7% of the limit. `h264_nvenc` landed 17% and 6% over on two
        short AV1 sources and the retry fixed both. A clip recorded at a
        nominal 240 fps averaging 60 kept 60 fps. Tried with
        `cargo run -p sub20-core --example compress -- FILE...`.
  - [ ] The rest, and an NVIDIA clip with the microphone on its own track.
- [x] HDR sources: tone-map to SDR, so they don't come out grey. PQ and HLG
      are read from the transfer characteristics and converted with
      `zscale` and the Mobius curve, then tagged BT.709. Checked on a
      generated PQ clip: colour saturation 87 against the SDR original's
      114, where the untreated result had 23. Needs an FFmpeg built with
      zscale (the bundled one is); without it the clip compresses and the
      stage warns that it may look grey.
- [x] Watch-folder shortcuts: Settings offers "Watch NVIDIA app clips"
      (`Videos\NVIDIA`, with subfolders) and "Watch Xbox Game Bar clips"
      (`Videos\Captures`) when those folders exist, new clips only. Steam
      isn't offered: it keeps recordings as stream segments, not video
      files, and saves an exported clip wherever the player chooses.
- [ ] **Make the folder watch visible** (proposed November 2026, not
      settled). Today the watch runs only while Sub20 is open, catches up
      at the next launch on what arrived while it was closed, and shows
      its jobs only in History and Settings. The direction discussed:
  - A title-bar indicator, only while a watch is on, with a count while
    it works; it opens History focused on the watch (folder, status, the
    job in progress, recent results with Copy and the drag-out chip).
  - A system notification when a watched clip is ready ("clip.mp4 is
    ready, 18.4 MB"); clicking it opens that result with Copy ready. No
    prompt per file.
  - Watched clips stay off the bench until the user reaches for one
    (owner agreed, November 2026). The bench is the user's working set,
    under the dock's size and options; clips arriving on their own would
    crowd it mid-trim and blur what "Clear finished" covers. The watch
    view's rows have Copy and the drag-out chip for a quick hand-off.
    Clicking a notification, or "Open" on a row, puts that clip on the
    bench as a finished clip and selects it, so the existing finished
    state does the rest: Original/Result preview, Copy, drag-out,
    renaming, and the trim (editing it puts the clip back to ready, for
    cutting a 30-second instant replay down and sending it again).
  - **Never while in a game** (owner's note: most users are gamers).
    Before showing one, ask Windows whether the user is busy
    (`SHQueryUserNotificationState`: a full-screen Direct3D game,
    presentation mode, quiet hours) and hold it if so; borderless
    windowed games don't always count as busy, so also hold it while a
    full-screen window that isn't Sub20 has the focus. Held notices
    become one summary ("3 clips are ready") once the user is back on
    the desktop. Never flash the taskbar or take the focus. macOS:
    respect Focus and Game Mode the same way.
  - At launch, when several clips (more than five, say) piled up while
    Sub20 was closed, ask once: compress them, or skip them.
  - Later, only after the memory and performance profile below: "Keep
    watching when the window is closed" (off by default, offered only
    when a watch is set up), closing the window rather than hiding it so
    the browser engine isn't kept running, with a tray menu (Open, Pause
    watching, Quit) and watched encodes at below-normal priority so they
    don't cost frames in a game. Windows and macOS first; Linux trays
    are patchy.

**Installing and updating**
- [x] FFmpeg bundled. See [Downloads](#downloads). Built by the release
      workflow; not yet tried on a fresh machine.
  - [x] Ship the GPL source for the bundled FFmpeg. Sub20 builds its own
        FFmpeg from pinned sources (`scripts/ffmpeg-build`,
        `.github/workflows/ffmpeg.yml`) and publishes the binaries with
        every source archive and the build scripts on one pre-release,
        [ffmpeg-9.0.2-1](https://github.com/Y4dd/Sub20/releases/tag/ffmpeg-9.0.2-1)
        (13 November 2026). A written offer was ruled out: GPL 3 allows it
        only for software shipped in a physical product.
- [x] When FFmpeg is missing (the lean build): a clear first-run screen that
      offers "Download FFmpeg" (pinned URL, SHA-256 checked) or shows how to
      install it. The archive comes from Sub20's own FFmpeg release (the
      one the full download carries), is checked against
      `scripts/ffmpeg.json`, unpacked with the system's bsdtar and
      installed in the app's local data folder. Checked on the mock backend
      only; try it for real on the first release.
- [x] Auto-update with `tauri-plugin-updater`, signed. One `latest.json`
      with a key per build (`windows-x86_64`, `windows-x86_64-lean`, …), so
      a bundled install stays bundled. Checked at startup (Settings can turn
      it off); a link in the title bar says when one is out.
  - [x] Add the `TAURI_SIGNING_PRIVATE_KEY` secret and back up the key.
  - [ ] Try an update from one real release to the next.
- [x] "Open with Sub20" for video files, and a "Compress with Sub20" entry
      in Explorer's context menu (NSIS installer hooks; never the default
      player). macOS and Linux declare the video types too. A video opened
      while Sub20 is running joins the open window (single instance).
  - [ ] Try them on each system from an installed build: Open with, the
        Explorer entry with several videos selected, Finder, and a video
        opened while Sub20 is running.
- [x] Code signing: set aside for 1.0 (owner, November 2026). Windows and
      macOS builds ship unsigned. See [Signing](#signing).
  - [x] First-launch steps for unsigned builds in the README (SmartScreen,
        Gatekeeper); the download page repeats them.

**Small gaps that look broken to a stranger**
- [x] Ask before quitting while jobs are running.
- [x] Remember the last options between launches.
- [x] Load filmstrip thumbnails one at a time, so they don't all appear
      together after a wait.
- [x] Disk full, a read-only folder, a file that vanishes mid-job, a corrupt
      file: each one gives a message that names the fix (`advice.rs`).
      Disk full is covered by unit tests only.
- [x] Very long and very large sources (a 2-hour 4K file): the planner,
      memory use, the time estimate. A generated 2-hour 4K file (3.5 GB)
      came out at 491 MB for 500 MB (1080p, `h264_nvenc`, 3.9% over on the
      first try, fixed by the retry) in 4.5 minutes. At 20 MB the plan is
      refused with how long a clip fits ("it holds 37 minutes at most").
      The FFmpeg log is capped at its last lines. Page Up and Page Down move
      a trim handle a twentieth of the video.
- [x] **Copy diagnostics** in Settings: the app and FFmpeg versions, the
      encoder report, the operating system, and the last job's log, ready
      to paste into a bug report, with the home folder hidden. No
      telemetry, by design, and the About line says so.
- [x] Panics and crashes are written to a log file the diagnostics include.
  - [ ] Try the quit dialog, Copy diagnostics and a crash in the real
        window on each system.
- [x] Accessibility: keyboard reaches everything, controls have labels,
      reduced motion is respected, contrast holds in both themes. Audited
      on the mock backend in both schemes (every text at 4.5:1 or more,
      every control named). Sheets keep Tab inside and return focus to
      what opened them; the size picker works by keyboard and says which
      size is chosen; segmented choices move with the arrow keys.
- [x] About box: version, licence, the FFmpeg licence, "not affiliated
      with Discord" (the end of Settings).
- [ ] **Memory and performance profile**, before anything that keeps Sub20
      running in the background (such as minimise to tray for the folder
      watch). Measure on Windows, the release build, at least:
  - Idle with an empty bench, and idle with a few clips loaded: memory
    (working set and private bytes, the WebView2 processes included), CPU
    and wake-ups per second.
  - Minimised and hidden: does the window stop animating and repainting,
    and does CPU drop to near zero?
  - The folder watch idling on a large folder (the NVIDIA folder) and on
    a network share: CPU and disk per poll, memory over an hour.
  - During a compress: Sub20's own share against FFmpeg's, and whether
    progress events or filmstrips make the window busy.
  - Memory after many clips added and removed, and over a long session
    (thumbnails, preview players, history), to catch leaks.
  Write the numbers here, set a budget for the background case (for
  example under 100 MB and under 0.5% CPU while hidden), and fix what
  misses it before the watch can run without the window.

### After 1.0 (only once 1.0 is stable)

- Crop and aspect (9:16 for phones).
- Translations. Put the strings in one place before 1.0 so this is cheap
  later.
- Merge several clips into one.
- GIF output for short clips.

### Exit criteria

- [x] The licence is settled and a `LICENSE` file is in place.
- [ ] Every "Must have" item is ticked.
- [ ] Ten or more beta testers on at least three GPU vendors, over two
      weeks or more, with no data-loss bugs and no open crash bugs.
- [ ] A fresh Windows machine goes from download to a finished clip in
      Discord without reading anything beyond the SmartScreen step.

## Engineering and release infrastructure

Built during Phase 1, because every later phase uses it.

**CI** (GitHub Actions, on every push and pull request)
- `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace` (with
  FFmpeg installed so the e2e tests run), `svelte-check`, and both front-end
  builds.
- On Windows, macOS and Ubuntu.
- `cargo deny` (licences and advisories) and `npm audit --omit=dev`.

**Release workflow** (`.github/workflows/release.yml`, on a `v*` tag)
- Done; a dry run passed (13 November 2026). Builds two variants (FFmpeg included, lean) for
  Windows x64, macOS Apple Silicon and Linux x64. Intel Macs were dropped
  in November 2026 (owner's decision): Apple stopped selling them in 2023,
  macOS 26 is the last version for them, and they cost a release job and
  an FFmpeg build of their own. Uploads to a
  draft release with `SHA256SUMS.txt`, the updater signatures and one
  `latest.json`. Release notes come from `CHANGELOG.md`. Plain
  `tauri build` and our own scripts rather than `tauri-action`, which
  can't give two variants their own update entries.
- Run by hand, it builds everything without releasing (a dry run).
- One job per system compiles once and packages both variants from the
  same program, which tells them apart at run time by the FFmpeg beside it.
  Measured from an empty cache (13 November 2026): eight jobs took 12
  minutes and 58 runner-minutes; four jobs with the lighter release profile
  took 13 minutes and 35 runner-minutes. Windows sets the pace: 7 minutes to
  compile from scratch and 3 to compress FFmpeg into the installer. With the
  cache saved from `main`, the same build took 6 minutes and 15
  runner-minutes: compiling fell to 20–80 seconds per system, and packaging
  is now most of it (Windows compressing FFmpeg, 2.5 minutes; Linux making
  the AppImage, deb and rpm twice, 3 minutes).
- One version number in `Cargo.toml`, `package.json` and `tauri.conf.json`,
  bumped by one script.

### Downloads

| Download | Who it's for | Size |
| --- | --- | --- |
| **Sub20** (FFmpeg included) | Everyone. The big button. | 32 MB (Windows installer) |
| Sub20 Lean (no FFmpeg) | Package managers, people who already have FFmpeg | ~10 MB |

- Done (13 November 2026): Sub20 builds its own FFmpeg in CI with only the
  codecs, filters and formats it uses, and publishes it with its source.
  The archives are 36 MB (Windows), 25 MB (Linux) and 19 MB (macOS).
  Before, the full download carried BtbN's and Martin Riedl's general
  builds. Measured on a dry run the same day: the Windows installer went
  from about 97 MB to 32 MB; everything one system's job makes (both
  downloads) went from 130 to 76 MB on macOS and from 590 to 289 MB on
  Linux (AppImage, deb and rpm, twice). With a warm cache the whole
  release now takes about 5 minutes; Windows packaging fell from 2.5
  minutes to seconds.
- Done: FFmpeg 9.0.2 ships as `externalBin` sidecars named
  `sub20-ffmpeg` and `sub20-ffprobe` (plain names would clash with the
  distribution's FFmpeg in `/usr/bin` on Linux), pinned with SHA-256 in
  `scripts/ffmpeg.json`. Its GPL text and origin ship in
  `resources/ffmpeg/`, linked from the About line in Settings.

### Signing

**Decision (owner, November 2026): no code signing for now.** SignPath
asks for a website and proof of the project's reputation, which Sub20
doesn't have yet, and the Apple account costs $99 a year. The cost is a
warning on first launch, which the README explains. Updates are signed
either way: that's the updater key, not code signing.

- **Windows:** SmartScreen says "Windows protected your PC" until a build
  earns reputation; **More info → Run anyway** opens it. Some people stop
  there. Later, when there's a site and some downloads: SignPath Foundation
  (free for OSI-licensed projects, needs a code signing policy page), or
  Azure Artifact Signing at about $10 a month.
- **macOS:** builds are ad-hoc signed, not notarised, so Gatekeeper blocks
  the first launch. On macOS 15 and later, **System Settings → Privacy &
  Security → Open Anyway** opens it (right-click → Open no longer does).
  Later: an Apple Developer account, once donations cover it.
- **Linux:** no signing needed. Flathub builds it themselves.

### Package managers

After 1.0, in this order:
1. winget
2. Scoop (extras bucket; the lean build can depend on Scoop's `ffmpeg`)
3. Homebrew cask
4. Flathub
5. AUR

### Repository

- `LICENSE`, `CHANGELOG.md` (Keep a Changelog), `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, and a code signing policy if
  SignPath is taken up later (it requires one).
- Issue templates for bugs (they ask for Copy diagnostics) and for feature
  requests. Discussions turned on. Labels `good first issue` and
  `help wanted`.
- `FUNDING.yml` (GitHub Sponsors or Ko-fi). The app stays free; donations
  pay for the Apple account and the domain.
- A README that opens with a 20-second demo GIF, then the download button,
  then "Try it in your browser".

## Phase 2: Sub20 Lite (in the browser)

Same repository, same interface, a third backend. Nothing is uploaded: the
browser's own hardware encoder does the work through WebCodecs, and
Mediabunny reads and writes the files.

### Step 0: go or no-go prototype (first, about two days)

Encode 20 real clips (game captures, phone videos) to 20 MB with WebCodecs
H.264 in Chrome, Edge, Safari and Firefox. Record how far each result
lands from the target and how long it takes. If some browsers overshoot,
check whether one measured retry, as on the desktop, always brings them
under. **If results can't be held under the limit reliably, stop here**:
the promise is the product.

The test page is `/lite-lab.html` under `npm run dev` (`src/lab`,
`src/lib/web/fit.ts`). Pick videos and a size and press Run, then Copy
results.

**First results (12 November 2026).**
- **Setup:** Windows 11, RTX 5070 Ti laptop GPU, Chromium 156 (the Claude
  desktop app's browser), a 20 MB limit.
- **Clips:** six synthetic clips with moving noise: 720p30, 1080p60, a
  2-minute 1080p60, 1440p60 HEVC, 4K30, and a 1080×1920 HEVC phone-style
  clip.

| Bitrate mode | Under on the first try | First try lands at |
| --- | --- | --- |
| Constant | 6 of 6 | 97.7–99.3% of the limit |
| Variable | 2 of 6; the rest needed one retry | up to 105% |

Constant bitrate is the default for Lite. GPU encodes ran at 2–15× real
time.

Two traps found and fixed in `fit.ts`:
- **The frame rate has to be passed.** Mediabunny gives the encoder a frame
  rate only when one is set, and without it 60 fps clips came out at about
  twice the target.
- **The GPU can refuse a size the browser claims to support.** This GPU's
  H.264 encoder takes 1440p30 but not 1440p60, at any level, while
  Mediabunny's `canEncodeVideo` (which ignores the frame rate) says yes. The
  browser then quietly uses its software encoder, which wouldn't go below
  about 4.7 Mbps: three attempts all came out at 26.5 MB. Lite now asks
  WebCodecs with the real frame rate and steps down the resolution ladder to
  a size the GPU takes (1440p60 → 1080p60: 97.8% on the first try, 7 s
  instead of 2 minutes of misses). A retry that shrinks the file by less
  than 5% stops, as on the desktop.

Still to run before go:
- [ ] Real clips: NVIDIA app, OBS, iPhone (HDR), Android (variable frame
      rate).
- [ ] Browsers: Chrome, Edge, Firefox, Safari on macOS and iOS, Chrome on
      Android.
- [ ] GPUs: Intel and AMD.
- [ ] Results play inline in Discord, on desktop and on phones.
- [ ] A planner rule for Lite that knows the GPU's limits, instead of
      stepping down after planning.

### Build

- **Shared planner:** put tokio, uuid, trash and the engine behind a
  default `engine` feature in `sub20-core`. Compile `plan.rs` and its types
  to WebAssembly with a small `crates/plan-wasm` (wasm-bindgen). The web
  version uses the same rule and the same tests; `mock.ts` keeps its copy
  only for design work, or switches to the WASM build too.
- **`src/lib/web/`**: a `Backend` built on WebCodecs and Mediabunny.
  - Probe the file and draw the filmstrip.
  - Trim, scale and encode H.264 (fall back to VP9 or AV1 in WebM where
    H.264 isn't available).
  - Audio: AAC where the browser can encode it, else Opus.
  - Measure the result and retry at a lower bitrate if it's over, as on the
    desktop.
  - Write large results to the origin private file system, not memory.
- **Capability flags on `Backend`** (`watch`, `replace`, `renameOnDisk`,
  `encoders`, `ffmpeg`, `windowFrame`, `copyFile`, `dragOut`, `share`). The
  interface hides what's missing and swaps Copy for Download, or for
  **Share** on phones.
- **Phones are the point.** `navigator.share({ files })` hands the result
  straight to the Discord app on Android and iOS. Add a web app manifest and
  a service worker (it works offline and can be installed), and on Android
  a share target, so "Share → Sub20" from the gallery opens the clip. An
  iPhone video over 20 MB is the most common case this version solves.
- **Browser check** at startup with `VideoEncoder.isConfigSupported`. Where
  it fails, say which browser to use, and point to the desktop app.
- **Build:** `npm run build:web` (`VITE_TARGET=web`) writes `dist-web/`.

### Exit criteria

- [ ] The prototype passed.
- [ ] Results stay under the limit in Chrome, Edge and Safari (desktop and
      phone) on the test clips.
- [ ] A phone video goes from the gallery to Discord in under a minute.
- [ ] Lighthouse PWA and accessibility checks pass.

## Phase 3: the website

Lives in the same repository under `site/` (Astro, with the Svelte app
mounted at `/app`). Deployed by one Pages workflow next to Lite.

- **Domain:** buy one (a `.app` name or similar, about $15 a year). It is
  easier to share than `y4dd.github.io/Sub20`, it survives a change of
  host, and search engines treat it better. Use GitHub Pages; move to
  Cloudflare Pages only if Lite needs custom headers.
- **Pages:**
  - **Home**
    - Hero line: "Trim it. Fit it. Send it."
    - A looping demo video (encoded by Sub20, naturally).
    - Download for your system (detected), with the other systems and the
      Lean build beneath it.
    - "Try it in your browser".
    - Three facts: under the limit every time, on your GPU, nothing
      uploaded.
  - **Download:** both builds, checksums, and what to expect from
    SmartScreen or Gatekeeper if a build isn't signed yet.
  - **Lite:** `/app`.
  - **Guides** (these are what bring search traffic):
    - "Discord says your file is too large: what to do"
    - "Send an NVIDIA Instant Replay clip on Discord"
    - "Send an iPhone video on Discord"
    - "Why your HEVC video won't play on Discord"
  - **Compare:** honest tables against the browser compressors that
    upload your files, HandBrake (no size target) and LosslessCut (no size
    target).
  - **FAQ, Privacy** (no telemetry, nothing uploaded), **Changelog**
    (from `CHANGELOG.md`), and **Code signing policy** if SignPath is
    taken up.
- **Analytics:** privacy-friendly (GoatCounter or Cloudflare Web Analytics,
  no cookies, no banner), on the site only, never in the app.
- **Design:** the app's tokens, fonts and trim bracket, so the site and
  the app read as one thing.
  - Real screenshots and a real recording, not mock-ups.
  - Run the taste skill and its checker.
- Open Graph image and social cards, `sitemap.xml`, structured data
  (`SoftwareApplication`).

### Exit criteria

- [ ] Fast on a phone (Lighthouse 95+).
- [ ] Download links follow the latest release automatically.
- [ ] Every guide is accurate against the current app.

## Phase 4: launch and getting noticed

**Who it's for.** People who record clips and send them on Discord:
- PC gamers using the NVIDIA app, Steam, OBS or Xbox Game Bar.
- Everyone with a phone video that is "too large".

**The one-line pitch:** "Trim any clip and get it under Discord's limit:
free, open source, runs on your GPU, nothing uploaded."

### Before launch

- [ ] Closed beta in Discord servers you're already part of. Collect real
      before-and-after numbers and two or three quotes.
- [ ] A 20–30 second demo video: a clip that's too large, two handle drags,
      Compress, Ctrl+V into Discord. It's for the README, the site, Reddit
      and Shorts.
- [ ] Accounts with the same name on Bluesky, X, Reddit and YouTube.
      Check the name isn't taken by a similar product.

### Launch, spread over about two weeks

Different audiences need different angles, so don't post everywhere on one
day.

1. **Show HN** (Tuesday to Thursday, US morning).
   - Lead with the engineering: a strict size limit with a measured retry,
     GPU encoders that can't reach low bitrates, and the same planner
     compiled to WebAssembly for the browser.
   - Post a technical write-up the same day: "Hitting an exact file size
     with hardware encoders".
2. **Reddit:**
   - r/rust and r/tauri ("built with Tauri 2"), r/sveltejs
   - r/opensource, r/software, r/coolgithubprojects
   - r/discordapp, then the clip-heavy game communities
   - Read each subreddit's self-promotion rules first, post as a person
     rather than an ad, and answer every comment.
3. **Directories and lists:**
   - AlternativeTo (as an alternative to the online compressors, HandBrake
     and LosslessCut)
   - Product Hunt (once there's a polished page)
   - awesome-tauri, awesome-rust (applications), awesome-ffmpeg,
     awesome-svelte
   - Lite on awesome-selfhosted, if it can be self-hosted as static files
4. **Package managers** (winget and the rest) as each one is accepted. Each
   one is a discovery channel in its own right.
5. **Short videos:**
   - "Discord says file too large? Fix it in 5 seconds."
   - On YouTube Shorts and TikTok.
   - One per platform the app supports (PC, iPhone, Android).

### After launch

- Answer issues within two days for the first month, because early
  contributors stay when they're answered.
- Release every two to four weeks with a short changelog post.
- A public roadmap (GitHub Projects) and five or more `good first issue`
  tasks at all times.
- Watch:
  - GitHub stars
  - release downloads (GitHub API)
  - Lite visits
  - the questions people ask
  The last of these shows what the FAQ and the next release need.

## Order of work

| Step | Work | Depends on |
| --- | --- | --- |
| 1 | Discord facts: free limit and AV1 playback confirmed; Nitro sizes left (licence done: MIT) | — |
| 2 | CI, repository files, version script | — |
| 3 | Phase 1 "Must have" items | 2 |
| 4 | Bundled FFmpeg, updater, release workflow | 2 |
| 5 | Lite prototype (go or no-go) | — (runs in parallel with 3) |
| 6 | Code signing: set aside (owner, November 2026); SignPath once there's a site and some reputation | 9 |
| 7 | Closed beta of desktop 0.9 | 3, 4 |
| 8 | Lite build (if 5 passed) | 5, 2 |
| 9 | Website and domain | 7 for real screenshots |
| 10 | 1.0, Lite, site live → launch sequence | 7, 8, 9 |

Steps 1, 2 and 5 can start today.
