<script lang="ts">
  // A panel that slides in from the right, over the bench.
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let { title, onclose, children }: { title: string; onclose: () => void; children: Snippet } = $props();
  let panel: HTMLElement;

  // Focus moves into the sheet, and back to whatever opened it on close.
  $effect(() => {
    const opener = document.activeElement as HTMLElement | null;
    panel.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  });

  // Tab stays inside the sheet while it is open, as in any modal dialog.
  function trap(event: KeyboardEvent) {
    if (event.key !== "Tab") return;
    const stops = [...panel.querySelectorAll<HTMLElement>("button, a[href], input, select, textarea, [tabindex]:not([tabindex='-1'])")].filter(
      (el) => !el.matches(":disabled") && el.getClientRects().length > 0,
    );
    if (!stops.length) return;
    const first = stops[0];
    const last = stops[stops.length - 1];
    const at = document.activeElement;
    if (event.shiftKey && (at === first || at === panel)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && at === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

<div class="scrim" onclick={onclose} aria-hidden="true"></div>
<div
  class="sheet"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  tabindex="-1"
  bind:this={panel}
  onkeydown={(event) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onclose();
    } else trap(event);
  }}
>
  <header>
    <h2>{title}</h2>
    <button type="button" class="close" onclick={onclose} aria-label="Close {title}"><Icon name="close" /></button>
  </header>
  <div class="body">{@render children()}</div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: var(--shade);
    animation: fade 160ms ease-out;
    z-index: 20;
  }

  .sheet {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(440px, 100vw);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-left: 1px solid var(--line);
    animation: slide 220ms var(--ease);
    z-index: 21;
    outline: none;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 16px 14px 24px;
  }

  h2 {
    font-family: var(--display);
    font-stretch: 85%;
    font-size: var(--step-3);
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .close {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    color: var(--muted);
  }

  .close:hover {
    background: var(--raised);
    color: var(--text);
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 4px 24px 32px;
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @keyframes slide {
    from {
      transform: translateX(24px);
      opacity: 0;
    }
  }
</style>
