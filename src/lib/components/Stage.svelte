<script lang="ts">
  // The selected clip: a preview, the trim timeline, and one line that says
  // what will happen (or is happening, or happened).
  import { untrack } from "svelte";
  import { app, backend, type Clip } from "../app.svelte";
  import { baseName, bitrate, clock, engineOf, megabytes, parseClock, plain, remaining, resolution, span, CODEC_LABEL } from "../format";
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
    return parts.join(", ");
  });

  const LOOK = {
    sharp: "Should look sharp.",
    good: "Should look good.",
    soft: "Will look a little soft.",
    rough: "Will look rough. Trim it shorter or raise the size.",
  } as const;

  const overLimit = $derived(
    clip.result?.targetMb != null && clip.result.sizeBytes > clip.result.targetMb * 1024 * 1024,
  );
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
  </div>

  <Timeline {clip} {playhead} onseek={seek} />

  <div class="under">
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

    <p class="status" aria-live="polite">
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
          <span class="muted">The graphics card's encoder failed, so the CPU took over.</span>
        {/if}
        <button type="button" class="link" onclick={() => app.cancel(clip)}>Cancel</button>
      {:else if clip.status === "done" && clip.result}
        {#if overLimit}
          <span class="bad">Saved at {megabytes(clip.result.sizeBytes)}, over the {plain(clip.result.targetMb ?? 0)} MB limit.</span>
          It could not be made smaller.
        {:else}
          <span class="ok"><Icon name="check" size={16} /></span>
          Saved {baseName(clip.result.output)} at <strong>{megabytes(clip.result.sizeBytes)}</strong>, down from {megabytes(clip.result.originalBytes)}.
        {/if}
        <button type="button" class="link" onclick={() => backend.reveal(clip.result!.output).catch((e) => app.notify(String(e)))}>Show in folder</button>
        <button type="button" class="link" onclick={() => backend.openFile(clip.result!.output).catch((e) => app.notify(String(e)))}>Play it</button>
        <button type="button" class="link quiet" onclick={() => { app.reset(clip); app.replan(clip, 0); }}>Compress again</button>
      {:else if clip.status === "failed"}
        <span class="bad">{clip.error}</span>
        <button type="button" class="link" onclick={() => { app.reset(clip); app.replan(clip, 0); }}>Try again</button>
      {:else if clip.planError}
        <span class="bad">{clip.planError}</span>
      {:else if clip.plan}
        {#if clip.status === "canceled"}Canceled.{/if}
        {#if app.options.audioOnly}
          Keeps {span(clip.kept)} of sound, about {megabytes(clip.plan.expectedBytes)}.
        {:else}
          {#if clip.trimmed}Keeps {span(clip.kept)} of {span(clip.duration)}.{/if}
          Comes out as {CODEC_LABEL[app.options.codec]} at {resolution(clip.plan.width, clip.plan.height)}, about {bitrate(clip.plan.videoKbps)}.
          <span class={clip.plan.quality === "rough" ? "bad" : "muted"}>{LOOK[clip.plan.quality]}</span>
        {/if}
        {#if clip.trimmed}
          <button type="button" class="link quiet" onclick={() => app.setTrim(clip, 0, clip.duration)}>Keep all of it</button>
        {/if}
      {:else}
        &nbsp;
      {/if}
      {#if clip.log.length > 0 && clip.status !== "ready"}
        <button type="button" class="link quiet" onclick={() => (showLog = !showLog)}>{showLog ? "Hide log" : "Show log"}</button>
      {/if}
    </p>

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
    padding: 20px 24px 14px;
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

  .ok {
    display: inline-block;
    vertical-align: -3px;
    color: var(--ok);
  }

  .bad {
    color: var(--bad);
  }
</style>
