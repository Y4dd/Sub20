<script lang="ts">
  // The empty bench. The drop target is drawn as the trim bracket itself:
  // the first thing you see is the thing you will be dragging.
  import { app, backend } from "../app.svelte";

  const mac = navigator.platform.toLowerCase().includes("mac");
  const windows = navigator.platform.toLowerCase().includes("win");
</script>

<section class="empty" class:over={app.dragging}>
  {#if !app.ffmpeg.found}
    <div class="notice" role="alert">
      <h1>FFmpeg is needed first</h1>
      <p>
        Sub20 uses FFmpeg to read and write video, and could not find it on this computer.
        {#if mac}Install it with <code>brew install ffmpeg</code>.{:else if windows}Install it with <code>winget install ffmpeg</code>.{:else}Install it with your package manager, for example <code>sudo apt install ffmpeg</code>.{/if}
      </p>
      <p class="choices">
        <button type="button" class="link" onclick={async () => (app.ffmpeg = await backend.ffmpegStatus())}>Look again</button>
        <button
          type="button"
          class="link"
          onclick={async () => {
            const folder = await backend.pickFolder();
            if (folder) await app.saveSettings((s) => (s.ffmpegFolder = folder));
          }}
        >
          Choose the folder it is in
        </button>
      </p>
    </div>
  {:else}
    <button type="button" class="bracket" onclick={() => app.choose()} disabled={app.adding}>
      <span class="grip left" aria-hidden="true"></span>
      <span class="copy">
        {#if app.adding}
          <strong>Reading your videos…</strong>
        {:else if app.dragging}
          <strong>Let go to add them</strong>
        {:else}
          <strong>Drop videos here</strong>
          <span>or click to choose them. Trim each one, pick a size, and they come out under it.</span>
        {/if}
      </span>
      <span class="grip right" aria-hidden="true"></span>
    </button>
  {/if}
</section>

<style>
  .empty {
    grid-column: 1 / -1;
    display: grid;
    place-items: center;
    padding: 24px 48px 56px;
    min-height: 0;
  }

  .bracket {
    --grip: 22px;
    --bar: 4px;
    position: relative;
    width: min(720px, 100%);
    height: min(300px, 62vh);
    display: grid;
    place-items: center;
    padding: 0 calc(var(--grip) + 40px);
    border-radius: 11px;
    transition: width 320ms var(--ease);
    animation: open 700ms var(--ease) both;
  }

  /* Top and bottom bars, running between the two grips. */
  .bracket::before {
    content: "";
    position: absolute;
    inset: 0 calc(var(--grip) - 1px);
    border-block: var(--bar) solid var(--trim);
    background: color-mix(in srgb, var(--trim) 5%, transparent);
    transition: background 200ms;
  }

  .bracket:hover:not(:disabled)::before {
    background: color-mix(in srgb, var(--trim) 9%, transparent);
  }

  .over .bracket {
    width: min(820px, 100%);
  }

  .over .bracket::before {
    background: color-mix(in srgb, var(--trim) 14%, transparent);
  }

  .grip {
    position: absolute;
    top: 0;
    bottom: 0;
    width: var(--grip);
    background: var(--trim);
  }

  .grip.left {
    left: 0;
    border-radius: 10px 0 0 10px;
  }

  .grip.right {
    right: 0;
    border-radius: 0 10px 10px 0;
  }

  .grip::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    width: 4px;
    height: 44px;
    margin: -22px 0 0 -2px;
    border-radius: 2px;
    background: var(--trim-ink);
    opacity: 0.7;
  }

  .copy {
    position: relative;
    display: grid;
    gap: 10px;
    max-width: 30em;
    text-align: center;
  }

  .copy strong {
    font-family: var(--display);
    font-stretch: 80%;
    font-size: clamp(2rem, 5vw, 3.25rem);
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1;
  }

  .copy span {
    color: var(--muted);
    font-size: var(--step-1);
    text-wrap: balance;
  }

  /* The bracket opens once, like handles being pulled apart. */
  @keyframes open {
    from {
      width: 180px;
      opacity: 0;
    }
  }

  .notice {
    max-width: 34em;
    display: grid;
    gap: 12px;
  }

  .notice h1 {
    font-family: var(--display);
    font-stretch: 85%;
    font-size: var(--step-4);
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .notice p {
    font-size: var(--step-1);
    color: var(--muted);
    user-select: text;
    -webkit-user-select: text;
  }

  .notice code {
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--raised);
    color: var(--text);
    font: 0.92em ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  }

  .choices {
    display: flex;
    gap: 20px;
  }
</style>
