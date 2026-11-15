<script lang="ts">
  // A filmstrip with its two trim handles: the original's trim, or the part
  // of a finished result to keep. The yellow bracket marks what is kept;
  // while a job runs, the kept part fills in from the left.
  import { clock, span } from "../format";

  let {
    duration,
    start,
    end,
    thumbs,
    playhead,
    busy = false,
    percent: progress = 0,
    settled = false,
    locked = false,
    stops = null,
    onseek,
    ontrim,
  }: {
    duration: number;
    start: number;
    end: number;
    thumbs: string[];
    playhead: number;
    /** Queued or encoding: the progress shows and the handles are locked. */
    busy?: boolean;
    percent?: number;
    /** Finished: the handles are drawn faded. */
    settled?: boolean;
    /** The handles can't be moved. */
    locked?: boolean;
    /** Where the start may land (a result's keyframes); arrow keys step between them. */
    stops?: number[] | null;
    onseek: (time: number, scrubbing: boolean) => void;
    ontrim: (start: number, end: number) => void;
  } = $props();

  let rail: HTMLElement;
  let dragging = $state<"start" | "end" | "playhead" | null>(null);
  let grabOffset = 0;
  // Where the pointer holds the start handle while `start` itself has moved
  // back to a stop (a result's keyframe): the handle follows the pointer
  // smoothly, a mark shows the stop, and on release the handle settles there.
  let loose = $state<number | null>(null);

  const percent = (time: number) => (duration > 0 ? (time / duration) * 100 : 0);
  const startAt = $derived(percent(loose ?? start));
  const keptAt = $derived(percent(start));
  const endAt = $derived(percent(end));

  function timeAt(clientX: number): number {
    const box = rail.getBoundingClientRect();
    const fraction = (clientX - box.left) / Math.max(1, box.width);
    return Math.min(duration, Math.max(0, fraction * duration));
  }

  function move(which: "start" | "end", time: number, held = false) {
    if (which === "start") {
      loose = held && stops?.length ? Math.min(Math.max(0, time), end) : null;
      ontrim(Math.min(time, end), end);
    } else ontrim(start, Math.max(time, start));
    // Show the frame the handle sits on.
    onseek(which === "start" ? start : end, true);
  }

  function grab(which: "start" | "end", event: PointerEvent) {
    if (busy || locked || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragging = which;
    // Keep the edge under the pointer where it was grabbed.
    grabOffset = timeAt(event.clientX) - (which === "start" ? (loose ?? start) : end);
  }

  function drag(event: PointerEvent) {
    if (dragging === "start" || dragging === "end") move(dragging, timeAt(event.clientX) - grabOffset, true);
    else if (dragging === "playhead") onseek(timeAt(event.clientX), true);
  }

  function release(event: PointerEvent) {
    if (!dragging) return;
    const target = event.currentTarget as HTMLElement;
    if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
    if (dragging === "playhead") onseek(timeAt(event.clientX), false);
    dragging = null;
    loose = null;
  }

  function scrub(event: PointerEvent) {
    if (event.button !== 0) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragging = "playhead";
    onseek(timeAt(event.clientX), true);
  }

  function nudge(which: "start" | "end", event: KeyboardEvent) {
    const step = event.shiftKey ? 1 : 0.1;
    // A twentieth of the video, so a long recording is crossed in a few presses.
    const page = Math.max(5, duration / 20);
    const now = which === "start" ? start : end;
    const back = event.key === "ArrowLeft" || event.key === "ArrowDown";
    const ahead = event.key === "ArrowRight" || event.key === "ArrowUp";
    let next: number | null = null;
    if (which === "start" && stops?.length && (back || ahead)) {
      // The start lands on a stop, so a step goes to the next one.
      next = back ? Math.max(0, ...stops.filter((s) => s < now - 0.001)) : (stops.find((s) => s > now + 0.001) ?? now);
    } else if (back) next = now - step;
    else if (ahead) next = now + step;
    else if (event.key === "PageDown") next = now - page;
    else if (event.key === "PageUp") next = now + page;
    else if (event.key === "Home") next = which === "start" ? 0 : start;
    else if (event.key === "End") next = which === "start" ? end : duration;
    if (next === null) return;
    event.preventDefault();
    event.stopPropagation();
    move(which, next);
  }
</script>

<div class="timeline" class:busy class:done={settled} class:locked class:dragging={dragging !== null}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="rail" bind:this={rail} onpointerdown={scrub} onpointermove={drag} onpointerup={release} onpointercancel={release}>
    <div class="strip" class:loading={thumbs.length === 0 || thumbs.includes("")}>
      {#each thumbs as thumb, index (index)}
        {#if thumb}
          <img src={thumb} alt="" draggable="false" />
        {:else}
          <span class="slot"></span>
        {/if}
      {/each}
    </div>

    <div class="cut" style:left="0" style:width="{keptAt}%"></div>
    {#if loose !== null && keptAt < startAt}
      <!-- Where the cut will really start: the stop before the handle. -->
      <div class="snap" style:left="{keptAt}%"></div>
    {/if}
    <div class="cut" style:left="{endAt}%" style:right="0"></div>
    {#if busy}
      <!-- The part not encoded yet slides off to the right, led by a bright
           edge. It moves by transform alone, so progress costs no layout. -->
      <div class="run" style:left="{startAt}%" style:width="{endAt - startAt}%">
        <div class="veil" style:transform="translateX({progress}%)"></div>
      </div>
    {/if}

    <div class="bracket" style:left="{startAt}%" style:width="{endAt - startAt}%">
      <button
        type="button"
        class="handle start"
        role="slider"
        aria-label="Start of the part to keep"
        aria-valuemin={0}
        aria-valuemax={end}
        aria-valuenow={start}
        aria-valuetext={clock(start)}
        aria-orientation="horizontal"
        disabled={busy || locked}
        onpointerdown={(event) => grab("start", event)}
        onpointermove={drag}
        onpointerup={release}
        onpointercancel={release}
        onkeydown={(event) => nudge("start", event)}
      ></button>
      <button
        type="button"
        class="handle end"
        role="slider"
        aria-label="End of the part to keep"
        aria-valuemin={start}
        aria-valuemax={duration}
        aria-valuenow={end}
        aria-valuetext={clock(end)}
        aria-orientation="horizontal"
        disabled={busy || locked}
        onpointerdown={(event) => grab("end", event)}
        onpointermove={drag}
        onpointerup={release}
        onpointercancel={release}
        onkeydown={(event) => nudge("end", event)}
      ></button>
      {#if dragging === "start" || dragging === "end"}
        <output class="readout" class:at-end={dragging === "end"}>
          {clock(dragging === "start" ? start : end)}
          <span>{span(end - start)} kept</span>
        </output>
      {/if}
    </div>

    {#if !busy}
      <div class="playhead" style:left="{percent(playhead)}%"></div>
    {/if}
  </div>
</div>

<style>
  .timeline {
    --handle: 16px;
    --height: 68px;
    /* Room for the handles to stick out at either end. */
    padding: 0 var(--handle);
  }

  .rail {
    position: relative;
    height: var(--height);
    cursor: pointer;
    touch-action: none;
  }

  .strip {
    position: absolute;
    inset: 0;
    display: flex;
    overflow: hidden;
    border-radius: 4px;
    background: var(--well);
  }

  .strip img,
  .strip .slot {
    flex: 1 1 0;
    min-width: 0;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
  }

  .strip.loading {
    background: linear-gradient(100deg, var(--well) 30%, var(--raised) 50%, var(--well) 70%) 0 0 / 300% 100%;
    animation: shimmer 1.4s linear infinite;
  }

  /* The parts that will be thrown away. */
  .cut {
    position: absolute;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--bench) 78%, transparent);
    backdrop-filter: grayscale(1);
    pointer-events: none;
  }

  /* The kept part while it is encoded; clipped at its ends only, so the
     edge's glow still shows above and below. */
  .run {
    position: absolute;
    top: 0;
    bottom: 0;
    clip-path: inset(-16px 0);
    pointer-events: none;
  }

  /* Not encoded yet. */
  .veil {
    position: absolute;
    inset: 0;
    background: color-mix(in srgb, var(--well) 62%, transparent);
    transition: transform 300ms linear;
    will-change: transform;
  }

  /* The front of the encode. */
  .veil::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: -1px;
    width: 2px;
    background: var(--trim);
    box-shadow: 0 0 12px 1px var(--trim);
  }

  .bracket {
    position: absolute;
    top: -3px;
    bottom: -3px;
    border-block: 3px solid var(--trim);
    pointer-events: none;
  }

  .handle {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: var(--handle);
    background: var(--trim);
    box-shadow: inset 0 0 0 1px var(--trim-edge);
    pointer-events: auto;
    cursor: ew-resize;
    touch-action: none;
  }

  .handle.start {
    right: 100%;
    border-radius: 7px 0 0 7px;
  }

  .handle.end {
    left: 100%;
    border-radius: 0 7px 7px 0;
  }

  /* The grip. */
  .handle::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    width: 3px;
    height: 22px;
    margin: -11px 0 0 -1.5px;
    border-radius: 2px;
    background: var(--trim-ink);
    opacity: 0.75;
  }

  /* A wider target than the visible handle. */
  .handle::before {
    content: "";
    position: absolute;
    inset: -6px -8px;
  }

  .handle:hover,
  .handle:focus-visible,
  .dragging .handle {
    filter: brightness(1.08);
  }

  .handle:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }

  .busy .bracket,
  .done .bracket {
    border-color: color-mix(in srgb, var(--trim) 45%, transparent);
  }

  .busy .handle,
  .done .handle {
    background: color-mix(in srgb, var(--trim) 45%, var(--bench));
  }

  .busy .handle,
  .locked .handle {
    cursor: default;
  }

  .snap {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--trim-mark);
    pointer-events: none;
  }

  .playhead {
    position: absolute;
    top: -7px;
    bottom: -7px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: #fff;
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.5);
    pointer-events: none;
  }

  .readout {
    position: absolute;
    bottom: calc(100% + 12px);
    left: 0;
    display: flex;
    gap: 8px;
    padding: 4px 9px;
    border-radius: 6px;
    background: var(--trim);
    color: var(--trim-ink);
    font-weight: 650;
    white-space: nowrap;
  }

  /* Each readout grows inwards, so neither can leave the window. */
  .readout.at-end {
    left: auto;
    right: 0;
  }

  .readout span {
    font-weight: 450;
    opacity: 0.8;
  }

  @keyframes shimmer {
    to {
      background-position: -300% 0;
    }
  }
</style>
