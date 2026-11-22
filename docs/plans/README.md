# Sub20: the plans to 1.0

Goal: a free, open-source 1.0 that anyone can install in one click, a
browser version for people who can't or won't install anything, a website
that ranks for "video too big for Discord", and a launch that gets it in
front of the people who send clips every day.

The road is split into seven plans that run in order (plan 7 sits between
4 and 5: it was added later and kept its number so task names stay put). Each plan is the source
of truth for its own work: what to build, in which order, and when it is
finished. What was built and measured before the split is in
[record.md](record.md).

| # | Plan | Delivers | Needs first | Status |
| --- | --- | --- | --- | --- |
| 1 | [Website](1-website.md) | The site under `site/`, deployed to GitHub Pages with Lite at `/app`, unannounced | nothing | Finished 15 November 2026; the subdomain moved to 5.4. Reopened for 1.12 (a calmer home page and search basics, 16 November 2026): built, waits on Lighthouse and the live check |
| 2 | [First release](2-first-release.md) | v0.9.0 on GitHub: every download, signed updates, the site's links following it | nothing (better after 1) | Finished 15 November 2026 |
| 3 | [Desktop beta](3-desktop-beta.md) | The desktop app tried by hand on three systems, a small beta (three to five people), its fixes | 2 | Started: Windows passed, 0.9.1 to 0.9.3 fixed the first reports; waits on the owner's Windows sitting and the testers |
| 4 | [Sub20 Lite](4-lite.md) | Lite past its size test on real clips, browsers and phones, and its gaps closed or labelled | 1 (phones need it hosted) | Built, Lighthouse and axe clean; not tested outside this machine. Doesn't hold up the launch |
| 7 | [The watch without a window](7-tray.md) | Windows: keep watching from the tray after the window closes, and start there at sign-in, off by default | nothing | Started: 7.1 passed, 8 MB with no window; 7.2 to 7.4, 7.12 and 7.13 done (20 November 2026), 7.5 and 7.6 (22 November), 7.7 built. Moved up from plan 6's 6.3 (owner, 20 November 2026); goes back there if its memory budget fails, and 1.0 doesn't wait on it |
| 5 | [1.0 and launch](5-launch.md) | The demo video, 1.0.0, the launch posts, the routine afterwards | 3 (the drafts may start now) | Not started; the beta post is first |
| 6 | [After 1.0](6-after-1.0.md) | Package managers, the watch without a window, new features | 5 | Parked |

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
  AI can't reach. Plans 3 and 4 run side by side: when one is waiting on
  the owner or testers, work on the other, or on plan 5's drafts.
- **A small beta, not a big one** (owner, 15 November 2026): why is in
  plan 3's goal.
- **The watch without a window beside the beta** (owner, 20 November 2026):
  it was after 1.0, but the watch is what gamers use and it stops with the
  window. It ships off by default in 0.9.x so testers try it; if its
  memory budget fails it goes back to plan 6, and the launch doesn't wait.
- **Launch last,** once the beta has no open crash or data-loss bug. Lite
  doesn't hold it up (plan 4, "Lite doesn't hold up the launch").
- **Plan 6 holds everything that isn't needed for 1.0,** so the plans
  before it stay small.

## How to work from these plans

For the AI doing the work:

1. Work on the lowest-numbered plan that isn't finished, unless the owner
   names another. Plans 3, 4 and 7 may be worked side by side, and plan
   5's drafts (5.3, 5.7 to 5.10) beside them. While plan 3 waits on
   people, plan 7 comes first.
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
8. **One home per fact.** A decision is written in `CLAUDE.md`, a
   measurement in `record.md`, and a plan says what it means for the plan
   and points there instead of copying it. A change of fact then edits
   one place, plus whatever shows it to people (the app, the site, the
   changelog).
9. **Bug reports** are written down in plan 3's task 3.6 with the date
   and who reported them; a tester's report also becomes a GitHub issue,
   so they can follow it.
10. **Releases:** while the only users are the owner and one tester, a
    release goes out as soon as something is ready (owner, 15 November
    2026). Once the beta has more testers: a fix for a lost file, a file
    over the limit or a crash goes out at once; anything else waits for
    one release a week.

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
- **Discord:** H.264 is the default; which formats play where and the
  size limits are in `CLAUDE.md` ("Discord facts"). Discord's name may
  describe what Sub20 is for; its
  logo and colours may not be used, and "Not affiliated with Discord"
  stays in the README, the site and the About line.
- **Code signing for 1.0,** the Windows installer through SignPath
  Foundation (owner, 15 November 2026; plan 3's task 3.8). Until it is
  granted, Windows shows SmartScreen and the README and the download page
  say how. Updates are signed either way.
- **No macOS desktop app** (owner, 15 November 2026; 0.9.3 was the last).
  Lite in Safari stays.
- **Two downloads:** "Sub20" (FFmpeg included) and "Sub20 without FFmpeg".
- **No telemetry in the app.** The update check is the only request it
  makes by itself.
