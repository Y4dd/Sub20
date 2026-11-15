# Plan 4: Sub20 Lite

**Goal.** Lite keeps the promise the desktop app makes, every result under
the limit, on real clips in the browsers people use, and a phone video
goes from the gallery to Discord in under a minute. What Lite can't do yet
is either built or said on the screen.

**Status.** Built and committed (November 2026): the backend on WebCodecs
and Mediabunny, the planner through WebAssembly, the capability flags, the
install and share-target support, the phone layout. Tested on one machine
only: Chromium 156, an RTX 5070 Ti, six generated clips, all under on the
first try with constant bitrate ([record.md](record.md)).

**Needs first:** plan 1 for anything on a phone, because a phone needs
Lite served over HTTPS. **Runs beside:** plan 3. **Unblocks:** plan 5.

## The gate

The size test decides whether Lite ships at all. **If results can't be held
under the limit in Chrome, Edge and Safari, Lite does not launch:** the
site's Lite section, its nav word and its footer links come out, `/app`
stays unlinked, and plan 5 launches the desktop app alone. Tasks 4.1 to 4.3
are the gate; do them before the rest.

## Tasks

- [ ] **4.1 Real clips on this machine.** `/lite-lab.html` under
      `npm run dev`, with the owner's NVIDIA clips and whatever plan 3's
      task 3.2 collected (OBS, iPhone, Android), at 20 MB: how close each
      lands, how many tries, how long. Results into the record.
      Started 14 November 2026: 13 of the owner's NVIDIA clips, all under
      on the first try ([record.md](record.md)). Left: OBS, iPhone and
      Android clips from 3.2.
      *Done when:* every clip is under, or each miss has a cause and a fix.
- [ ] **4.2 A way to test on other devices.** The lab page is in neither
      build, and a phone can't run `npm run dev`. Use Lite itself: a short
      sheet in `docs/testing.md` (five clips, 20 MB, write down the size
      Lite shows for each, then send one to Discord and say whether it
      plays in the chat). If that proves too thin to find a miss's cause,
      publish the lab beside Lite at `/app/lab.html`.
      *Done when:* a tester can run it on a phone without help.
      Written 14 November 2026: part 9 of `docs/testing.md`, six steps,
      labels taken from the components (the bracket says "Choose videos"
      on a phone, "Drop videos here" on a computer). Waits for Lite to be
      online and a tester's run.
- [ ] **4.3 Read the results** as they come in from the owner and testers
      (see below), and fix what misses. A browser that can't hold the limit
      after a fix is told so on Lite's first screen, as browsers without
      WebCodecs are today.
      *Done when:* Chrome, Edge and Safari hold the limit on computer and
      phone, on NVIDIA, Intel and AMD.
- [ ] **4.4 HDR.** Lite has no tone mapping, so an iPhone HDR clip comes
      out flat. First, say it: when the source is PQ or HLG, the stage
      warns that the colours may look washed out and that the desktop app
      converts them. Then try the conversion (a WebGL or canvas step
      between decode and encode, Mobius like the desktop), and keep it only
      if the colour measures close to the desktop's result.
      *Done when:* the warning is there, and the conversion is either in or
      moved to plan 6 with what was measured.
      Warning in, 14 November 2026: Lite keeps `info.hdr` and the stage
      says "HDR, the colours will come out flat here (the desktop app
      converts them)" (`can.toneMap`), checked on a generated PQ clip.
      Left: the conversion.
- [x] **4.5 Several sound tracks.** Lite keeps the first only, which loses
      a microphone recorded on its own track. First, say it on the stage
      ("2 sound tracks, only the first is kept"). Then mix them (decode
      each, sum with a limiter, as the desktop does).
      *Done when:* the line is there, and mixing is either in and heard in
      a test clip, or moved to plan 6.
      Done 14 November 2026: Lite mixes every track (`web/mix.ts`, the
      desktop's rule: stereo, summed at full volume, limited at 0.95), and
      Options shows "Mix all sound tracks" there too. A 44.1 kHz mono mic
      beside a silent game track came out at full volume (peak 0.131, as
      in the source), and two loud tracks peaked at 0.95. With mixing off
      the stage says "2 sound tracks, only the first is kept".
- [ ] **4.6 A planner that knows the GPU's limits.** Today Lite plans, then
      steps down the resolution ladder when the encoder refuses a size
      (1440p60 on this GPU). Ask the encoder first and plan within what it
      takes, so the stage's "1080p, good" is what will happen.
      *Done when:* the plan shown equals the encode done, on a clip that
      used to step down.
- [x] **4.7 Lighthouse and accessibility** on the built Lite, on a phone
      profile: installable, works offline, every control named.
      *Done when:* the PWA and accessibility checks pass.
      Done 14 November 2026, on `npm run build:web` served gzipped, in a
      phone profile. Lighthouse 12 (which has no PWA category any more):
      accessibility 100, best practices 100, performance 93 to 96 (LCP
      1.4 s). Installable and offline checked in headless Edge over the
      DevTools protocol: no manifest or installability errors, the worker
      controls the page, the page loads with the network off, and the
      planner's wasm and Mediabunny's chunk are in its cache. axe (WCAG 2.1
      AA and best practice) with a clip loaded, Options and Settings open,
      and a finished result: clean, after three fixes (the empty screen
      had no `h1`; the play button and the size limit had names that left
      out the time and size they show, so voice control couldn't find
      them).
- [ ] **4.8 Large results on disk.** Results are blobs in memory, fine at
      20 MB and not at 500 MB or 1 GB on a phone. Write them to the origin
      private file system. Not needed for the gate; do it if plans 3 and 4
      are both waiting on people, otherwise it moves to plan 6.
- [ ] **4.9 Other codecs.** VP9 or AV1 in WebM where H.264 can't be
      encoded, Opus where AAC can't. Same standing as 4.8.

## Needs the owner and testers

- [ ] **Browsers:** Chrome, Edge and Firefox on a computer; Safari on
      macOS and iOS; Chrome on Android. Each runs the sheet from 4.2.
- [ ] **Cards:** one Intel and one AMD machine run it too.
- [ ] **Discord:** a result from Lite plays in the chat on desktop and on
      a phone.
- [ ] **A phone, start to finish:** a video from the gallery, through
      Share to an installed Lite on Android (or opened in Safari on iOS),
      trimmed, compressed, and handed to Discord with Share, timed.

## Exit criteria

- [ ] Results stay under the limit in Chrome, Edge and Safari, on computer
      and phone, on the test clips.
- [ ] A phone video goes from the gallery to Discord in under a minute.
- [x] Lighthouse's PWA and accessibility checks pass.
- [ ] HDR and several sound tracks are either handled or said on the
      stage, and the site's Lite section and the iPhone guide say the same.
