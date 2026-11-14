<script lang="ts">
  // The selected clip: a preview, the trim timeline, and one line that says
  // what will happen (or is happening, or happened).
  import { untrack } from "svelte";
  import { app, backend, type Clip } from "../app.svelte";
  import { baseName, bitrate, clock, engineOf, megabytes, parseClock, plain, remaining, resolution, span, BIN, CODEC_LABEL } from "../format";
  import Icon from "./Icon.svelte";
  import Timeline from "./Timeline.svelte";

  let { clip }: { clip: Clip } = $props();

  /** The original's player, and the result's. */
  let video = $state<HTMLVideoElement | null>(null);
  let resultVideo = $state<HTMLVideoElement | null>(null);
  /** The window cannot play this format; show still frames instead. */
  let stills = $state(false);
  let still = $state<string | null>(null);
  let playing = $state(false);
  let playhead = $state(0);
  let showLog = $state(false);
  let stillTimer = 0;
  let stillTicket = 0;

  const url = $derived(backend.videoUrl(clip.path));

  // A finished clip opens on its result: that is what is about to be sent.
  let view = $state<"original" | "result">("original");
  let resultSrc = $state<string | null>(null);
  /** The result can't play here; show still frames of it. */
  let resultStills = $state(false);

  const onResult = $derived(view === "result" && !!clip.result);
  /** What the player and the timeline show: the original and its trim, or
   *  the finished result and the part of it to keep. */
  const track = $derived(
    onResult && clip.result
      ? { path: clip.result.output, duration: clip.result.duration, start: clip.cutStart, end: clip.cutEnd }
      : { path: clip.path, duration: clip.duration, start: clip.start, end: clip.end },
  );
  const player = $derived(onResult ? resultVideo : video);
  const showingStills = $derived(onResult ? resultStills : stills);

  // A different clip.
  $effect(() => {
    void clip.key;
    untrack(() => {
      showLog = false;
      stills = url === null;
    });
  });

  // A result appears or goes: show it, or the original.
  $effect(() => {
    void clip.key;
    const finished = !!clip.result;
    untrack(() => (view = finished ? "result" : "original"));
  });

  // A different clip or view: stop, and start over at the kept part's start.
  $effect(() => {
    void clip.key;
    void view;
    untrack(() => {
      video?.pause();
      resultVideo?.pause();
      playing = false;
      still = null;
      playhead = track.start;
      if (showingStills) loadStill(playhead, 0);
    });
  });

  // Where the result plays from. A cut keeps the file's name but changes the
  // file, so it loads again; while cutting, the file is let go.
  $effect(() => {
    const output = clip.result?.output ?? null;
    const cutting = clip.cutting;
    untrack(() => {
      resultSrc = null;
      resultStills = false;
    });
    if (!output || cutting) return;
    const target = clip;
    void backend
      .resultUrl(output)
      .then((src) => {
        if (target !== clip || clip.result?.output !== output) return;
        resultSrc = src;
        if (src) return;
        resultStills = true;
        if (onResult) loadStill(playhead, 0);
      })
      .catch(() => (view = "original"));
  });

  // The result's own filmstrip and keyframes, the first time it is shown.
  $effect(() => {
    if (onResult && clip.result) void app.loadResultTrack(clip);
  });

  function loadStill(time: number, delay = 90) {
    window.clearTimeout(stillTimer);
    const ticket = ++stillTicket;
    const target = clip;
    const path = track.path;
    stillTimer = window.setTimeout(async () => {
      try {
        const frame = await backend.previewFrame(path, time);
        if (ticket === stillTicket && target === clip) still = frame;
      } catch {
        /* Keep the previous frame. */
      }
    }, delay);
  }

  export function seek(time: number, scrubbing = false) {
    time = Math.min(track.duration, Math.max(0, time));
    playhead = time;
    if (scrubbing && playing) player?.pause();
    if (showingStills) loadStill(time);
    else if (player && Math.abs(player.currentTime - time) > 0.01) player.currentTime = time;
  }

  export function toggle() {
    if (showingStills || !player || clip.busy || clip.cutting) return;
    if (player.paused) {
      // Play the part that will be kept, from wherever the playhead is in it.
      if (playhead < track.start || playhead >= track.end - 0.05) player.currentTime = track.start;
      void player.play();
    } else {
      player.pause();
    }
  }

  /** Moves the original's trim, or the result's cut. */
  function trim(start: number, end: number) {
    if (onResult) app.setCut(clip, start, end);
    else app.setTrim(clip, start, end);
  }

  /** Moves the nearer trim edge to the playhead. */
  export function mark(edge: "start" | "end") {
    if (edge === "start") trim(Math.min(playhead, track.end), track.end);
    else trim(track.start, Math.max(playhead, track.start));
  }

  /** Puts the playhead on the kept part's start or end. */
  export function edge(which: "start" | "end") {
    seek(which === "start" ? track.start : track.end, true);
  }

  export function nudge(seconds: number) {
    seek(playhead + seconds, true);
  }

  // While playing, follow the video and loop inside the kept part.
  $effect(() => {
    if (!playing || !player) return;
    const element = player;
    let frame = requestAnimationFrame(function follow() {
      playhead = element.currentTime;
      if (element.currentTime >= track.end) element.currentTime = track.start;
      frame = requestAnimationFrame(follow);
    });
    return () => cancelAnimationFrame(frame);
  });

  function commit(edge: "start" | "end", text: string) {
    const time = parseClock(text);
    if (time === null) return;
    if (edge === "start") app.setTrim(clip, time, Math.max(clip.end, time));
    else app.setTrim(clip, Math.min(clip.start, time), time);
    seek(edge === "start" ? clip.start : clip.end, true);
  }

  const facts = $derived.by(() => {
    const info = clip.info;
    const parts = [`${info.displayWidth}×${info.displayHeight}`];
    if (info.fps) parts.push(`${plain(info.fps, 2)} fps`);
    parts.push(megabytes(info.sizeBytes));
    // HDR is turned into ordinary colour, so it doesn't play grey in Discord.
    if (info.hdr && !app.options.audioOnly) {
      parts.push(app.report && !app.report.toneMap ? "HDR (this FFmpeg can't convert it, it may look grey)" : "HDR, converted to ordinary colour");
    }
    // Several sound tracks usually means game and microphone kept apart.
    if (info.audioTracks > 1 && (app.options.audio !== "none" || app.options.audioOnly)) {
      parts.push(`${info.audioTracks} sound tracks, ${app.options.mixAudio ? "mixed" : "first only"}`);
    }
    return parts.join(", ");
  });

  const LOOK = {
    sharp: "Should look sharp",
    good: "Should look good",
    soft: "A little soft",
    rough: "Rough: trim it shorter or raise the size",
  } as const;

  const thumb = $derived(clip.thumbs[Math.floor(clip.thumbs.length / 2)] || null);
  const copyKey = navigator.platform.toLowerCase().includes("mac") ? "⌘C" : "Ctrl+C";
  // Where there is no clipboard for files (Sub20 Lite), the same button
  // shares (phones) or downloads.
  const handOff = {
    copy: { icon: "copy", label: "Copy", done: "Copied" },
    share: { icon: "share", label: "Share", done: "Shared" },
    download: { icon: "download", label: "Download", done: "Downloaded" },
  } as const;
  const hand = $derived(handOff[app.handOff]);
</script>

{#snippet transport()}
  <button type="button" class="transport" onclick={toggle} aria-label={playing ? "Pause" : "Play the kept part"} title="Play or pause (Space)">
    <Icon name={playing ? "pause" : "play"} size={16} />
    <span>{clock(playhead)}</span>
  </button>
{/snippet}

<!-- Keeps the clip on the bench but out of the next Compress, or back in. -->
{#snippet leaveOut()}
  {#if clip.leftOut}
    <span class="muted">Left out of Compress.</span>
    <button type="button" class="link" onclick={() => app.toggleLeftOut(clip)} title="Include (S)">Include</button>
  {:else}
    <button type="button" class="link quiet" onclick={() => app.toggleLeftOut(clip)} title="Leave out of Compress (S)">Leave out</button>
  {/if}
{/snippet}

<section class="stage" aria-label={clip.name}>
  <header>
    <div class="title">
      <h1 title={clip.path}>{clip.name}</h1>
      <p class="muted">{facts}</p>
    </div>
  </header>

  <div class="well">
    {#if showLog}
      <pre class="log" aria-label="FFmpeg log">{clip.log.join("\n") || "Nothing logged yet."}</pre>
    {:else if onResult}
      <!-- The result plays in the same player as the original, over its own
           timeline, so it can be sought and cut. -->
      {#if resultStills}
        {#if still}
          <img class="picture" src={still} alt="" draggable="false" />
        {/if}
      {:else if resultSrc}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          class="picture"
          bind:this={resultVideo}
          src={resultSrc}
          preload="auto"
          playsinline
          onclick={toggle}
          onplay={() => (playing = true)}
          onpause={() => (playing = false)}
          onloadedmetadata={() => seek(playhead)}
          onerror={() => {
            resultStills = true;
            loadStill(playhead, 0);
          }}
        ></video>
        {@render transport()}
      {/if}
    {:else if stills}
      {#if still}
        <img class="picture" src={still} alt="" draggable="false" />
      {/if}
      {#if !backend.demo}
        <p class="caption">This format can't play here, so the preview shows single frames.</p>
      {/if}
    {:else if url}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        class="picture"
        bind:this={video}
        src={url}
        preload="auto"
        playsinline
        onclick={toggle}
        onplay={() => (playing = true)}
        onpause={() => (playing = false)}
        onloadedmetadata={() => seek(playhead)}
        onerror={() => {
          stills = true;
          loadStill(playhead, 0);
        }}
      ></video>
      {@render transport()}
    {/if}
    {#if clip.result && !clip.result.replacedOriginal && !showLog}
      <div class="views" role="group" aria-label="Preview">
        <button type="button" aria-pressed={view === "original"} onclick={() => (view = "original")}>Original</button>
        <button type="button" aria-pressed={view === "result"} onclick={() => (view = "result")}>Result</button>
      </div>
    {/if}
  </div>

  {#if onResult && clip.result}
    <!-- The result's own handles: a cut keeps the part between them. -->
    <Timeline
      duration={clip.result.duration}
      start={clip.cutStart}
      end={clip.cutEnd}
      thumbs={clip.resultThumbs}
      {playhead}
      settled={!backend.can.cut}
      locked={!backend.can.cut || clip.keyframes === null || clip.cutting}
      stops={clip.keyframes}
      onseek={seek}
      ontrim={(start, end) => app.setCut(clip, start, end)}
    />
  {:else}
    <Timeline
      duration={clip.duration}
      start={clip.start}
      end={clip.end}
      thumbs={clip.thumbs}
      {playhead}
      busy={clip.busy}
      percent={clip.percent}
      settled={clip.status === "done"}
      locked={!!clip.result?.replacedOriginal}
      onseek={seek}
      ontrim={(start, end) => app.setTrim(clip, start, end)}
    />
  {/if}

  <!-- Once done, the trim is settled and the result needs the room. -->
  <div class="under" class:done={clip.status === "done"}>
    <label class="edge">
      <span class="muted">From</span>
      <input
        value={clock(clip.start)}
        disabled={clip.busy}
        aria-label="Start time"
        onchange={(event) => commit("start", event.currentTarget.value)}
        onkeydown={(event) => event.key === "Enter" && event.currentTarget.blur()}
      />
    </label>

    <div class="status" aria-live="polite">
      {#if clip.status === "queued"}
        Waiting for its turn.
        <button type="button" class="link" onclick={() => app.cancel(clip)}>Cancel</button>
      {:else if clip.status === "running"}
        {#if clip.phase === "analyzing" || !clip.encoder}
          Reading the video…
        {:else if clip.phase === "finalizing"}
          Finishing the file…
        {:else}
          {#if clip.retries > 0}
            Came out too big, so it is being compressed again.
          {:else if clip.phase === "analysis"}
            Studying the video, first of two passes.
          {:else}
            Compressing on the {engineOf(clip.encoder)}.
          {/if}
          <strong>{Math.floor(clip.percent)}%</strong>{#if clip.eta != null && clip.percent > 2}, {remaining(clip.eta)}{/if}.
        {/if}
        {#if clip.fallback}
          <span class="muted">The graphics card could not do this one, so the CPU took over.</span>
        {/if}
        <button type="button" class="link" onclick={() => app.cancel(clip)}>Cancel</button>
      {:else if clip.status === "done" && clip.result && onResult && (clip.cutPending || clip.cutting)}
        {@const result = clip.result}
        {@const kept = clip.cutEnd - clip.cutStart}
        <!-- What the cut keeps. It copies the file, so nothing is lost. -->
        <ul class="plan" aria-label="What the cut keeps">
          <li><b>{span(kept)}</b><span class="muted"> of {span(result.duration)}</span></li>
          <li>About <b>{megabytes((result.sizeBytes * kept) / Math.max(0.001, result.duration))}</b></li>
          <li class="muted">Same quality</li>
        </ul>
        <p class="after">
          {#if clip.cutting}
            <span class="muted">Cutting the result…</span>
          {:else}
            <button type="button" class="link" onclick={() => app.cut(clip)}>Cut</button>
            <button type="button" class="link quiet" onclick={() => app.setCut(clip, 0, result.duration)}>Keep all of it</button>
          {/if}
        </p>
      {:else if clip.status === "done" && clip.result}
        {@const result = clip.result}
        <div class="result">
          <!-- The file itself: drag it into Discord, or click to play it. -->
          <button
            type="button"
            class="file"
            draggable={backend.can.dragOut}
            ondragstart={(event) => {
              event.preventDefault();
              void app.dragOut(clip);
            }}
            onclick={() => backend.openFile(result.output).catch((e) => app.notify(String(e)))}
            title={backend.can.dragOut ? "Drag into Discord or a folder, or click to play" : "Click to play"}
          >
            <span class="thumb">{#if thumb}<img src={thumb} alt="" draggable="false" />{/if}</span>
            <span class="words">
              <span class="name">{baseName(result.output)}</span>
              <span class="size"><Icon name="check" size={14} /><span><strong>{megabytes(result.sizeBytes)}</strong>, down from {megabytes(result.originalBytes)}</span></span>
            </span>
            {#if backend.can.dragOut}
              <span class="grip" aria-hidden="true"><Icon name="grip" size={16} /></span>
            {/if}
          </button>
          <button type="button" class="copy" class:copied={clip.copied} onclick={() => app.copy(clip)}>
            <Icon name={clip.copied ? "check" : hand.icon} size={16} />
            {clip.copied ? hand.done : hand.label}
            {#if app.handOff === "copy"}<kbd>{copyKey}</kbd>{/if}
          </button>
        </div>
        <p class="after">
          {#if backend.can.files}
            <button type="button" class="link" onclick={() => backend.reveal(result.output).catch((e) => app.notify(String(e)))}>Show in folder</button>
          {/if}
          {#if result.replacedOriginal}
            <span class="muted">The original is in the {BIN}.</span>
          {:else}
            <button type="button" class="link quiet" onclick={() => { app.reset(clip); app.replan(clip, 0); }}>Compress again</button>
          {/if}
          {#if clip.log.length > 0}
            <button type="button" class="link quiet" onclick={() => (showLog = !showLog)}>{showLog ? "Hide log" : "Show log"}</button>
          {/if}
        </p>
      {:else if clip.status === "failed"}
        <span class="bad">{clip.error}</span>
        <button type="button" class="link" onclick={() => { app.reset(clip); app.replan(clip, 0); }}>Try again</button>
        <p class="after">{@render leaveOut()}</p>
      {:else if clip.planError}
        <span class="bad">{clip.planError}</span>
      {:else if clip.plan}
        <!-- What will come out, as facts rather than a sentence. -->
        <ul class="plan" aria-label="What will come out">
          {#if clip.status === "canceled"}<li class="muted">Canceled</li>{/if}
          <li><b>{span(clip.kept)}</b>{#if clip.trimmed}<span class="muted"> of {span(clip.duration)}</span>{/if}</li>
          {#if app.options.audioOnly}
            <li>Sound only, about <b>{megabytes(clip.plan.expectedBytes)}</b></li>
          {:else}
            <li><b>{resolution(clip.plan.width, clip.plan.height)}</b></li>
            <li class="muted">{CODEC_LABEL[app.options.codec]}, {bitrate(clip.plan.videoKbps)}</li>
            <li class="verdict {clip.plan.quality}"><i aria-hidden="true"></i>{LOOK[clip.plan.quality]}</li>
          {/if}
        </ul>
        <p class="after">
          {#if clip.trimmed}
            <button type="button" class="link quiet" onclick={() => app.setTrim(clip, 0, clip.duration)}>Keep all of it</button>
          {/if}
          {@render leaveOut()}
        </p>
      {:else}
        &nbsp;
      {/if}
      {#if clip.log.length > 0 && clip.status !== "ready" && clip.status !== "done"}
        <button type="button" class="link quiet" onclick={() => (showLog = !showLog)}>{showLog ? "Hide log" : "Show log"}</button>
      {/if}
    </div>

    <label class="edge end">
      <span class="muted">to</span>
      <input
        value={clock(clip.end)}
        disabled={clip.busy}
        aria-label="End time"
        onchange={(event) => commit("end", event.currentTarget.value)}
        onkeydown={(event) => event.key === "Enter" && event.currentTarget.blur()}
      />
    </label>
  </div>
</section>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    padding: 6px 24px 14px;
    gap: 14px;
  }

  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    padding: 0 16px;
  }

  .title {
    min-width: 0;
  }

  h1 {
    font-family: var(--display);
    font-stretch: 88%;
    font-size: var(--step-3);
    font-weight: 600;
    letter-spacing: -0.012em;
    line-height: 1.15;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .well {
    position: relative;
    flex: 1;
    min-height: 120px;
    margin: 0 16px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 10px;
    background: var(--well);
  }

  .picture {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  video.picture {
    cursor: pointer;
  }

  .transport {
    position: absolute;
    left: 12px;
    bottom: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 12px 0 10px;
    border-radius: 16px;
    background: rgb(12 13 16 / 0.72);
    backdrop-filter: blur(8px);
    color: #fff;
  }

  .transport:hover {
    background: rgb(12 13 16 / 0.9);
  }

  .caption {
    position: absolute;
    left: 12px;
    bottom: 12px;
    padding: 5px 10px;
    border-radius: 6px;
    background: rgb(12 13 16 / 0.72);
    color: #d5d8de;
    font-size: var(--step--1);
  }

  .log {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: 14px 16px;
    overflow: auto;
    color: #c9cdd6;
    font: 12px/1.5 ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, monospace;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }

  .under {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: baseline;
    gap: 20px;
    padding: 0 16px;
    min-height: 44px;
  }

  .under.done {
    grid-template-columns: minmax(0, 1fr);
  }

  .under.done .edge {
    display: none;
  }

  .edge {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Always drawn as a field, so it is plain that a time can be typed. */
  .edge input {
    width: 9ch;
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--surface);
    font-weight: 600;
    text-align: center;
  }

  .edge input:hover:not(:disabled) {
    border-color: var(--line-strong);
  }

  .edge input:disabled {
    background: none;
    border-color: transparent;
  }

  .edge input:focus {
    border-color: var(--line-strong);
    background: var(--surface);
    outline: none;
  }

  .status {
    text-align: center;
    text-wrap: balance;
    max-width: 78ch;
    justify-self: center;
  }

  .status .link {
    margin-left: 6px;
  }

  .bad {
    color: var(--bad);
  }

  /* The plan: facts in a row, divided by hairlines. */
  .plan {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .plan li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 12px;
    white-space: nowrap;
  }

  .plan li + li {
    border-left: 1px solid var(--line);
  }

  .plan b {
    font-weight: 650;
  }

  .verdict i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
  }

  .verdict.soft i {
    background: var(--trim-mark);
  }

  .verdict.rough {
    color: var(--bad);
  }

  .verdict.rough i {
    background: var(--bad);
  }

  /* Original / Result, over the preview. */
  .views {
    position: absolute;
    top: 12px;
    right: 12px;
    display: flex;
    padding: 3px;
    border-radius: 10px;
    background: rgb(12 13 16 / 0.72);
    backdrop-filter: blur(8px);
  }

  .views button {
    height: 28px;
    padding: 0 12px;
    border-radius: 7px;
    color: #c9cdd6;
    font-weight: 600;
  }

  .views button:hover {
    color: #fff;
  }

  .views button[aria-pressed="true"] {
    background: #fff;
    color: #14161b;
  }

  /* The finished file, ready to go. */
  .result {
    display: flex;
    justify-content: center;
    align-items: stretch;
    gap: 8px;
  }

  .file {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    max-width: 440px;
    padding: 6px 10px 6px 6px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: var(--surface);
    text-align: left;
    cursor: grab;
    transition: transform 120ms var(--ease), border-color 120ms;
  }

  .file:hover {
    border-color: var(--ok);
    transform: translateY(-1px);
  }

  .file:active {
    cursor: grabbing;
    transform: none;
  }

  .file .thumb {
    flex: none;
    width: 54px;
    height: 36px;
    overflow: hidden;
    border-radius: 5px;
    background: var(--well);
  }

  .file .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .file .words {
    display: grid;
    min-width: 0;
  }

  .file .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .file .size {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: var(--step--1);
    white-space: nowrap;
  }

  .file .size :global(svg),
  .file .size strong {
    color: var(--ok);
  }

  .file .grip {
    margin-left: auto;
    color: var(--faint);
  }

  .copy {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    font-weight: 650;
    white-space: nowrap;
  }

  .copy:hover {
    background: var(--raised);
  }

  .copy:active {
    transform: scale(0.98);
  }

  .copy.copied {
    border-color: var(--ok);
    color: var(--ok);
  }

  kbd {
    padding: 1px 6px;
    border: 1px solid var(--line);
    border-radius: 5px;
    color: var(--muted);
    font: inherit;
    font-size: var(--step--1);
    font-weight: 500;
  }

  .after {
    margin-top: 6px;
  }

  .after .link:first-child {
    margin-left: 0;
  }

  /* Narrow: less margin, the two times on one row and the words under them. */
  @media (max-width: 720px) {
    /* Clipped: the trim handles' grab areas reach past the screen's edge,
       and a phone would widen the whole page to show them. */
    .stage {
      padding: 4px 0 10px;
      gap: 10px;
      overflow: clip;
    }

    .under {
      grid-template-columns: auto auto;
      justify-content: space-between;
      gap: 10px 12px;
    }

    .edge.end {
      justify-self: end;
    }

    .status {
      grid-column: 1 / -1;
      grid-row: 2;
    }

    .result {
      flex-wrap: wrap;
    }

    .file {
      flex: 1 1 100%;
      max-width: none;
    }

    .copy {
      flex: 1;
      justify-content: center;
      min-height: 44px;
    }

    kbd {
      display: none;
    }
  }
</style>
