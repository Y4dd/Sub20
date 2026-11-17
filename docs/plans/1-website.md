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

## Needs the owner

- [x] **Enable GitHub Pages:** repository Settings → Pages → Source:
      "GitHub Actions".
- [x] **Say when to push** (one push with everything in 1.3 to 1.10).
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
