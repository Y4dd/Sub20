# Sub20: the plans to 1.0

Goal: a free, open-source 1.0 that anyone can install in one click, a
browser version for people who can't or won't install anything, a website
that ranks for "video too big for Discord", and a launch that gets it in
front of the people who send clips every day.

The road is split into six plans that run in order. Each plan is the source
of truth for its own work: what to build, in which order, and when it is
finished. What was built and measured before the split is in
[record.md](record.md).

| # | Plan | Delivers | Needs first | Status |
| --- | --- | --- | --- | --- |
| 1 | [Website](1-website.md) | The site under `site/`, deployed to GitHub Pages with Lite at `/app`, unannounced | nothing | Live at y4dd.github.io/Sub20 (15 November 2026); a subdomain of amjad.sh to come |
| 2 | [First release](2-first-release.md) | v0.9.0 on GitHub: every download, signed updates, the site's links following it | nothing (better after 1) | Done: v0.9.0 published, an install updated to it, the site's links follow it |
| 3 | [Desktop beta](3-desktop-beta.md) | The desktop app tried by hand on three systems and three GPU makers, a two-week beta, its fixes | 2 | Started: test sheet written, first beta reports fixed, Windows passed; v0.9.0 is out, so testers can install it; macOS, Linux, other GPUs and the folder watch wait on testers |
| 4 | [Sub20 Lite](4-lite.md) | Lite past its size test on real clips, browsers and phones, and its gaps closed or labelled | 1 (phones need it hosted) | Built, Lighthouse and axe clean, warnings and mixing in; not tested outside this machine |
| 5 | [1.0 and launch](5-launch.md) | The demo video, 1.0.0, the launch posts, the routine afterwards | 3, and 4 or a decision to launch without Lite | Not started |
| 6 | [After 1.0](6-after-1.0.md) | Package managers, code signing, the watch without a window, new features | 5 | Parked |

## Why this order

- **The website first.** It is half built and uncommitted, it needs nobody
  but the owner's one click to enable Pages, and two later plans stand on
  it: the release notes and README point to its download page, and Lite
  can only be tried on a phone once it is served over HTTPS.
- **The first release second.** Everything after it needs a real release:
  an update from one version to the next, a friend's install, beta testers,
  the site's download links. The dry run already passes, so this plan is
  short.
- **The beta third, Lite beside it.** Both wait on people and machines the
  AI can't reach, for two weeks or more. Plans 3 and 4 run side by side:
  when one is waiting on the owner or testers, work on the other.
- **Launch last,** once the beta has no open crash or data-loss bug.
- **Plan 6 holds everything that isn't needed for 1.0,** so the plans
  before it stay small.

## How to work from these plans

For the AI doing the work:

1. Work on the lowest-numbered plan that isn't finished, unless the owner
   names another. Plans 3 and 4 may be worked side by side.
2. Inside a plan, take the tasks in order. A task's number (`1.4`, `3.2`)
   is its name: use it in commits and when reporting.
3. Tick a task only when its "Done when" holds, and write beside the tick
   the date and what was checked. "Built" and "tried" are different: say
   which.
4. Tasks under **Needs the owner** can't be done from here (another
   machine, an account, a decision, a post). Don't wait on them: do every
   other task, then say plainly which owner tasks are open.
5. A new idea goes into the plan it belongs to, or plan 6, before any code
   is written for it. Don't grow a plan with work its goal doesn't need.
6. When a plan's exit criteria all hold, set its status in the table above
   to "Finished" with the date.
7. Decisions and facts that outlive a plan go into `CLAUDE.md`; numbers
   worth keeping go into [record.md](record.md).

## Rules that hold in every plan

`CLAUDE.md` has the architecture, the commands and the decisions. On top of
it:

- **Commit freely, push only on the owner's word.** Several tasks go into
  one push and one CI run; don't offer a push after each task.
- **Never download or run third-party FFmpeg binaries on the owner's
  machine.** CI fetches them.
- **The updater's private key** (`~/.tauri/sub20-updater.key`) stays out of
  the repository and is never printed.
- Build Rust with `CARGO_TARGET_DIR` set outside the OneDrive folder
  (`C:\Users\yadds\AppData\Local\sub20-target`).
- Don't capture the owner's screen, and don't change the owner's real Sub20
  settings or history. Test copies use their own identifier.
- Before finishing any code task: `cargo fmt --all`,
  `cargo clippy --workspace --all-targets`, `npx svelte-check`.
- The README's one line about what inspired Sub20 is the only mention.
- Nothing is invented in public copy: every number on the site, in the
  README or in a post was measured, and every quote was said.

## Settled, not to be reopened

- **Licence:** MIT.
- **Discord:** H.264 is the default and plays on every device. HEVC may not
  play on every device, AV1 may not play on phones, and Options says so
  (owner's test, 15 November 2026: all three on Windows, H.264 and HEVC on
  a phone). The sizes are 20 MB (Free), 50 MB (Nitro
  Basic), 500 MB (Server boost) and 1 GB (Nitro); the free limit is 20 MiB,
  confirmed by upload. Discord's name may describe what Sub20 is for; its
  logo and colours may not be used, and "Not affiliated with Discord"
  stays in the README, the site and the About line.
- **No code signing for 1.0.** Windows shows SmartScreen and macOS needs
  Open Anyway; the README and the download page say how. Updates are signed
  either way. Signing comes back in plan 6.
- **Intel Macs** are not built for.
- **Two downloads:** "Sub20" (FFmpeg included) and "Sub20 without FFmpeg".
- **No telemetry in the app.** The update check is the only request it
  makes by itself.
