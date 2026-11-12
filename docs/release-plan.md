# Sub20 release plan

Goal: a free, open-source 1.0 that anyone can install in one click, a
browser version for people who can't or won't install anything, a website
that ranks for "video too big for Discord", and a launch that gets it in
front of the people who send clips every day.

Each phase has exit criteria. A phase is done when they all hold, not when
its list is ticked.

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
  HEVC. Keep H.264 as the default (it already is), label HEVC "smallest
  file, but Discord won't play it in chat", and check whether AV1 plays on
  Discord desktop, Android and iOS before recommending it.
- **The limit in bytes.** Discord's API reference gives 20 MiB
  (20,971,520 bytes). Sub20 counts 1 MB as 1024 × 1024 bytes, which
  matches. Confirm by uploading one file of 20,971,000 bytes and one of
  20,972,000 bytes from a free account. Check the Nitro figures the same
  way, since the 500 MB one is unconfirmed.
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
- [ ] HDR sources: tone-map to SDR, so they don't come out grey.
- [ ] Watch-folder shortcuts for the NVIDIA app, Steam and Xbox Game Bar
      clip folders (detected, one click to turn on).

**Installing and updating**
- [ ] FFmpeg bundled. See [Downloads](#downloads).
- [ ] When FFmpeg is missing (the lean build): a clear first-run screen that
      offers "Download FFmpeg" (pinned URL, SHA-256 checked) or shows how to
      install it.
- [ ] Auto-update with `tauri-plugin-updater`, signed. One update feed per
      build, so a bundled install stays bundled.
- [ ] "Open with Sub20" for video files, and a "Compress with Sub20" entry
      in Explorer's context menu (NSIS installer).
- [ ] Code signing. See [Signing](#signing).

**Small gaps that look broken to a stranger**
- [ ] Ask before quitting while jobs are running.
- [ ] Remember the last options between launches.
- [ ] Load filmstrip thumbnails one at a time, so they don't all appear
      together after a wait.
- [ ] Disk full, a read-only folder, a file that vanishes mid-job, a corrupt
      file: each one gives a message that names the fix.
- [ ] Very long and very large sources (a 2-hour 4K file): the planner,
      memory use, the time estimate.
- [ ] **Copy diagnostics** in Settings: the app and FFmpeg versions, the
      encoder report, the operating system, and the last job's log, ready
      to paste into a bug report. No telemetry, by design. Say so.
- [ ] Panics and crashes are written to a log file the diagnostics include.
- [ ] Accessibility: keyboard reaches everything, controls have labels,
      reduced motion is respected, contrast holds in both themes.
- [ ] About box: version, licence, credits, the FFmpeg licence, "not
      affiliated with Discord".

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
      Discord without reading anything.

## Engineering and release infrastructure

Built during Phase 1, because every later phase uses it.

**CI** (GitHub Actions, on every push and pull request)
- `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace` (with
  FFmpeg installed so the e2e tests run), `svelte-check`, and both front-end
  builds.
- On Windows, macOS and Ubuntu.
- `cargo deny` (licences and advisories) and `npm audit --omit=dev`.

**Release workflow** (on a `v*` tag, using `tauri-action`)
- Builds two variants for each of three systems. Uploads to a draft release
  with SHA-256 checksums, the updater signatures and `latest.json` for each
  variant. Release notes come from `CHANGELOG.md`.
- One version number in `Cargo.toml`, `package.json` and `tauri.conf.json`,
  bumped by one script.

### Downloads

| Download | Who it's for | Size |
| --- | --- | --- |
| **Sub20** (FFmpeg included) | Everyone. The big button. | ~60–100 MB |
| Sub20 Lean (no FFmpeg) | Package managers, people who already have FFmpeg | ~10 MB |

- Start with a pinned FFmpeg release build (BtbN for Windows and Linux, a
  signed arm64 and x86-64 build for macOS). Later, build FFmpeg in CI with
  only the codecs, filters and formats Sub20 uses, which roughly quarters
  its size.
- Ship FFmpeg as a Tauri `externalBin` sidecar rather than a plain
  resource, so it is signed and notarised with the app on macOS. Update the
  bundled-FFmpeg lookup in `src-tauri/src/lib.rs` to match.
- Put `FFMPEG-LICENSE.txt` and the exact source URL and version in the
  bundle and in the About box.

### Signing

- **Windows:** apply to SignPath Foundation (free for OSI-licensed
  projects; publishing a code signing policy page is a condition). Without
  signing, SmartScreen warns "Windows protected your PC" and many people
  stop there. The fallback is Azure Artifact Signing, about $10 a month.
- **macOS:** notarisation needs an Apple Developer account ($99 a year).
  Without it, Gatekeeper blocks the app. Either pay before the macOS
  release, or label macOS "beta, unsigned" with the right-click → Open
  steps until donations cover it.
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
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, a code signing policy (SignPath
  requires one).
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
    (from `CHANGELOG.md`), **Code signing policy**.
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
| 1 | Check the Discord facts (licence done: MIT) | — |
| 2 | CI, repository files, version script | — |
| 3 | Phase 1 "Must have" items | 2 |
| 4 | Bundled FFmpeg, updater, release workflow | 2 |
| 5 | Lite prototype (go or no-go) | — (runs in parallel with 3) |
| 6 | Apply for SignPath (needs the MIT licence, now in place) | — |
| 7 | Closed beta of desktop 0.9 | 3, 4 |
| 8 | Lite build (if 5 passed) | 5, 2 |
| 9 | Website and domain | 7 for real screenshots |
| 10 | Signed 1.0, Lite, site live → launch sequence | 6, 7, 8, 9 |

Steps 1, 2, 5 and 6 can start today. SignPath reviews applications by
hand, so send that one early.
