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
  import TitleBar from "./lib/components/TitleBar.svelte";

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
    // Ctrl+C on a finished clip copies the file, unless text is selected.
    if ((event.metaKey || event.ctrlKey) && event.key === "c" && !typing && !app.sheet) {
      const done = app.selected?.result;
      if (done && !window.getSelection()?.toString()) {
        event.preventDefault();
        void app.copy(app.selected!);
      }
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
    <TitleBar />

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

  /* The title bar runs across the top; under it, the list beside the stage. */
  main:not(.empty) > :global(aside) {
    grid-column: 1;
    grid-row: 2;
  }

  main:not(.empty) > :global(.stage) {
    grid-column: 2;
    grid-row: 2;
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
