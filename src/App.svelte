<script lang="ts">
  import { untrack } from "svelte";
  import { app } from "./lib/app.svelte";
  import ClipRail from "./lib/components/ClipRail.svelte";
  import Dock from "./lib/components/Dock.svelte";
  import DropStage from "./lib/components/DropStage.svelte";
  import HistorySheet from "./lib/components/HistorySheet.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import OptionsSheet from "./lib/components/OptionsSheet.svelte";
  import SettingsSheet from "./lib/components/SettingsSheet.svelte";
  import Stage from "./lib/components/Stage.svelte";

  let stage = $state<ReturnType<typeof Stage> | null>(null);

  void app.init();

  // Any change to the options changes what every clip will come out as.
  $effect(() => {
    JSON.stringify(app.options);
    untrack(() => app.replanAll());
  });

  function onkeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    const typing = target.matches("input, select, textarea") || target.isContentEditable;
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      void app.compress();
      return;
    }
    if (event.key === "Escape" && app.sheet) {
      app.sheet = null;
      return;
    }
    if (typing || app.sheet || event.metaKey || event.ctrlKey || event.altKey) return;
    const clip = app.selected;
    if (!clip) return;
    const onSlider = target.getAttribute("role") === "slider";
    switch (event.key) {
      case " ":
        if (target.tagName === "BUTTON" && !onSlider) return;
        stage?.toggle();
        break;
      case "ArrowUp":
        if (onSlider) return;
        app.step(-1);
        break;
      case "ArrowDown":
        if (onSlider) return;
        app.step(1);
        break;
      case "ArrowLeft":
        if (onSlider) return;
        stage?.nudge(event.shiftKey ? -1 : -0.1);
        break;
      case "ArrowRight":
        if (onSlider) return;
        stage?.nudge(event.shiftKey ? 1 : 0.1);
        break;
      case "i":
      case "[":
        stage?.mark("start");
        break;
      case "o":
      case "]":
        stage?.mark("end");
        break;
      case "Delete":
      case "Backspace":
        // Removing a clip that is being compressed is a deliberate click.
        if (clip.busy) return;
        app.remove(clip);
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

<svelte:window {onkeydown} />

{#if app.ready}
  <main class:empty={app.clips.length === 0} class:over={app.dragging}>
    <div class="top">
      <span class="mark" aria-label="Sub20">Sub<i>20</i></span>
      <div class="tools">
        <button type="button" onclick={() => (app.sheet = "history")} aria-label="History" title="History"><Icon name="history" /></button>
        <button type="button" onclick={() => (app.sheet = "settings")} aria-label="Settings" title="Settings"><Icon name="sliders" /></button>
      </div>
    </div>

    {#if app.clips.length === 0}
      <DropStage />
    {:else}
      <ClipRail />
      {#if app.selected}
        <Stage bind:this={stage} clip={app.selected} />
      {/if}
      <Dock />
    {/if}
  </main>

  {#if app.sheet === "options"}
    <OptionsSheet />
  {:else if app.sheet === "settings"}
    <SettingsSheet />
  {:else if app.sheet === "history"}
    <HistorySheet />
  {/if}

  {#if app.notices.length}
    <div class="notices" role="status">
      {#each app.notices as notice (notice.id)}
        <p>
          {notice.text}
          <button type="button" onclick={() => app.dismiss(notice.id)} aria-label="Dismiss"><Icon name="close" size={14} /></button>
        </p>
      {/each}
    </div>
  {/if}
{/if}

<style>
  main {
    height: 100%;
    display: grid;
    grid-template-columns: 264px minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr) auto;
  }

  main.empty {
    grid-template-columns: 1fr;
    grid-template-rows: auto minmax(0, 1fr);
  }

  /* Files held over a full bench: the whole window says "yes". */
  main.over:not(.empty) {
    outline: 3px solid var(--trim);
    outline-offset: -3px;
  }

  .top {
    grid-column: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 12px 6px 20px;
  }

  main.empty .top {
    padding-right: 16px;
  }

  .mark {
    font-family: var(--display);
    font-size: 1.625rem;
    font-weight: 800;
    font-stretch: 76%;
    letter-spacing: -0.03em;
    line-height: 1;
  }

  .mark i {
    font-style: normal;
    font-weight: 500;
    color: var(--muted);
  }

  .tools {
    display: flex;
    gap: 2px;
  }

  .tools button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 7px;
    color: var(--muted);
  }

  .tools button:hover {
    background: var(--surface);
    color: var(--text);
  }

  /* With clips, the rail sits under the top bar and the stage spans both rows. */
  main:not(.empty) > :global(aside) {
    grid-column: 1;
    grid-row: 2;
  }

  main:not(.empty) > :global(.stage) {
    grid-column: 2;
    grid-row: 1 / 3;
  }

  .notices {
    position: fixed;
    left: 50%;
    bottom: 96px;
    transform: translateX(-50%);
    display: grid;
    gap: 8px;
    z-index: 30;
  }

  .notices p {
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 60ch;
    padding: 10px 10px 10px 16px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: var(--raised);
    box-shadow: 0 12px 32px -10px rgb(0 0 0 / 0.5);
    animation: rise 180ms var(--ease);
  }

  .notices button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 5px;
    color: var(--muted);
  }

  .notices button:hover {
    color: var(--text);
  }

  @keyframes rise {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
