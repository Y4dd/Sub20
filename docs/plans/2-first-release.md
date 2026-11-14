# Plan 2: the first release

**Goal.** v0.9.0 is published on GitHub: every download for Windows, macOS
and Linux, signed update entries, checksums, and notes that open with
"Which file do I need?". It is a beta in name only: a normal release, not a
GitHub pre-release, because the updater and the site both read "latest".

**Status.** Not started. Everything it needs is built: the release workflow
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
- [ ] **2.2 README for a stranger.** The top reads: what it is, the
      download, "Try it in your browser" (the site's `/app`, once plan 1
      is live), then the rest. The Requirements section says the main
      download needs nothing installed (it still says "FFmpeg included").
      The demo GIF's place is left for plan 5.
      *Done when:* someone who has never seen Sub20 finds the right file
      from the README's first screen.
- [ ] **2.3 The AppImage without FFmpeg: keep it.** It is 79 MB because it
      carries its own WebKitGTK, and few will pick it. Keeping it costs
      nothing but storage, and dropping it after a release would leave its
      installs without updates, so the choice is made now: it stays,
      unless the owner says otherwise before the tag.
      *Done when:* the owner has seen this line.
- [ ] **2.4 Version and changelog.** `npm run set-version -- 0.9.0`; turn
      "Unreleased" into `## [0.9.0]` with the date, written for people who
      use the app, with a fresh empty "Unreleased" above it. Run fmt,
      clippy, svelte-check and the tests. Commit.
      *Done when:* the three checks are clean and the tests pass.
- [ ] **2.5 Push, CI, dry run** (on the owner's word). Push `main`; when CI
      is green, run the release workflow by hand on `main`.
      *Done when:* CI and the dry run are green, and the dry run's
      `latest.json` has all ten keys with a signature each.
- [ ] **2.6 Tag** (on the owner's word). `git tag v0.9.0`, push the tag.
      Read the draft: the notes open with the table and the sizes are
      right, every file in the table is attached, `SHA256SUMS.txt` matches
      a file checked by hand, `latest.json` points at this release's
      files.
      *Done when:* the draft is right, or fixed and tagged again before
      anyone publishes it.
- [ ] **2.7 After the owner publishes.** `gh release view v0.9.0`: it is
      the latest release, not a draft, not a pre-release. An installed
      dry-run build (0.1.0) offers the update in its title bar.
      *Done when:* both were seen.
- [ ] **2.8 The site follows it.** The Pages workflow runs on a published
      release; the home page and `download/` then show the version, the
      file names and the sizes, and each link downloads that file.
      (Only if plan 1 is live; otherwise this moves with it.)
      *Done when:* seen on the live site.

## Needs the owner

- [ ] **A short try on Windows before the tag**, from the dry run in 2.5:
      install, compress a clip, Ctrl+C then Ctrl+V into Discord, drag the
      file chip into a channel, close the window while a clip is
      compressing. Ten minutes; the full pass is plan 3.
- [ ] **A friend installs "Sub20 without FFmpeg"** on a machine with no
      FFmpeg: the first screen offers "Download FFmpeg", it downloads, a
      clip compresses. A dry-run build will do.
- [ ] **Turn on private vulnerability reporting:** repository Settings →
      Advanced Security (or Code security) → Private vulnerability
      reporting → Enable. `SECURITY.md`, the code of conduct and the issue
      form's "Security problem" link all send people to it, and it was off
      on 14 November 2026. (Or say so, and the AI turns it on with `gh`.)
- [ ] **Say when to push, and when to tag.**
- [ ] **Publish the draft release.**
- [ ] Optional: a GitHub Sponsors or Ko-fi account, then `FUNDING.yml`.
      Turn on Discussions if questions should go there rather than to
      issues, and add the labels `good first issue` and `help wanted`
      (the AI can do both with `gh` once told to).

## Exit criteria

- [ ] v0.9.0 is published and is GitHub's "latest".
- [ ] Its notes, files, checksums and `latest.json` were read and are
      right.
- [ ] A Windows machine installed it from the release page and compressed
      a clip.
