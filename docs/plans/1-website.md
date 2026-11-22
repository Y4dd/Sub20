# Plan 1: the website

**Goal.** The site under `site/` is built, checked, committed and deployed
to GitHub Pages, with Sub20 Lite served at `/app`. It stays unannounced
until the launch (plan 5): nothing links to it yet except the repository.

**Status.** Finished 15 November 2026: live at y4dd.github.io/Sub20 with
Lite at `/app`, deploying itself on every published release. The
subdomain of amjad.sh is carried to plan 5's task 5.4.

**Needs first:** nothing. **Unblocks:** plan 2 (the README and release notes
point to the download page), plan 4 (Lite on a phone needs HTTPS), and
SignPath later (it asks for a website).

## What it is

- **Stack:** Astro 7, static output, `@astrojs/sitemap`. The design system
  is the taste skill's kit, copied to `site/public/assets` with its MIT
  licence. The palette and fonts are the app's own (`src/app.css`:
  Bricolage Grotesque, Hanken Grotesk), so the site and the app read as one
  thing.
- **Address:** `site` comes from `SITE_URL` (default
  `https://y4dd.github.io`) and `base` from `BASE` (default `/Sub20`), so a
  domain later is two variables. Every internal link goes through `u()` in
  `src/lib/site.ts`.
- **Downloads:** `src/lib/release.ts` reads GitHub's latest release at
  build time (`GITHUB_TOKEN` if set) and gives each row its file and size.
  With no release, every link falls back to the releases page.
- **Files:** `src/layouts/Base.astro` (head, nav, closing screen, footer;
  `doc` for text pages), `src/pages/index.astro`, `src/styles/site.css`
  (only what the kit lacks), `src/assets/img/*.webp` (screenshots of the
  app on the mock backend, served through `Shot.astro` with a `srcset`),
  `public/icon.svg`, `public/share.jpg`. Text pages use `Doc.astro`.

Decisions already taken:

- The h1 names the category ("Video compressor for Discord clips"), which
  is what people search for. "Trim it. Fit it. Send it." is not used.
- The hero shows pieces of the app's own window. The looping demo video
  comes with plan 5's recording.
- **No analytics** for now. Plan 5 decides whether to add one.
- No canonical link until there is a domain.
- The FAQ sends questions to GitHub issues (Discussions are off).

## Commands

From `site/`. `<skill>` is
`C:/Users/yadds/.claude/plugins/cache/taste-skill/taste-skill/2.5.1/skills/taste-skill`.
Python is not installed on the owner's machine: write helper scripts in
Node.

```bash
MSYS_NO_PATHCONV=1 BASE=/ npx astro build
```

The checks run on that build (`BASE=/`), one at a time, in the foreground:

```bash
CHROME_PATH="C:\Users\yadds\AppData\Local/ms-playwright/chromium-1248/chrome-win64/chrome.exe" node <skill>/scripts/consistency.mjs dist/index.html --root dist --quick --widths 1440,390 --profile .taste-skill/chrome-cons --page software --category "video compressor for Discord clips" --primary "Download Sub20" > .taste-skill/cons.txt
```

```bash
node <skill>/scripts/check.mjs --url dist/index.html --root dist --sizes 1440x900,1024x768,820x1180,768x1024,390x844
```

```bash
node <skill>/scripts/copy-lint.mjs dist/index.html --kind software
```

```bash
node <skill>/scripts/gate.mjs dist/index.html --root dist
```

`shoot.mjs --url dist/index.html --root dist --w 1440 --out .taste-skill/shots/1440.png --profile .taste-skill/chrome-check`
takes the pictures to look at, and `perf.mjs` measures weight and speed.
The consistency report is `dist/.taste-skill/consistency/report.json`. The
gate passed on 14 November 2026 and closed its build; a redesign arms it
again with `gate.mjs --start dist/index.html --root dist --kind software`.

The skill's rules that shaped the page: one section per screen, equal left
and right edges, a 12-column grid, a noun or a digit in every heading,
buttons as a verb and an object, no captions under pictures, a visual of a
quarter of the section or more, no more than two cards drawn the same way,
"Download Sub20" only in the nav, the hero and the close, and nothing
invented.

## Tasks

- [x] **1.1 Scaffold.** Astro project, the kit, the app's palette and
      fonts, `Base.astro`, the release reader. Built 13 November 2026.
- [x] **1.2 Home page.** Hero (collage of the app's pieces), the app's
      window, four feature cards with the formats marquee, downloads beside
      the first screen, Lite, questions, the closing screen. Built
      13 November 2026; screenshots are real ones from the mock backend.
- [x] **1.3 Home page passes the consistency check.** Rebuild, run
      `consistency.mjs`, fix what fails. The last edits aimed at: overflow
      in the feature cards, headings without a noun, the hero's first
      collage set, no visual in Downloads, text-only FAQ, contrast of the
      `#app` lead, the Lite facts off the grid, air between sections.
      Warnings that are right to keep: the main action repeated in nav and
      hero, the h1's length, links to pages that don't exist until 1.6.
      *Done when:* no FAIL at 1440 and 390.
      Done 14 November 2026: 0 FAIL at 390; at 1440 one FAIL, the dealt
      marquee's rail (`row-lines`), waived with its reason in
      `.taste-skill/waivers.json` with the two `bleed-one-side` findings for
      its tracks. The page had run 3063 px wide (the marquee widened the
      `.wrap` grid) and sections ended early; both fixed.
- [x] **1.4 The other checks.** `check.mjs` at five sizes, `copy-lint`,
      `perf.mjs`, then look at a shot of every width.
      *Done when:* all pass and the shots show nothing cut off or empty.
      Done 14 November 2026: `check.mjs` finds only the waived marquee
      tracks; `copy-lint` 0 FAIL, 0 WARN; `perf.mjs` 0 FAIL after every
      picture went through `Shot.astro` (width, height, `srcset`).
- [x] **1.5 Share image and structured data.** `public/share.jpg` from the
      hero (the head already points to it), and `SoftwareApplication`
      JSON-LD on the home page: name, the three systems, price 0, the
      licence, the download URL. No ratings: there are none.
      *Done when:* the built page has both and the JSON-LD parses.
      Done 14 November 2026: `share.jpg` (1200x630, cut from the hero),
      og and Twitter image tags, and the JSON-LD in the built page parses.
- [x] **1.6 Text pages**, each on `Base` with `doc`
      (`article.doc > .wrap`), each through `copy-lint`:
  - `download/`: both downloads per system from `release.ts`,
    `SHA256SUMS.txt`, the SmartScreen and Gatekeeper steps and the
    requirements, in the README's words.
  - `guides/` and four guides: "Discord says your file is too large: what
    to do", "Send an NVIDIA Instant Replay clip on Discord", "Send an
    iPhone video on Discord" (Lite has no HDR conversion; the desktop app
    does), "Why your HEVC video may not play on Discord" (it said "won't" until
    15 November 2026).
  - `compare/`: HandBrake, LosslessCut and the online compressors that
    upload files, fairly: what each does better is said too.
  - `privacy/`: nothing uploaded, no telemetry, the update check, what
    GitHub Pages itself logs.
  - `changelog/`, rendered from `../CHANGELOG.md`.
  *Done when:* every page builds, every claim in a guide was checked
  against `CLAUDE.md` or the app, and no link on the site is dead.
      Done 14 November 2026: all built, each 0 FAIL and 0 WARN in
      `copy-lint` (the changelog keeps one WARN from `CHANGELOG.md`'s own
      words), claims checked against `CLAUDE.md`, every internal link
      answered under `/Sub20/`, lists and tables looked at in shots.
- [x] **1.7 The visitor's system first.** A few inline lines that mark the
      visitor's row in the download list (Windows, macOS, Linux), with no
      layout shift and nothing changed without JavaScript.
      *Done when:* it works at 1440 and 390 and the checks still pass.
      Done 14 November 2026: an inline head script sets `data-os` on
      `<html>` (none on phones and tablets); a yellow bar beside the rows,
      drawn outside the flow, in the home list and the download table.
- [x] **1.8 Pages workflow.** `.github/workflows/pages.yml`, run by hand
      and when a release is published: `npm run build:web`, copy `dist-web`
      to `site/dist/app`, build the site with the real base, deploy with
      the Pages actions. Root `.gitignore` gains `site/dist`,
      `site/node_modules`, `site/.astro`, `site/.taste-skill`. CI's "prose
      only" skip should cover `site/` so a site edit doesn't run the Rust
      jobs.
      *Done when:* the workflow file passes `actionlint` or a careful read,
      and a local run of the same steps gives a `site/dist` with `/app`
      in it that opens from a static server under `/Sub20/`.
      Done 14 November 2026: `.github/workflows/pages.yml`; CI skips
      `site/**` and `pages.yml`. Run locally: Lite at `/Sub20/app/` loads
      and its service worker registers, and every link on the home page and
      a guide answers under `/Sub20/`.
- [x] **1.9 Finish gate and Lighthouse.** The gate's last run, and
      Lighthouse on a phone profile for the home page and one guide.
      *Done when:* the gate passes and both score 95 or more for
      performance and accessibility.
      Done 14 November 2026: the gate passed in round 2 (round 1 failed
      `button-styles` at 390, the selected size tile's outline). Lighthouse
      on a phone, behind a gzip server as Pages serves: the iPhone guide 96
      for performance, 100 for accessibility, best practices and SEO; the
      home page 95 (three runs) and 100. It started at 74 (89 with gzip):
      dropping the kit's unused grounds and preloading both fonts took
      first paint from 2.4 s to 1.7 s (93), and cutting the display face to
      the widths and weights the site uses (78 KB to 57 KB,
      `npm run fonts`) took LCP to 2.8 s (95). The rest of the LCP is the
      hero's entrance, which the skill requires. Measure again live in 1.11.
- [x] **1.10 Write it down and commit.** A "Site decisions" section in
      `CLAUDE.md` (stack, the address variables, the checks, what the kit
      is), a `CHANGELOG.md` line, the plan's table. Commit `site/`, the
      workflow and the docs.
      *Done when:* `git status` is clean and fmt, clippy and svelte-check
      are still clean.
      Done 14 November 2026.
- [x] **1.11 Deploy and check it live** (after the owner enables Pages and
      says to push). Run the workflow; open the live site: every page, the
      download links, `/app/` loading and compressing a clip, its service
      worker registering, `sitemap-index.xml`. Set the repository's
      homepage to the site.
      *Done when:* all of that was seen on the live address.
      Done 15 November 2026 at https://y4dd.github.io/Sub20/: every page,
      `/app/` and its manifest, `sw.js` and `sitemap-index.xml` answer 200;
      the download links go to the releases page (no release yet); in
      headless Edge on a phone profile, `/app/` has no installability
      errors, its worker takes control, it opens offline, a clip compresses,
      and axe is clean. The repository homepage points to the site.
- [ ] **1.12 A calmer home page, and search basics** (owner, 16 November
      2026: "more UI/UX friendly and less cluttered", and better SEO for
      the site and the README). The hero becomes one app window under a
      centred headline (the dealt `centred` hero) in place of the cycling
      collage, and takes over the `#app` section, so the page loses a
      screen; the nav's three screenshot cards become plain links; the
      card panels get a ground in the app's yellow instead of the kit's
      beige; the FAQ's About screenshot gives way to links to the guides,
      and the FAQ answers the questions people search for. Search: titles
      and descriptions shaped like the searches, `twitter:` title and
      description, `og:url`, a sitemap link, `FAQPage` from the visible
      questions, `Article` and `BreadcrumbList` on guides. The README gets
      its keywords into the heading and first lines, a screenshot and links
      to the site and guides. Still no canonical link (no domain yet), and
      no `robots.txt` (crawlers only read it at the host's root, which
      `/Sub20` isn't).
      *Done when:* the gate passes on the new page, Lighthouse on a phone
      stays at 95 or more, and the shots at 1440 and 390 show nothing cut
      off or empty.
      Built 16 November 2026: the gate passed in round 3 (round 1: the
      questions ran 1.77 phone screens, so the footer showed in their
      second; round 2: `--hm: 2` left the gap at the foot, so the spare
      room now sits between the guides and the questions). `check.mjs` at
      five sizes finds only the waived marquee tracks; `copy-lint` 0 FAIL,
      0 WARN on the home page, the guides, download and compare; `perf.mjs`
      0 FAIL (two WARNs for the size and watch shots drawn smaller than
      their boxes, as before). The questions gained a picture (`done.webp`,
      the finished file and Copy) for `text-only`. Left: Lighthouse on a
      phone (not installed on this machine), and the live check after the
      Pages workflow runs. The repository's description and topics were
      updated for search on 17 November 2026 (owner's yes): the
      description opens "Video compressor for Discord", and ten topics
      were added (discord-video, video-compressor, compress-video and
      others).
- [ ] **1.13 Answers that AI search can quote** (owner, 17 November 2026:
      come up in Google's AI summary for Discord compression searches).
      Google's AI Overviews quote pages Google has already indexed and
      ranks; there is no markup, file or opt-in for them (Google's "AI
      features and your website", read 17 November 2026; it says no
      `llms.txt` is needed). So: indexing (below, the owner's), and pages
      that answer the search in their first lines. A guide whose heading
      is the search itself, "How to compress a video for Discord", opening
      with the whole answer, the bitrate sum and a hand-run FFmpeg command
      tested on a real encode; `file-too-large` stays the error's page and
      the two link to each other. Each guide shows the date it was last
      checked and repeats it as `dateModified`. Mentions and links from
      elsewhere come with plan 5's launch.
      *Done when:* the guide is live, linked from the home page, the guides
      page and the README, and the gate still passes.
      Built 17 November 2026: `guides/compress-video-for-discord/`, first
      in `GUIDES`, so the home page's questions, the guides page and every
      guide's "More guides" list it; `file-too-large` links to it and back.
      Its FFmpeg command was run as printed on a generated 1080p 60 fps
      clip with a 440 Hz tone (FFmpeg 9.0.2): 50 s kept, 18.3 MB, 91.5% of
      20 MB; the same clip at 60 s in one pass came out at 92%, in two
      passes 98%. Guides show "Checked against Sub20 on …" from
      `GUIDES.checked` (15 November for the four read then). Gate PASS,
      copy-lint 0 FAIL and 0 WARN on the new guide, the guides page and
      `file-too-large`. Left: it goes live with the next Pages run, and
      indexing waits on the owner (below).
- [ ] **1.14 The home page after the critique** (20 and 21 November 2026;
      the critique is `.impeccable/critique/2026-11-21T02-56-26Z__site-src-pages-index-astro.md`).
      Five fixes, merged at `3b02c9c`: links that say where they go and a
      download mark; the trim bracket round the headline and the close,
      and the formats as the app's timeline (`.kinds`) in place of the
      marquee; legible feature shots in features as tall as their cards;
      phones and Macs offered Sub20 Lite first; one download button for
      the visitor's system, the rest behind "Other systems and downloads".
      Then a polish pass: one download mark (`DownloadMark.astro`) after
      the words everywhere; the download button at the kit's one size; on
      phones, tablets and Macs the download block lists the files for the
      visitor's computer and the close names Lite's terms; a section at
      least a screen and growing from 1280px too (at 1366x700 the last
      question ran under the close); the questions in one screen at
      1440x900; the phone's questions in their own rows; the upright
      tablet's hero frame holding its window alone; the marquee's leftover
      CSS gone.
      The marquee's three waivers went with it. `.taste-skill/waivers.json`
      now waives two findings about the formats rail: `row-lines` (its
      handles stand on the container's lines, so the frames start 16px in)
      and `one-section` (the rail reads as a band; it is the app's
      filmstrip). The main checkout's git-ignored copy still holds the
      three marquee waivers until it is replaced.
      Checks on 21 November 2026 (skill 2.5.1, where `check.mjs` is
      `assets/layout/check.mjs`; Edge's launcher exits at once under
      `consistency.mjs`, so Chrome from Playwright): copy-lint 0 FAIL,
      0 WARN on the home page, download and the first guide; `perf.mjs`
      0 FAIL (3 WARN, as before); `check.mjs` at five sizes finds only
      `#features` (1.5 screens at 1440, 2.16 at 768x1024, 2.08 on a phone)
      and the neighbour it lets show. `consistency.mjs --quick` 16 FAIL
      (22 after the five fixes, 1 before them); with `--rhythm-widths 390`
      the questions are exactly two phone screens, and Lite and the close
      run 1.2 (as on 16 November). Open,
      for the owner: `#features` is 1.5 screens at 1440x900 by the
      features fix, which fails the kit's one-or-two-screens rhythm there
      and moves every later section off the screen grid (`section-height`
      5, `one-section` 3); this can't be waived (one waiver a finding,
      five at most): either features go back to two screens with the room
      that brings, or the rhythm rule is set aside. `no-visual` on
      `#download` and `text-only` on `#faq` (the finished clip moved from
      the questions to the downloads, and a single download button leaves
      the downloads 44% full, `section-gaps` and `section-air` with it)
      need a picture: a fresh capture of the finished window would serve
      both.
      *Done when:* the owner has decided the rhythm and the pictures, and
      the gate passes again.
- [ ] **1.15 The home page after the second critique** (21 November 2026,
      30/40; `.impeccable/critique/2026-11-21T09-33-39Z__site-src-pages-index-astro.md`).
      Five fixes on branches of their own, merged at `2af7284`:
  1. The download section is as tall as it holds where its list starts
     closed, and its frame is a before and after: the plan row cropped
     from `window.webp` ("Should look good"), then the file chip from
     `done.webp` (612 MB to 19.4 MB); three steps for one game clip since
     1.16. Where the desktop app doesn't run
     (`data-lite`), Sub20 Lite comes first and the files are folded; a
     link to `#download` opens them.
  2. Lite's facts say what it saves and how to install it, and each guide
     beside the questions carries its summary.
  3. On Windows or Linux the hero's and the close's "Download Sub20" is
     that system's file with its size and the download mark
     (`GetSub20.astro`), with Windows's warning under it
     (`SmartScreenNote.astro`); the hero's lead names Lite in the browser
     on `data-lite`; `/download/` tells the installer's signing from the
     update key.
  4. The formats rail drawn quietly: 8px handles (10px when it wraps),
     2px bars, frames of the page's surface colour.
  5. The hero's window has no slabs beside it and a short screen shows a
     larger crop of its foot; each feature card sets its title and lines
     side by side over its shot; under 800px tall the hero's words start
     24px higher.
  6. A polish pass after the merge: Windows's warning in one wording
     everywhere ("Windows will warn you because Sub20 is not code-signed
     yet: click More info, then Run anyway.") under each button that
     downloads the installer, in the download section also without
     JavaScript or a release (where the warning shows, its link to the
     install steps stands in for the list's own), and the FAQ's answer
     gives the same cause and steps;
     `data-tone="dark"` written on `<html>`, so the close and the footer
     stay dark without JavaScript (they turned light); the hero window's
     `sizes` as the CSS draws it (at 1280x720 and 1366x700 it loaded the
     1000w file for 1120px; now 1280w, and 1000w at 1440x900, where the
     window is 880px); the hero's and the close's file buttons wrap under
     360px (at 320 the close's Windows button ran 21px past the screen,
     the hero's Linux one 9px); the download section's dead `--ratio`
     rule gone. Looked at 1440x900, 1280x720, 1366x700, 820x1180,
     390x844, 360 and 320 (Windows, Linux, Mac and Android), with and without JavaScript, `/download/`
     at 1440, and a build with no release.
      Checks on 21 November 2026 (`BASE=/` build): copy-lint 0 FAIL, 0 WARN
      on the home page, download, the guides and the first guide (the
      merge had one WARN, three "isn't"); `perf.mjs` 0 FAIL, 3 WARN (icon.svg
      not lazy at both widths, and the size shot's 580px box holding its
      310px file, which `scale-down` draws at its own size), LCP the hero
      image; `check.mjs` at five sizes finds `#features` (1.46 screens at
      1440, 2.06 at 768x1024 and on a phone), `#download` shorter than a
      screen and the neighbours they let show, and the hero window's frame
      reaching 32px into the gutters (`spill`, at 1440x900 and 1024x768),
      by design, so its light isn't clipped; `consistency.mjs --quick`
      14 FAIL (17 after the merge): at 1440 `section-height` 5 and
      `one-section` 2 from `#features` and `#download`'s heights,
      `one-heading` on Lite's heading, `container-edges` on the hero's
      window, the rail's `row-lines`, `no-visual` on `#download` (its crops
      cover 14%) and `text-only` on `#faq`, the last two at 390 too.
      `.taste-skill/waivers.json` (git-ignored; only `gate.mjs` reads it)
      now waives three: `row-lines` on `figure.kinds > ul.kinds__rail` (its
      frames start inside the handles), `container-edges` on
      `div.hero__window > div.ly-piece` (the window drawn whole on a screen
      840 to 990px tall) and `section-height` on `section#download` (as tall
      as it holds where its list starts closed). The rail's `one-section`
      no longer fires, so its waiver went.
      *Done when:* the owner has decided `#features`' rhythm (as in 1.14)
      and a picture for `#download` and `#faq`, and the gate passes.
- [ ] **1.16 The home page after the third critique** (21 November 2026,
      27/36; `.impeccable/critique/2026-11-21T20-30-46Z__site-src-pages-index-astro.md`).
      Four branches of their own, merged at `bb8dd66`, then a polish pass:
  1. Phones, tablets and Macs (`data-lite`, `site-r3-lite`): an inline
     script at the end of `#lite` moves that section before `#download`
     in the DOM (not CSS `order`, so focus follows what is seen; without
     JavaScript and on Windows or Linux the page stays as written).
     `#download` is headed "The desktop app" there, and
     `.downloads__lite` is one line on getting it onto a computer. The
     hero's lead says Share hands the clip to Discord on a phone, and the
     FAQ says "Add it to Sub20".
  2. The payoff (`site-r3-proof`): the download section's frame is three
     steps for one game clip, `clutch-1v4-ranked`, 248 MB to 19.4 MB: the
     plan row from a new `plan.webp`, the file chip from `done.webp`, then
     "Copy it, then paste it in Discord" with Copy and Ctrl+C cropped from
     `done.webp`. New `watch.webp` and `watch-narrow.webp` show a Windows
     path, the watch card says "In the desktop app", and the mock's demo
     paths are Windows ones (`VIDEOS`, `inFolder` in `src/lib/mock.ts`).
  3. Windows's warning (`site-r3-warn`): under the hero's and the close's
     Windows button the short form, "If Windows stops the installer, click
     More info, then Run anyway." and "Install steps"; the full sentence
     stays in the download section and the FAQ.
  4. Spacing and small fixes (`site-r3-layout`): `#features` and
     `#download` end at their own words where the next section shares
     their ground (the 192px void before `#download` at 1440 is 117px);
     Lite's link as wide as its words; the feature cards' title and lines
     columns rebalanced; the pill's changelog link reads as a link; the
     phone's hero window hides its clip strip when cut; Lite's phone shot
     has no fade at its foot.
  5. The polish pass (`site-r3-polish`): the size picker drawn 372px wide
     (as tall as its panel's inside; it was 310px, half its panel) from
     its 620px file, the scale of History and Options (832px files at
     479px), and back on the cards' one ground (round 3's own ground for
     it was a stand-in), so all four panels match and a sheet shows as a
     box in each. At a 2x display it is 1.2 times its file, as History and
     Options are 1.15. Share is said once a page: `#lite`'s lead leaves it
     out on `data-lite`, where the hero says it. The download steps' words
     at 7:1 (they were 6.9, `--p-muted` on the frame). Stale comments
     fixed. "Open Sub20 Lite" stays five times on a phone: the nav, the
     hero and the close are the kit's main action in its three places,
     `#lite` has its own link, and the footer is the same site map on
     every page and system.
      Looked at 1440x900, 1280x720, 1366x700 (Windows and Mac), 820x1180
      (iPad), 390x844 (Android, and Windows), 1440 and 390 without
      JavaScript, `/download/` at 1440, and a build with no release. The
      hero's foot is above the fold at 1280x720 and 1366x700 on Windows
      and Mac (680 of 720, 660 of 700); at 1440x900 the window is 901px
      wide under Windows's note and 943px elsewhere. On `data-lite` focus
      runs through `#lite` and then `#download`, and `/#download` or the
      hero's "Download for Windows or Linux" opens the file list. No
      horizontal scroll at any size; measured text is 6.35:1 or more (the
      rail's 12px labels the lowest).
      Checks on 21 November 2026 (`BASE=/` build): copy-lint 0 FAIL,
      0 WARN on the home page, download, the guides and the first guide;
      `perf.mjs` 0 FAIL, 3 WARN (icon.svg not lazy at both widths, the
      size shot's 580px box holding its 372px slot); `check.mjs` at five
      sizes finds what 1.15 found: `#features` (1.40 screens at 1440, 2.04
      at 768x1024), `#download` shorter than a screen, the neighbours they
      let show, and the hero window's `spill`; `consistency.mjs --quick`
      15 FAIL (17 before the polish pass: the download steps' contrast at
      both widths): at 1440 `section-height` 6 and `one-section` 3 from
      `#features` and `#download`'s heights (one of them `#features`'
      cards ending under its bottom padding, which round 3 set to 0),
      `container-edges` on the hero's window, the rail's `row-lines`,
      `no-visual` on `#download` (its three steps cover 9%) and
      `text-only` on `#faq`, the last two at 390 too. WARNs worth knowing:
      the hero note's words sit 8px left of the middle (its link's arrow
      makes up the rest) and, in a Windows window 390px wide, its second
      line sits over the window frame's faded top. `one-heading` on Lite's
      heading no longer fires. `npx svelte-check`: 0 errors, 0 warnings
      after the mock's change. `.taste-skill/waivers.json` waives the same
      three, the hero window's reason updated (901 and 943px at
      1440x900) and the download section's naming Lite's place.
      *Done when:* the owner has decided `#features`' rhythm and a
      picture for `#download` and `#faq` (as in 1.14 and 1.15), and the
      gate passes.
- [ ] **1.17 The home page after the fourth critique** (21 November 2026,
      28/36 with no P0 or P1;
      `.impeccable/critique/2026-11-21T22-05-33Z__site-src-pages-index-astro.md`).
      The owner chose to do the payoff first, to show the clip arriving
      in a chat, and to do everything. Three branches from `c8d5888`,
      merged locally:
  1. The payoff (`site-r4-payoff`):
     - **The story.** It is written once, in `PayoffStory.astro`. Its
       third step is no longer the cropped Copy button, which read as a
       dead control, with a faded chip slice beside it. It is now the clip
       arriving in a chat, drawn in the site's own palette: a "You"
       initial, then an attachment with the chip's thumbnail from
       `done.webp`, its name and 19.4 MB. It is `role="img"`, with no
       Discord colours or marks.
     - **Lite's story.** On `data-lite`, `#lite` tells it in place of the
       phone screenshot, ending "Tap Share, then pick Discord"
       (`pointer: coarse`) or "Download it, then drop it in Discord" (a
       Mac). `#download` is then the heading, its line and the file list.
     - **The file list.** From 1024px "Other systems and downloads" sits
       32px under the warning at 1440 (it was 182px). Opened, it runs the
       full width under both columns, checked in Edge only.
     - **Install steps.** "Read the install steps" has a 29px target.
     - **Renamed classes.** `.downloads__frame`, `__story` and `__crop`
       are now `.payoff`, `.payoff__story` and `.payoff__crop`.
  2. The questions (`site-r4-faq`):
     - The FAQ asks about Windows's warning third and ends on "Are my
       videos uploaded anywhere?", with the wording unchanged.
     - `#faq` is in reading order in the DOM (heading, answers, guides)
       and is laid out in two columns from 1024px. On a phone the answers
       come first, at two screens (1712px), as before.
  3. The hero's note and the close (`site-r4-small`):
     - The note sits 9px under the hero's button, so the focus ring
       clears it.
     - "Install steps" has a 25px target.
     - Under 640px the close's big button centres its words with their
       arrow or mark.
     - On `data-lite` the hero's "Download for Windows or Linux" is a
       text link (`.hero__other`), not a secondary button.
      Not done, because they were judged wrong:
      - The critique asked for an "In the desktop app" tag on the GPU
        card, but Lite encodes on the GPU too, with a CPU fallback.
      - It said the Mac's route is missing from `#lite`, but its second
        bullet already names Chrome or Edge.
      Checks on 21 November 2026 (`BASE=/` build):
      - **copy-lint:** 0 FAIL, 0 WARN on the home page.
      - **consistency.mjs --quick:** 14 FAIL (15 after round 3). They are
        `#features`' rhythm moving every later section off the screen
        grid, the three waived findings, `no-visual` on `#download` at
        1440 (its story covers 11%) and `text-only` on `#faq`.
      - **check.mjs** at five sizes: the same rhythm findings only.
      - **Safari:** not checked. Playwright's WebKit download stalled.
      Pushed and published on 21 November 2026 (Pages run 44975210153).
      **The owner's decisions after it:**
      - `#features` is the kit's two screens (`.g2`). The site's override
        to its cards' height is gone, and the extra height goes to the
        shots' row.
      - The 1280px rule keeps a section at least its `--h` screens.
      - `no-visual` on `#download` and `text-only` on `#faq` are waived,
        so five waivers are used, the limit.
      **consistency.mjs --quick after them:** 12 FAIL.
      - 7 come from `#download` at 0.73 screens (657px at 1440x900):
        its height, Lite's, the questions' and the close's starts, and
        three `one-section`.
      - The rest are the five waived findings.
      - A trial at a full screen left 315px of empty ground under the
        story at 1440, and on a phone a screen holding three lines, so
        it isn't in.
      *Done when:* the owner has decided `#download`'s height, and the
      gate passes.

## Needs the owner

- [x] **Enable GitHub Pages:** repository Settings → Pages → Source:
      "GitHub Actions".
- [x] **Say when to push** (one push with everything in 1.3 to 1.10).
- [ ] **Get the site indexed** (for 1.13): a Search Console URL-prefix
      property for `https://y4dd.github.io/Sub20/` (an HTML-tag token goes
      into `Base.astro`'s head; the AI can add it), then submit
      `sitemap-index.xml` there and ask for the home page and the guides to
      be indexed. The same in Bing Webmaster Tools, which ChatGPT search
      and Copilot draw on. The subdomain is set aside for now (owner,
      17 November 2026), so this is done on `y4dd.github.io/Sub20`; a move
      later means a new property and a redirect.
      17 November 2026: Google verified (the tag in `Base.astro`), the
      sitemap submitted at `/Sub20/sitemap-index.xml` (Search Console
      showed "Couldn't fetch" at first, while URL inspection said it was
      available to Google), indexing requested for the home page,
      `compress-video-for-discord`, `file-too-large`, the guides page and
      download. 18 November 2026: indexing requested for the other three
      guides; Bing Webmaster Tools set up (imported from Search Console,
      sitemap and URLs in); nothing indexed by Google yet, and its
      Sitemaps report still says "Couldn't fetch" on day two. Next: add
      `sitemap-0.xml` beside the index, then watch Indexing → Pages for a
      week or two.
- [ ] Optional, any time before plan 5: buy a domain (a `.app` name is
      about $15 a year). Until then the address is `y4dd.github.io/Sub20`.
      15 November 2026: the owner has `amjad.sh` (Porkbun) and would use a
      subdomain. Then: a CNAME record at Porkbun pointing it at
      `y4dd.github.io`, the custom domain set in Pages, and the Pages
      workflow run again (it reads the address from Pages, so no code
      changes; `BASE` becomes `/`). Carried to plan 5's task 5.4 when
      this plan closed.

## Exit criteria

- [x] The finish gate passes and Lighthouse on a phone is 95 or more.
      (Locally: home 95, the guide 97. Seen again live in 1.11.)
- [x] The download links read the latest release at build time, and fall
      back to the releases page when there is none. (Seen with a real
      release in plan 2, task 2.8.)
      Seen with four releases, 15 November 2026: after each was published
      the live download page named its version (22 links to 0.9.3).
- [x] Every guide is accurate against the current app.
      Read again 15 November 2026 for 0.9.3: the HEVC guide, the iPhone
      guide and the FAQ say HEVC and AV1 may not play on every device.
- [x] The site is live on Pages with Lite at `/app`.
