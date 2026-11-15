# Contributing to Sub20

Thanks for helping. Bug reports, test results from hardware we don't have,
and pull requests are all welcome.

## Reporting a bug

Open an issue with the bug form. The most useful things to include:

- The diagnostics: **Copy diagnostics** at the bottom of Settings, then
  paste. It
  holds the versions, your system, GPU and the encoders found, with your
  home folder written as `~`.
- What you did, what you expected, and what happened.
- The FFmpeg log for the job ("Show log" under the video).
- If you can share it, a short clip that shows the problem.

Results from Intel, AMD and Apple GPUs are especially welcome: the encoder
settings for those have not been tried on real hardware here yet.

Questions (how to do something, whether Sub20 can) go to
[Discussions](https://github.com/Y4dd/Sub20/discussions). Issues labelled
`good first issue` are small and self-contained; `help wanted` marks ones
that need hardware or know-how we don't have.

By contributing, you agree that your work is published under the
[MIT licence](LICENSE), like the rest of Sub20.

## Building

You need [Rust](https://rustup.rs) 1.80 or newer, Node.js 20 or newer,
FFmpeg 6 or newer, and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your
system.

```bash
npm install
npm run dev            # the interface in a browser, with invented clips
npm run tauri dev      # the real app
cargo test --workspace # unit tests and real FFmpeg encodes
```

## How the code is laid out

[`CLAUDE.md`](CLAUDE.md) holds the design notes: the layers, why each
decision was made, and what has and has not been verified. Read the section
for the part you are changing. The short version:

- `crates/core` is the engine and holds all behaviour. Put logic there, with
  a test.
- `src-tauri` is a thin shell: one command for each `Backend` method.
- `src` is the window (Svelte 5). It talks to the app only through the
  `Backend` interface in `src/lib/types.ts`.

## Before opening a pull request

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check
```

CI runs the same checks on Windows and Linux.

- One change per pull request, with a message that says what changed and
  why.
- For interface changes, attach screenshots in both the dark and light
  themes. `npm run dev` with `?clips` in the address opens the demo clips.
- Add a line to `CHANGELOG.md` under "Unreleased".
- Keep the interface's wording plain: sentence case, plain verbs, errors
  that name the cause and the fix.

## Conduct

Be kind and assume good intent. Everyone taking part follows the
[code of conduct](CODE_OF_CONDUCT.md), which also says how to report a
problem privately.
