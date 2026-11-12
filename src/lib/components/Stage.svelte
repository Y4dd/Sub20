<script lang="ts">
  // The selected clip: a preview, the trim timeline, and one line that says
  // what will happen (or is happening, or happened).
  import { untrack } from "svelte";
  import { app, backend, type Clip } from "../app.svelte";
  import { baseName, bitrate, clock, engineOf, megabytes, parseClock, plain, remaining, resolution, span, BIN, CODEC_LABEL } from "../format";
  import Icon from "./Icon.svelte";
  import Timeline from "./Timeline.svelte";

  let { clip }: { clip: Clip } = $props();

  let video = $state<HTMLVideoElement | null>(null);
  /** The window cannot play this format; show still frames instead. */
  let stills = $state(false);
  let still = $state<string | null>(null);
  let playing = $state(false);
  let playhead = $state(0);
  let showLog = $state(false);
  let stillTimer = 0;
  let stillTicket = 0;

  const url = $derived(backend.videoUrl(clip.path));

  // A different clip: start over at its in point.
  $effect(() => {
    void clip.key;
    untrack(() => {
      playing = false;
      showLog = false;
      still = null;
      stills = url === null;
      playhead = clip.start;
      if (stills) loadStill(clip.start, 0);
    });
  });

  function loadStill(time: number, delay = 90) {
    window.clearTimeout(stillTimer);
    const ticket = ++stillTicket;
    const target = clip;
    stillTimer = window.setTimeout(async () => {
      try {
        const frame = await backend.previewFrame(target.path, time);
        if (ticket === stillTicket && target === clip) still = frame;
      } catch {
        /* Keep the previous frame. */
      }
    }, delay);
  }

  export function seek(time: number, scrubbing = false) {
    time = Math.min(clip.duration, Math.max(0, time));
    playhead = time;
    if (scrubbing && playing) video?.pause();
    if (stills) loadStill(time);
    else if (video && Math.abs(video.currentTime - time) > 0.01) video.currentTime = time;
  }

  export function toggle() {
    if (stills || !video || clip.busy) return;
    if (video.paused) {
      // Play the part that will be kept, from wherever the playhead is in it.
      if (playhead < clip.start || playhead >= clip.end - 0.05) video.currentTime = clip.start;
      void video.play();
    } else {
      video.pause();
    }
  }

  /** Moves the nearer trim edge to the playhead. */
  export function mark(edge: "start" | "end") {
    if (edge === "start") app.setTrim(clip, Math.min(playhead, clip.end), clip.end);
    else app.setTrim(clip, clip.start, Math.max(playhead, clip.start));
  }

  export function nudge(seconds: number) {
    seek(playhead + seconds, true);
  }

  // While playing, follow the video and loop inside the kept part.
  $effect(() => {
    if (!playing || !video) return;
    const element = video;
    let frame = requestAnimationFrame(function follow() {
      playhead = element.currentTime;
      if (element.currentTime >= clip.end) element.currentTime = clip.start;
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

  // A finished clip opens on its result: that is what is about to be sent.
  let view = $state<"original" | "result">("original");
  let resultSrc = $state<string | null>(null);
  let resultStill = $state<string | null>(null);

  $effect(() => {
    const output = clip.result?.output ?? null;
    untrack(() => {
      view = output ? "result" : "original";
      resultSrc = null;
      resultStill = null;
    });
    if (!output) return;
    const target = clip;
    void backend
      .resultUrl(output)
      .then((src) => {
        if (target !== clip) return;
        resultSrc = src;
        // No playable URL: show a frame from the middle of the result.
        if (!src) return backend.previewFrame(output, clip.kept / 2).then((frame) => (resultStill = frame));
      })
      .catch(() => (view = "original"));
  });

  // Showing the result pauses the original.
  $effect(() => {
    if (view === "result") untrack(() => video?.pause());
  });

  const thumb = $derived(clip.thumbs[Math.floor(clip.thumbs.length / 2)] || null);
  const copyKey = navigator.platform.toLowerCase().includes("mac") ? "⌘C" : "Ctrl+C";
</script>

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
    {:else if view === "result" && clip.result}
      {#if resultSrc}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video class="picture" src={resultSrc} controls preload="metadata" playsinline></video>
      {:else if resultStill}
        <img class="picture" src={resultStill} alt="" draggable="false" />
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
      <button type="button" class="transport" onclick={toggle} aria-label={playing ? "Pause" : "Play the kept part"} title="Play or pause (Space)">
        <Icon name={playing ? "pause" : "play"} size={16} />
        <span>{clock(playhead)}</span>
      </button>
    {/if}
    {#if clip.result && !clip.result.replacedOriginal && !showLog}
      <div class="views" role="group" aria-label="Preview">
        <button type="button" aria-pressed={view === "original"} onclick={() => (view = "original")}>Original</button>
        <button type="button" aria-pressed={view === "result"} onclick={() => (view = "result")}>Result</button>
      </div>
    {/if}
  </div>

  <Timeline {clip} {playhead} onseek={seek} />

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
      {:else if clip.status === "done" && clip.result}
        {@const result = clip.result}
        <div class="result">
          <!-- The file itself: drag it into Discord, or click to play it. -->
          <button
            type="button"
            class="file"
            draggable="true"
            ondragstart={(event) => {
              event.preventDefault();
              void app.dragOut(clip);
            }}
            onclick={() => backend.openFile(result.output).catch((e) => app.notify(String(e)))}
            title="Drag into Discord or a folder, or click to play"
          >
            <span class="thumb">{#if thumb}<img src={thumb} alt="" draggable="false" />{/if}</span>
            <span class="words">
              <span class="name">{baseName(result.output)}</span>
              <span class="size"><Icon name="check" size={14} /><span><strong>{megabytes(result.sizeBytes)}</strong>, down from {megabytes(result.originalBytes)}</span></span>
            </span>
            <span class="grip" aria-hidden="true"><Icon name="grip" size={16} /></span>
          </button>
          <button type="button" class="copy" class:copied={clip.copied} onclick={() => app.copy(clip)}>
            <Icon name={clip.copied ? "check" : "copy"} size={16} />
            {clip.copied ? "Copied" : "Copy"}
            <kbd>{copyKey}</kbd>
          </button>
        </div>
        <p class="after">
          <button type="button" class="link" onclick={() => backend.reveal(result.output).catch((e) => app.notify(String(e)))}>Show in folder</button>
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
        {#if clip.trimmed}
          <p class="after"><button type="button" class="link quiet" onclick={() => app.setTrim(clip, 0, clip.duration)}>Keep all of it</button></p>
        {/if}
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
</style>
