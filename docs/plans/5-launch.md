# Plan 5: 1.0 and the launch

**Goal.** Sub20 1.0.0 is released, the site is announced with a real demo
on it, and the people who send clips every day have had a fair chance to
see it.

**Status.** Not started. The drafts (5.3, 5.7 to 5.10) may start now.

**Needs first:** for 5.1, 5.2, 5.5, 5.6 and the posts going out, plan 3
finished. 5.4 (the address) needs only the owner's CNAME record and is
best done soon: before the beta post, so testers get the final address,
and before more people install Lite on the old one. Lite doesn't hold this plan up: if plan 4's gate is still open,
everything below leaves Lite out (plan 4, "Lite doesn't hold up the
launch"). **May start now** (owner, 15 November 2026): 5.3, 5.4 once the
CNAME exists, 5.7, 5.8, 5.9 and 5.10, which need nothing from the beta, so the AI's time isn't idle
while plan 3 waits on people. 5.7's beta post comes first, because plan 3
uses it to find testers. **Unblocks:** plan 6.

**Who it's for.** People who record clips and send them on Discord: PC
gamers using the NVIDIA app, Steam, OBS or Xbox Game Bar, and everyone with
a phone video that is "too large".

**The pitch, in one line:** "Trim any clip and get it under Discord's
limit: free, open source, runs on your GPU, nothing uploaded."

The AI drafts and builds; the owner records, posts and holds the accounts.
Nothing is posted from here.

## Tasks

- [ ] **5.1 The demo, cut for each place.** From the owner's recording
      (below): a looping, silent clip for the site's hero and a GIF or
      short video for the README's top, each compressed with Sub20 itself.
      *Done when:* the home page's hero plays it within the site's weight
      budget (Lighthouse stays 95 or more) and the README opens with it.
- [ ] **5.2 Real numbers and words on the site.** The before and after
      sizes and the quotes collected in plan 3, with permission, where the
      home page can carry them. None invented; if there are none, the
      page stays as it is.
- [ ] **5.3 The write-up.** "Hitting an exact file size with hardware
      encoders", as a page on the site: the strict limit and the measured
      retry, encoders that can't reach low bitrates (NVENC's floor near
      600 kbps), the CPU hand-over, the same planner compiled to
      WebAssembly for the browser. Every number from `CLAUDE.md` or the
      record.
      *Done when:* the owner has read it and it passes `copy-lint`.
- [ ] **5.4 The address.** The owner has `amjad.sh` and would use a
      subdomain (carried from plan 1): once its CNAME record points at
      `y4dd.github.io`, set the custom domain in Pages, then `SITE_URL` and
      `BASE` in the Pages workflow, the `CNAME`, a canonical link in
      `Base.astro`, Lite's manifest scope, and a check that every old
      `y4dd.github.io/Sub20` link still lands.
- [ ] **5.5 Visits.** With the owner's choice (below): GoatCounter or
      Cloudflare Web Analytics, no cookies, no banner, on the site only and
      never in the app or in Lite's encoding path; the privacy page says
      what is counted. Or nothing, and the page says that.
- [ ] **5.6 Release 1.0.0,** the way plan 2 released 0.9.0: changelog,
      version, push, dry run, tag, read the draft. Every guide and the
      compare page are read once more against the app as released.
      *Done when:* the owner has published it and a 0.9.x install updated
      to it.
- [ ] **5.7 Drafts for every post,** in `docs/launch/`, one file each,
      written as a person and not as an advert:
  - **The beta post,** first: a few lines asking for three to five people
    to try the desktop app for a week (at least one not on NVIDIA, one on
    a Mac), with the release page and `docs/testing.md`. For a Discord
    server the owner is in and one small subreddit. Plan 3 uses it.
  - **Show HN** (Tuesday to Thursday, morning in the US): leads with the
    engineering, links the write-up.
  - **Reddit:** r/rust, r/tauri and r/sveltejs (how it's built);
    r/opensource, r/software, r/coolgithubprojects; r/discordapp, then
    the game communities where clips are shared. Each draft starts with
    that subreddit's rules on self-promotion, read that week.
  - **Directories:** AlternativeTo (beside the online compressors,
    HandBrake and LosslessCut), Product Hunt, and pull requests to
    awesome-tauri, awesome-rust, awesome-ffmpeg and awesome-svelte.
  - **Short videos:** "Discord says file too large? Fix it in 5
    seconds", one script each for PC, iPhone and Android.
  *Done when:* the owner has read each draft.
- [ ] **5.8 A calendar,** in `docs/launch/README.md`: the posts spread over
      about two weeks, different audiences on different days, with a line
      under each for what happened.
- [ ] **5.9 The routine afterwards,** written into `CONTRIBUTING.md` or the
      README where it concerns others: issues answered within two days for
      the first month, a release every two to four weeks with a short
      changelog post, a public roadmap (GitHub Projects) built from plan 6,
      five or more `good first issue` tasks open at any time.
- [ ] **5.10 What to watch,** as one small script or a saved `gh` command:
      stars, downloads per release (GitHub's API), and the questions
      people ask, which say what the FAQ and the next release need.

## Needs the owner

- [ ] **Record the demo,** 20 to 30 seconds, in the real app: a clip that
      is too large, two drags of the handles, Compress, Ctrl+V into
      Discord.
- [ ] **Accounts** with one name on Bluesky, X, Reddit and YouTube, after
      checking the name isn't used by a similar product.
- [ ] **Decide:** a visit counter or none. (The address: a subdomain of
      amjad.sh, 5.4. Lite at launch: decided 15 November 2026, left out if
      plan 4's gate is still open.)
- [ ] **The CNAME record** at Porkbun: host `sub20`, pointing at
      `y4dd.github.io` (5.4).
- [ ] **Publish 1.0.0** and **post each draft**, on the calendar's days,
      and answer every comment.

## Exit criteria

- [ ] 1.0.0 is GitHub's latest release, and the site shows it.
- [ ] The site is announced, with the demo on the home page and the README.
- [ ] Show HN, the Reddit posts and the directory entries have gone out,
      each with its result written in the calendar.
