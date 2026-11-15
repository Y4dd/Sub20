# Changelog

What changed in each release. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- Cutting a finished video starts exactly where the start handle is, not
  at the keyframe up to two seconds before it. The cut still copies the
  file without compressing it again. (In an MKV result the start still
  moves back to a keyframe; it used to go back one keyframe too far.)

## [0.9.1] - 2026-11-15

Fixes from the first days of the beta.

### Fixed

- An FFmpeg that is found but won't start no longer looks like a working
  one, which made every dropped video fail as unreadable. Sub20 now shows
  the FFmpeg screen, with where it found FFmpeg and why it didn't start,
  and offers the download; Copy diagnostics says the same.
- When cutting a finished video, the start handle follows the pointer
  smoothly instead of jumping two seconds at a time. A mark shows the
  keyframe the cut will start on, and the handle settles there when you
  let go.

### Changed

- Sub20 Lite writes results of 64 MB or more to the browser's own storage
  as they are made, instead of holding them in memory, so a 500 MB or 1 GB
  limit works on a phone.
- Sub20 Lite encodes the codec picked in Options (H.264, AV1 or HEVC) where
  the browser can, and a browser without H.264 falls back to AV1.

## [0.9.0] - 2026-11-15

The first public release. Sub20 trims your clips and fits each one under
Discord's limit, on your own graphics card, with nothing uploaded.

### Added

- **Cut a finished video shorter** without compressing it again: on
  Result, move its handles and press Cut. It takes a moment, loses nothing
  and only gets smaller. The start lands on the nearest keyframe before it,
  at most two seconds earlier.
- **Every sound track is mixed into one**, so a microphone that OBS or the
  NVIDIA app recorded on its own track is no longer lost. "Mix all sound
  tracks" in Options → More turns it off.
- **HDR videos** (HDR10 and HLG, from phones and games) are converted to
  ordinary colour, so they no longer come out grey. The stage says when a
  clip is HDR.
- **Watch a folder**: Settings offers the NVIDIA app's and Xbox Game Bar's
  clip folders in one click, and new clips are compressed as they are
  saved. The title bar says "Watching", History lists the watched clips
  with Open and Copy, and Windows tells you when one is ready, never during
  a game; clicking the notice opens the clip, ready to copy. When more than
  five clips arrived while Sub20 was closed, it asks once whether to
  compress them.
- **Open videos from the system**: right-click videos in Explorer and
  choose "Compress with Sub20", or use "Open with" on any system. Sub20
  doesn't make itself the default player, and videos opened while it runs
  join the open window.
- **Drop or choose a folder**, and the videos in it and its subfolders are
  added in name order, without Sub20's own results.
- **Leave a video out** of Compress without removing it: "Leave out" under
  its plan, or the `s` key.
- **Updates from inside the app**: Sub20 checks for a newer version when it
  opens (Settings → Updates turns that off), and the title bar says when
  one is out. Updates are signed.
- **Two downloads**: Sub20 with FFmpeg included, and Sub20 without FFmpeg,
  which offers to download it when the computer has none (checked against
  a pinned fingerprint before it is installed). The release notes open with
  a table of which file to take.
- **Copy diagnostics** (Settings, at the end) puts what a bug report needs
  on the clipboard, with your home folder hidden. Crashes are written to a
  log it includes.
- A failed video says what went wrong and what to do: the source was moved
  or deleted, the results folder is gone, the disk is full, the folder
  can't be written to, or the source can't be read (it may be damaged or
  still being recorded).
- Sub20 asks before quitting while videos are compressing.
- The options you last used are there again the next time Sub20 opens.
- A video too long for the size says how long a clip the size holds.
- Page Up and Page Down move a focused trim handle a twentieth of the video.
- A phone-sized layout: under 720 pixels the video list becomes a strip
  above the preview.
- Sub20 Lite, the same app in a browser at the website's `/app`, encoding
  on the browser's own graphics card with nothing uploaded. It can be
  installed, works offline, and on a phone hands the result to Discord with
  Share. Still being tested, so not announced yet.
- A website with the downloads, guides (a file too large for Discord,
  NVIDIA Instant Replay clips, iPhone videos, HEVC), a comparison with
  other tools, the privacy page and this changelog.

### Changed

- Sub20 is open source under the MIT licence.
- The size presets follow Discord's limits: 20 MB (free), 50 MB (Nitro
  Basic), 500 MB (a server boost) and 1 GB (Nitro). A list you edited
  yourself is kept.
- The result plays in the same player as the original, over a filmstrip of
  its own, so it can be sought.
- Results have a keyframe every two seconds, so they cut close to any point
  and seek better in Discord.
- HEVC says Discord won't play it in the chat, where you pick it and in the
  options summary.
- Sub20 builds its own FFmpeg, with only what it uses, and publishes its
  source code beside it, so the download with FFmpeg included is smaller.

### Fixed

- Adding many videos no longer leaves the list empty until the last one is
  read: they appear as they are read, and filmstrips fill in frame by frame.
- A settings file saved with a byte-order mark (as Windows PowerShell 5 and
  older Notepad save them) no longer resets every setting.
- The folder watch costs a tenth of what it did on a folder with thousands
  of clips, and the window does less work while compressing.
- Sheets keep keyboard focus inside and return it when closed; the size
  picker and the segmented choices work fully from the keyboard; the play
  button and the size limit are named by what they show, for voice control.

### For contributors

- CI on Windows, macOS and Linux: formatting, Clippy, the unit and
  end-to-end tests, the interface check and build, dependency advisories.
- A contributing guide, security policy, code of conduct, issue and pull
  request templates, and Discussions for questions.
- `npm run set-version -- <x.y.z>` sets the version everywhere it is written.
- The Sub20 Lite size test (`npm run dev`, then `/lite-lab.html`).

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

[Unreleased]: https://github.com/Y4dd/Sub20/compare/v0.9.1...HEAD
[0.9.1]: https://github.com/Y4dd/Sub20/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/Y4dd/Sub20/releases/tag/v0.9.0
[0.1.0]: https://github.com/Y4dd/Sub20/commits/main
