# Changelog

What changed in each release. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- CI on Windows, macOS and Linux: formatting, Clippy, the unit and end-to-end
  tests, the interface check and build, and dependency advisories.
- Contributing guide, security policy, issue and pull request templates.
- `npm run set-version -- <x.y.z>` sets the version everywhere it is written.
- Sub20 Lite size test (`npm run dev`, then `/lite-lab.html`): checks how
  close the browser's own encoder lands to a size limit.

## [0.1.0] - not released

The first version, before any public release.

### Added

- Drop videos, trim each with two handles, pick a size, compress. Every
  result is measured and encoded again if it lands over the limit; a file
  over the limit is never saved.
- H.264, HEVC and AV1 on NVIDIA, Intel, AMD, Media Foundation, Apple and
  VAAPI hardware, or the CPU, with every encoder tested at startup and a CPU
  fallback mid-job.
- Size presets named after Discord's limits, 20 MB by default.
- Automatic resolution with a quality verdict before compressing. A shorter
  trim never gets a worse verdict.
- Copy a finished video (Ctrl+C) to paste into Discord, or drag it out of
  the window. The preview switches to the result.
- Rename a result from the clip list, before or after compressing.
- Replace the original: the result takes its place and the original goes
  to the Recycle Bin.
- Vim motions and a key sheet (`?`).
- Folder watch, history, saved profiles, the FFmpeg log for each job.
- A title bar drawn by the app on Windows.

[Unreleased]: https://github.com/Y4dd/Sub20/commits/main
[0.1.0]: https://github.com/Y4dd/Sub20/commits/main
