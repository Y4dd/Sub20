<script lang="ts">
  // The bottom bar: one size, one sentence of options, one action.
  import { app } from "../app.svelte";
  import { plain, plural, CODEC_LABEL } from "../format";
  import Icon from "./Icon.svelte";

  let picking = $state(false);
  let custom = $state("");
  let picker = $state<HTMLElement | null>(null);

  const shortcut = navigator.platform.toLowerCase().includes("mac") ? "⌘ Return" : "Ctrl+Enter";
  const sizes = $derived(app.settings?.sizes ?? []);
  const pending = $derived(app.pending.length);
  const busy = $derived(app.busy.length);
  const finished = $derived(app.finished.length);

  function setSize(mb: number) {
    if (!Number.isFinite(mb) || mb <= 0) return;
    app.options.targetMb = Math.min(51200, mb);
    app.options.videoKbps = null;
    picking = false;
    custom = "";
  }

  // What the options amount to, as a sentence.
  const summary = $derived.by(() => {
    const o = app.options;
    if (o.audioOnly) return `Sound only, ${o.audio === "opus" ? "Opus" : "AAC"} at ${o.audioKbps} kbps`;
    const parts = [CODEC_LABEL[o.codec]];
    if (o.autoResolution) parts.push("resolution picked for you");
    else if (o.maxShortSide) parts.push(`up to ${o.maxShortSide}p`);
    else if (o.maxWidth || o.maxHeight) parts.push("custom size limit");
    else parts.push("original resolution");
    if (o.maxFps) parts.push(`${plain(o.maxFps)} fps at most`);
    if (o.audio === "none") parts.push("no sound");
    if (o.effort === "max") parts.push("best quality");
    return parts.join(", ");
  });

  $effect(() => {
    if (!picking) return;
    const close = (event: PointerEvent) => {
      if (picker && !picker.contains(event.target as Node)) picking = false;
    };
    window.addEventListener("pointerdown", close, true);
    return () => window.removeEventListener("pointerdown", close, true);
  });
</script>

<footer>
  <div class="size" bind:this={picker}>
    <span class="lead">{app.options.videoKbps ? "Encode at" : app.clips.length === 1 ? "Fit it in" : "Fit each video in"}</span>
    <button
      type="button"
      class="figure"
      aria-haspopup="dialog"
      aria-expanded={picking}
      onclick={() => (picking = !picking)}
      title="Change the size limit"
    >
      {#if app.options.videoKbps}
        <b>{plain(app.options.videoKbps, 0)}</b><span>kbps</span>
      {:else}
        <b>{plain(app.options.targetMb)}</b><span>MB</span>
      {/if}
      <Icon name="down" size={16} />
    </button>

    {#if picking}
      <div class="picker" role="dialog" aria-label="Size limit">
        <div class="choices">
          {#each sizes as size (size)}
            <button type="button" class:current={!app.options.videoKbps && size === app.options.targetMb} onclick={() => setSize(size)}>
              {plain(size)}<span>MB</span>
            </button>
          {/each}
        </div>
        <form
          onsubmit={(event) => {
            event.preventDefault();
            setSize(Number(custom));
          }}
        >
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="field"
            inputmode="decimal"
            placeholder="Another size"
            aria-label="Another size in MB"
            bind:value={custom}
            autofocus
            onkeydown={(event) => event.key === "Escape" && (picking = false)}
          />
          <span class="unit">MB</span>
        </form>
        <p class="muted">Every result is checked. If one comes out over the limit, it is compressed again until it fits.</p>
      </div>
    {/if}
  </div>

  <button type="button" class="options" onclick={() => (app.sheet = "options")} title="Change format, resolution, sound and more">
    <Icon name="sliders" />
    <span>{summary}</span>
  </button>

  <div class="actions">
    {#if busy > 0}
      <span class="muted">{finished} of {finished + busy + pending} done</span>
      <button type="button" class="quiet" onclick={() => app.cancelAll()}>Stop</button>
    {:else if pending > 0}
      <button type="button" class="primary" onclick={() => app.compress()} title="Compress ({shortcut})">
        Compress {pending === 1 && app.clips.length === 1 ? "" : plural(pending, "video")}
      </button>
    {:else if finished > 0}
      <button type="button" class="quiet" onclick={() => app.clearFinished()}>Clear finished</button>
    {/if}
  </div>
</footer>

<style>
  footer {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 28px;
    padding: 12px 24px 14px;
    border-top: 1px solid var(--line);
    background: var(--surface);
  }

  .size {
    position: relative;
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  .lead {
    color: var(--muted);
  }

  /* The one loud piece of type: the number everything else answers to. */
  .figure {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 0 8px 0 10px;
    margin: -4px 0;
    border-radius: 8px;
    font-family: var(--display);
    line-height: 1.1;
  }

  .figure:hover,
  .figure[aria-expanded="true"] {
    background: var(--raised);
  }

  .figure b {
    font-size: var(--step-4);
    font-weight: 750;
    font-stretch: 78%;
    letter-spacing: -0.02em;
  }

  .figure span {
    font-size: var(--step-2);
    font-weight: 600;
    font-stretch: 85%;
    color: var(--muted);
  }

  .figure :global(svg) {
    align-self: center;
    margin-left: 2px;
    color: var(--muted);
  }

  .picker {
    position: absolute;
    left: 0;
    bottom: calc(100% + 14px);
    width: 320px;
    padding: 14px;
    display: grid;
    gap: 12px;
    border: 1px solid var(--line-strong);
    border-radius: 12px;
    background: var(--surface);
    box-shadow: 0 18px 50px -12px rgb(0 0 0 / 0.55);
    animation: rise 160ms var(--ease);
    z-index: 10;
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .choices button {
    display: flex;
    align-items: baseline;
    gap: 3px;
    height: 34px;
    padding: 0 12px;
    border: 1px solid var(--line);
    border-radius: 17px;
    font-weight: 600;
  }

  .choices button span {
    color: var(--muted);
    font-size: var(--step--1);
    font-weight: 450;
  }

  .choices button:hover {
    border-color: var(--line-strong);
    background: var(--raised);
  }

  .choices button.current {
    border-color: var(--trim);
    background: var(--trim);
    color: var(--trim-ink);
  }

  .choices button.current span {
    color: inherit;
    opacity: 0.7;
  }

  form {
    position: relative;
  }

  .unit {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    pointer-events: none;
  }

  .picker p {
    font-size: var(--step--1);
  }

  .options {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    justify-self: start;
    max-width: 100%;
    height: 36px;
    padding: 0 12px 0 10px;
    border-radius: 8px;
    color: var(--muted);
    text-align: left;
  }

  .options span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .options:hover {
    background: var(--raised);
    color: var(--text);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 44px;
  }

  .primary,
  .quiet {
    height: 44px;
    padding: 0 22px;
    border-radius: 10px;
    font-size: var(--step-1);
    font-weight: 650;
    white-space: nowrap;
  }

  .primary {
    background: var(--trim);
    color: var(--trim-ink);
    transition: transform 120ms var(--ease), filter 120ms;
  }

  .primary:hover {
    filter: brightness(1.07);
  }

  .primary:active {
    transform: scale(0.98);
  }

  .quiet {
    border: 1px solid var(--line-strong);
  }

  .quiet:hover {
    background: var(--raised);
  }

  @keyframes rise {
    from {
      transform: translateY(6px);
      opacity: 0;
    }
  }
</style>
