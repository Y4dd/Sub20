# Plan 2: the first release

**Goal.** v0.9.0 is published on GitHub: every download for Windows, macOS
and Linux, signed update entries, checksums, and notes that open with
"Which file do I need?". It is a beta in name only: a normal release, not a
GitHub pre-release, because the updater and the site both read "latest".

**Status.** Finished 15 November 2026: v0.9.0 published, and an installed
build updated itself to it. Everything it needs is built: the release workflow
passed a dry run on 13 November 2026 (see [record.md](record.md)), and the
owner installed and used the Windows build from it.

**Needs first:** nothing; better after plan 1, so the README can point to
the site. **Unblocks:** plan 3 (testers need something to install, and an
update needs two releases), the site's download links, and the friend's
test of the download without FFmpeg.

## What exists

- `.github/workflows/release.yml`: a `v*` tag makes a draft release; run by
  hand it builds everything without releasing. One job per system compiles
  once and packages both downloads.
- `scripts/collect-release.mjs` names the files, builds `latest.json` (ten
  keys: each system, with and without FFmpeg, deb and rpm apart) and
  rewrites the notes with the table of files and sizes. The rewrite runs
  only on a tag, so this release is the first time it is seen.
- `npm run set-version -- x.y.z` writes the version everywhere.
- `CHANGELOG.md` has everything under "Unreleased". The version is 0.1.0.
- In the repository: `LICENSE`, `CONTRIBUTING.md`, `SECURITY.md`, issue
  templates for bugs and features, a pull request template. Missing:
  `FUNDING.yml`, the labels, Discussions.
- The release cache is saved from `main` only, so a dry run on `main`
  shortly before tagging leaves it warm (about 6 minutes instead of 12).

## Tasks

- [x] **2.1 Repository files.** `CODE_OF_CONDUCT.md` (Contributor
      Covenant), and a look through `CONTRIBUTING.md`, `SECURITY.md` and
      the bug template: the template asks for Copy diagnostics, and none
      of them names something that no longer exists.
      *Done when:* the files are committed.
      Done 14 November 2026: the Contributor Covenant 2.0 from GitHub's
      template, reports going to the private report form (no address is
      published); the bug form asks for Copy diagnostics in place of the
      GPU and log fields and no longer offers Intel Macs; `CONTRIBUTING.md`
      points to both. Private reporting itself is off: see below.
- [x] **2.2 README for a stranger.** The top reads: what it is, the
      download, "Try it in your browser" (the site's `/app`, once plan 1
      is live), then the rest. The Requirements section says the main
      download needs nothing installed (it still says "FFmpeg included").
      The demo GIF's place is left for plan 5.
      *Done when:* someone who has never seen Sub20 finds the right file
      from the README's first screen.
      Partly done 14 November 2026: a download line under the tagline
      (latest release, the three systems, "Which file do I need?" to the
      Install table), Requirements per download, the Windows run recorded,
      "Coming next" pointing to the plans. Left: the "Try it in your
      browser" line, once the site is live (1.11).
      Closed 15 November 2026 with that line moved to plan 4 (task 4.4),
      since it waits on Lite being ready to point people at.
- [x] **2.3 The AppImage without FFmpeg: keep it.** It is 79 MB because it
      carries its own WebKitGTK, and few will pick it. Keeping it costs
      nothing but storage, and dropping it after a release would leave its
      installs without updates, so the choice is made now: it stays,
      unless the owner says otherwise before the tag.
      *Done when:* the owner has seen this line.
      Kept: v0.9.0 to v0.9.3 all shipped it. The owner read a list naming
      2.3 on 15 November 2026 and answered "Apply them".
- [x] **2.4 Version and changelog.** `npm run set-version -- 0.9.0`; turn
      "Unreleased" into `## [0.9.0]` with the date, written for people who
      use the app, with a fresh empty "Unreleased" above it. Run fmt,
      clippy, svelte-check and the tests. Commit.
      *Done when:* the three checks are clean and the tests pass.
      Done 15 November 2026: 0.9.0, the changelog rewritten for people who
      use the app (contributors' items last); fmt, clippy, svelte-check
      clean, 120 tests pass.
- [x] **2.5 Push, CI, dry run** (on the owner's word). Push `main`; when CI
      is green, run the release workflow by hand on `main`.
      *Done when:* CI and the dry run are green, and the dry run's
      `latest.json` has all ten keys with a signature each.
      Done 15 November 2026: CI green on all three systems; the dry run
      (run 43683589199) green, its `latest.json` at 0.9.0 with ten signed keys.
- [x] **2.6 Tag** (on the owner's word). `git tag v0.9.0`, push the tag.
      Read the draft: the notes open with the table and the sizes are
      right, every file in the table is attached, `SHA256SUMS.txt` matches
      a file checked by hand, `latest.json` points at this release's
      files.
      *Done when:* the draft is right, or fixed and tagged again before
      anyone publishes it.
      Done 15 November 2026: `v0.9.0` on ad1d13a; the draft has 25 files,
      notes opening with the table (sizes right), the Windows installer
      without FFmpeg matching its `SHA256SUMS.txt` line by hand, and all ten
      `latest.json` entries pointing at v0.9.0. The tag also started the
      FFmpeg workflow (a new tag has no paths to compare); it was canceled,
      and the workflow now runs on branch pushes only.
- [x] **2.7 After the owner publishes.** `gh release view v0.9.0`: it is
      the latest release, not a draft, not a pre-release. An installed
      dry-run build (0.1.0) offers the update in its title bar.
      *Done when:* both were seen.
      *Half seen, 15 November 2026:* published at 06:40 UTC, latest, not a
      draft or pre-release. `releases/latest/download/latest.json` (the
      updater's endpoint) says 0.9.0 with ten signed keys, and every URL in
      it downloads. Seen the same morning: the owner's Windows install
      offered 0.9.0, installed it and restarted as 0.9.0 (the running
      `sub20.exe` started at 06:50 UTC with the installer as its parent;
      Copy diagnostics says 0.9.0, without FFmpeg). After that restart its
      FFmpeg didn't answer; see plan 3, 3.6.
- [x] **2.8 The site follows it.** The Pages workflow runs on a published
      release; the home page and `download/` then show the version, the
      file names and the sizes, and each link downloads that file.
      (Only if plan 1 is live; otherwise this moves with it.)
      *Done when:* seen on the live site.
      *Done 15 November 2026:* the run the release started built the site
      but its deploy was refused: the `github-pages` environment allows
      deploys from `main` only, and a release runs on its tag. Run again from
      `main` by hand, it deployed; `download/` lists all ten files of v0.9.0
      with sizes, and each link answers 200. For the next release the owner
      adds a tag rule (Settings → Environments → github-pages → Deployment
      branches and tags → add `v*` as a tag), or runs Website by hand after
      publishing.

## Needs the owner

- [x] **A short try on Windows before the tag**, from the dry run in 2.5:
      install, compress a clip, Ctrl+C then Ctrl+V into Discord, drag the
      file chip into a channel, close the window while a clip is
      compressing. Ten minutes; the full pass is plan 3.
      Done by the owner on the 13 November dry run (15 November 2026: works).
- [x] **A friend installs "Sub20 without FFmpeg"** on a machine with no
      FFmpeg: the first screen offers "Download FFmpeg", it downloads, a
      clip compresses. A dry-run build will do. Done, 15 November 2026.
- [x] **Turn on private vulnerability reporting:** repository Settings →
      Advanced Security (or Code security) → Private vulnerability
      reporting → Enable. `SECURITY.md`, the code of conduct and the issue
      form's "Security problem" link all send people to it, and it was off
      on 14 November 2026. (Or say so, and the AI turns it on with `gh`.)
      On, 15 November 2026 (the API reports it enabled).
- [x] **Say when to push, and when to tag.** 15 November 2026: push, and
      tag at the best time.
- [x] **Publish the draft release.** Done 15 November 2026 ("Yes, publish
      it").
- [ ] Optional: a GitHub Sponsors or Ko-fi account, then `FUNDING.yml`.
      Discussions on and both labels there, 15 November 2026 (the labels
      are GitHub's defaults); the issue chooser and `CONTRIBUTING.md` point
      questions to Discussions. Left: an account, if wanted.
      Turn on Discussions if questions should go there rather than to
      issues, and add the labels `good first issue` and `help wanted`
      (the AI can do both with `gh` once told to).

## Exit criteria

- [x] v0.9.0 is published and is GitHub's "latest". (15 November 2026,
      06:40 UTC; 2.7.)
- [x] Its notes, files, checksums and `latest.json` were read and are
      right. (2.6 and 2.7.)
- [x] A Windows machine installed it from the release page and compressed
      a clip. (A friend of the owner installed "Sub20 without FFmpeg" on
      15 November 2026 and compressed clips; not recorded: whether from
      the release page or a dry-run build. The owner's own install
      updated to v0.9.0 through the updater the same morning.)
