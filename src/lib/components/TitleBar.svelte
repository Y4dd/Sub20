<script lang="ts">
  // The top of the window: the wordmark, History and Settings, and on
  // Windows the window buttons, since the app draws its own frame there.
  // The bar itself moves the window; double-clicking it maximizes.
  import { app, backend } from "../app.svelte";
  import Icon from "./Icon.svelte";

  let maximized = $state(false);

  $effect(() => {
    if (backend.frame !== "custom") return;
    const stop = backend.onMaximized((value) => (maximized = value));
    return () => void stop.then((unlisten) => unlisten());
  });
</script>

<header class="titlebar {backend.frame}" data-tauri-drag-region>
  <span class="mark" aria-label="Sub20" data-tauri-drag-region>Sub<i>20</i></span>
  <span class="room" data-tauri-drag-region></span>

  <div class="tools">
    <button type="button" onclick={() => (app.sheet = "history")} aria-label="History" title="History"><Icon name="history" /></button>
    <button type="button" onclick={() => (app.sheet = "settings")} aria-label="Settings" title="Settings"><Icon name="gear" /></button>
  </div>

  {#if backend.frame === "custom"}
    <div class="window" role="group" aria-label="Window">
      <button type="button" onclick={() => backend.minimize()} aria-label="Minimize" title="Minimize">
        <Icon name="minimize" size={16} />
      </button>
      <button type="button" onclick={() => backend.toggleMaximize()} aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"}>
        <Icon name={maximized ? "restore" : "maximize"} size={16} />
      </button>
      <button type="button" class="close" onclick={() => backend.closeWindow()} aria-label="Close" title="Close">
        <Icon name="close" size={16} />
      </button>
    </div>
  {/if}
</header>

<style>
  .titlebar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    height: 44px;
    padding-left: 20px;
    user-select: none;
    -webkit-user-select: none;
  }

  /* macOS draws its window buttons over the left of the bar. */
  .titlebar.overlay {
    padding-left: 80px;
  }

  .titlebar:not(.custom) {
    padding-right: 12px;
  }

  .mark {
    font-family: var(--display);
    font-size: 1.375rem;
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

  .room {
    flex: 1;
    align-self: stretch;
  }

  .tools {
    display: flex;
    gap: 2px;
    margin-right: 8px;
  }

  .tools button {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 8px;
    color: var(--muted);
  }

  .tools button:hover {
    background: var(--surface);
    color: var(--text);
  }

  /* Window buttons, sized and placed the way Windows places its own. */
  .window {
    display: flex;
    align-self: stretch;
  }

  .window button {
    display: grid;
    place-items: center;
    width: 46px;
    color: var(--muted);
    border-radius: 0;
  }

  .window button:hover {
    background: var(--surface);
    color: var(--text);
  }

  .window .close:hover {
    background: #c42b1c;
    color: #fff;
  }

  .window button:focus-visible {
    outline-offset: -2px;
  }
</style>
