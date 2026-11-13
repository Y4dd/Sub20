<script lang="ts">
  import { untrack } from "svelte";
  import { app } from "./lib/app.svelte";
  import ClipRail from "./lib/components/ClipRail.svelte";
  import Dock from "./lib/components/Dock.svelte";
  import DropStage from "./lib/components/DropStage.svelte";
  import HistorySheet from "./lib/components/HistorySheet.svelte";
  import KeysSheet from "./lib/components/KeysSheet.svelte";
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
    if (event.key === "?") {
      event.preventDefault();
      app.sheet = "keys";
      return;
    }
    const clip = app.selected;
    if (!clip) return;
    const onSlider = target.getAttribute("role") === "slider";
    // On a trim handle the arrows move the handle; the handle deals with them.
    if (onSlider && event.key.startsWith("Arrow")) return;
    if (event.key === " " && target.tagName === "BUTTON" && !onSlider) return;
    if (vim(event.key, clip)) event.preventDefault();
  }

  // Vim motions, with counts ("5l") and two-key commands ("gg", "dd").
  let count = "";
  let pending = "";
  let pendingTimer = 0;

  function vim(key: string, clip: NonNullable<typeof app.selected>): boolean {
    window.clearTimeout(pendingTimer);
    // A count: 1-9 start it, 0 extends it (alone, 0 is a motion).
    if (/^[1-9]$/.test(key) || (key === "0" && count)) {
      count += key;
      pendingTimer = window.setTimeout(reset, 1000);
      return true;
    }
    const n = Math.max(1, Math.min(999, Number(count) || 1));
    const counted = count !== "";
    const sequence = pending + key;
    reset();
    switch (sequence) {
      case "g":
      case "d":
      case "c":
        // The first key of a pair: wait a second for the second.
        count = counted ? String(n) : "";
        pending = sequence;
        pendingTimer = window.setTimeout(reset, 1000);
        return true;
      case "gg":
        app.select(app.clips[counted ? Math.min(n, app.clips.length) - 1 : 0]);
        return true;
      case "G":
        app.select(app.clips[counted ? Math.min(n, app.clips.length) - 1 : app.clips.length - 1]);
        return true;
      case "j":
      case "ArrowDown":
        app.step(n);
        return true;
      case "k":
      case "ArrowUp":
        app.step(-n);
        return true;
      case "h":
      case "ArrowLeft":
        stage?.nudge(-0.1 * n);
        return true;
      case "l":
      case "ArrowRight":
        stage?.nudge(0.1 * n);
        return true;
      case "H":
      case "b":
        stage?.nudge(-1 * n);
        return true;
      case "L":
      case "w":
        stage?.nudge(1 * n);
        return true;
      case "0":
      case "^":
        stage?.seek(clip.start, true);
        return true;
      case "$":
        stage?.seek(clip.end, true);
        return true;
      case " ":
        stage?.toggle();
        return true;
      case "i":
      case "[":
        stage?.mark("start");
        return true;
      case "o":
      case "]":
        stage?.mark("end");
        return true;
      case "x":
      case "dd":
      case "Delete":
      case "Backspace":
        // Removing a clip that is being compressed is a deliberate click.
        if (!clip.busy) app.remove(clip);
        return true;
      case "s":
        app.toggleLeftOut(clip);
        return true;
      case "y":
      case "yy":
        if (clip.result) void app.copy(clip);
        return true;
      case "r":
      case "cw":
      case "cc":
      case "F2":
        if (!clip.busy) app.renaming = clip;
        return true;
      case "Escape":
        return true;
      default:
        return false;
    }
  }

  function reset() {
    count = "";
    pending = "";
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
  {:else if app.sheet === "keys"}
    <KeysSheet />
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
