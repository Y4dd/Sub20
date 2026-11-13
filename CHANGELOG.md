# Changelog

What changed in each release. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Leave a video out of Compress without removing it: "Leave out" under its
  plan, or the `s` key. It stays in the list with its trim and name, and
  "Include" puts it back.
- Every sound track is mixed into one, so a microphone that OBS or the
  NVIDIA app recorded on its own track is no longer lost. "Mix all sound
  tracks" in Options → More turns it off.
- CI on Windows, macOS and Linux: formatting, Clippy, the unit and end-to-end
  tests, the interface check and build, and dependency advisories.
- Contributing guide, security policy, issue and pull request templates.
- `npm run set-version -- <x.y.z>` sets the version everywhere it is written.
- Sub20 Lite size test (`npm run dev`, then `/lite-lab.html`): checks how
  close the browser's own encoder lands to a size limit.
- Updates from inside the app: Sub20 checks GitHub for a newer version when
  it opens (Settings → Updates turns that off), and the title bar says when
  one is out. Updates are signed.
- A release workflow that builds Sub20 with FFmpeg included, and a lean
  build without it, for Windows, macOS and Linux.
- The lean build offers to download FFmpeg when the computer has none: the
  same FFmpeg the full build carries, checked against its pinned
  fingerprint before it is installed.
- Sub20 now builds its own FFmpeg, with only what it uses, and publishes its
  source code beside it. The download with FFmpeg included is smaller.
- Sub20 asks before quitting while videos are compressing.
- The options you last used are there again the next time Sub20 opens.
- Filmstrip thumbnails appear one by one as they are made, not all at once
  after a wait.
- Copy diagnostics (Settings, at the end) puts what a bug report needs on
  the clipboard, with your home folder hidden. Crashes are written to a log
  it includes.
- A failed video says what went wrong and what to do: the source was moved
  or deleted, the results folder is gone, the disk is full, the folder
  can't be written to, or the source can't be read (it may be damaged or
  still being recorded).
- The About line names the licence, says Sub20 isn't affiliated with
  Discord, and links the bundled FFmpeg's licence.
- HDR videos (HDR10 and HLG, from phones or games) are converted to
  ordinary colour, so they no longer come out grey and washed out. The
  stage says when a clip is HDR.
- A video too long for the size says how long a clip the size holds, in
  hours and minutes, instead of a count of seconds.
- Page Up and Page Down move a focused trim handle a twentieth of the video,
  for long recordings.
- Sheets keep keyboard focus inside and return it when closed; the size
  picker and the segmented choices work fully from the keyboard.
- Right-click videos in Explorer and choose "Compress with Sub20", or use
  "Open with" on any system; Sub20 doesn't make itself the default player.
  Videos opened while Sub20 is running join the open window instead of
  starting a second copy.
- Settings offers to watch the NVIDIA app's and Xbox Game Bar's clip folders
  in one click when they are found, compressing new clips as they are saved.

### Changed

- Sub20 is now open source under the MIT licence (`LICENSE`).
- The size presets follow Discord's limits: 20 MB (free), 50 MB (Nitro
  Basic), 500 MB (a server boost) and 1 GB (Nitro). 100 MB is gone; a list
  you edited yourself is kept.
- HEVC now says Discord won't play it in the chat, where you pick it and in
  the options summary.

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
