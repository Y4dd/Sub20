<script lang="ts">
  // The filmstrip with its two trim handles. The yellow bracket marks what
  // is kept; while a job runs, the kept part fills in from the left.
  import { app, type Clip } from "../app.svelte";
  import { clock, span } from "../format";

  let {
    clip,
    playhead,
    onseek,
  }: { clip: Clip; playhead: number; onseek: (time: number, scrubbing: boolean) => void } = $props();

  let rail: HTMLElement;
  let dragging = $state<"start" | "end" | "playhead" | null>(null);
  let grabOffset = 0;

  const percent = (time: number) => (clip.duration > 0 ? (time / clip.duration) * 100 : 0);
  const startAt = $derived(percent(clip.start));
  const endAt = $derived(percent(clip.end));
  const progressAt = $derived(startAt + ((endAt - startAt) * clip.percent) / 100);

  function timeAt(clientX: number): number {
    const box = rail.getBoundingClientRect();
    const fraction = (clientX - box.left) / Math.max(1, box.width);
    return Math.min(clip.duration, Math.max(0, fraction * clip.duration));
  }

  function move(which: "start" | "end", time: number) {
    if (which === "start") app.setTrim(clip, Math.min(time, clip.end), clip.end);
    else app.setTrim(clip, clip.start, Math.max(time, clip.start));
    // Show the frame the handle sits on.
    onseek(which === "start" ? clip.start : clip.end, true);
  }

  function grab(which: "start" | "end", event: PointerEvent) {
    if (clip.busy || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragging = which;
    // Keep the edge under the pointer where it was grabbed.
    grabOffset = timeAt(event.clientX) - (which === "start" ? clip.start : clip.end);
  }

  function drag(event: PointerEvent) {
    if (dragging === "start" || dragging === "end") move(dragging, timeAt(event.clientX) - grabOffset);
    else if (dragging === "playhead") onseek(timeAt(event.clientX), true);
  }

  function release(event: PointerEvent) {
    if (!dragging) return;
    const target = event.currentTarget as HTMLElement;
    if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
    if (dragging === "playhead") onseek(timeAt(event.clientX), false);
    dragging = null;
  }

  function scrub(event: PointerEvent) {
    if (event.button !== 0) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragging = "playhead";
    onseek(timeAt(event.clientX), true);
  }

  function nudge(which: "start" | "end", event: KeyboardEvent) {
    const step = event.shiftKey ? 1 : 0.1;
    const now = which === "start" ? clip.start : clip.end;
    let next: number | null = null;
    if (event.key === "ArrowLeft" || event.key === "ArrowDown") next = now - step;
    else if (event.key === "ArrowRight" || event.key === "ArrowUp") next = now + step;
    else if (event.key === "Home") next = which === "start" ? 0 : clip.start;
    else if (event.key === "End") next = which === "start" ? clip.end : clip.duration;
    if (next === null) return;
    event.preventDefault();
    event.stopPropagation();
    move(which, next);
  }
</script>

<div class="timeline" class:busy={clip.busy} class:done={clip.status === "done"} class:dragging={dragging !== null}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="rail" bind:this={rail} onpointerdown={scrub} onpointermove={drag} onpointerup={release} onpointercancel={release}>
    <div class="strip" class:loading={clip.thumbs.length === 0 || clip.thumbs.includes("")}>
      {#each clip.thumbs as thumb, index (index)}
        {#if thumb}
          <img src={thumb} alt="" draggable="false" />
        {:else}
          <span class="slot"></span>
        {/if}
      {/each}
    </div>

    <div class="cut" style:left="0" style:width="{startAt}%"></div>
    <div class="cut" style:left="{endAt}%" style:right="0"></div>
    {#if clip.busy}
      <div class="veil" style:left="{progressAt}%" style:width="{Math.max(0, endAt - progressAt)}%"></div>
      <div class="front" style:left="{progressAt}%"></div>
    {/if}

    <div class="bracket" style:left="{startAt}%" style:width="{endAt - startAt}%">
      <button
        type="button"
        class="handle start"
        role="slider"
        aria-label="Start of the part to keep"
        aria-valuemin={0}
        aria-valuemax={clip.end}
        aria-valuenow={clip.start}
        aria-valuetext={clock(clip.start)}
        aria-orientation="horizontal"
        disabled={clip.busy || !!clip.result?.replacedOriginal}
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
        aria-valuemin={clip.start}
        aria-valuemax={clip.duration}
        aria-valuenow={clip.end}
        aria-valuetext={clock(clip.end)}
        aria-orientation="horizontal"
        disabled={clip.busy || !!clip.result?.replacedOriginal}
        onpointerdown={(event) => grab("end", event)}
        onpointermove={drag}
        onpointerup={release}
        onpointercancel={release}
        onkeydown={(event) => nudge("end", event)}
      ></button>
      {#if dragging === "start" || dragging === "end"}
        <output class="readout" class:at-end={dragging === "end"}>
          {clock(dragging === "start" ? clip.start : clip.end)}
          <span>{span(clip.kept)} kept</span>
        </output>
      {/if}
    </div>

    {#if !clip.busy}
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

  /* Not encoded yet. */
  .veil {
    position: absolute;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--well) 62%, transparent);
    pointer-events: none;
  }

  .front {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: var(--trim);
    box-shadow: 0 0 12px 1px var(--trim);
    pointer-events: none;
    transition: left 300ms linear;
  }

  .veil {
    transition: left 300ms linear, width 300ms linear;
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

  .busy .handle {
    cursor: default;
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
