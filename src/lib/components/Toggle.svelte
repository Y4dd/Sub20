<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    checked,
    onchange,
    disabled = false,
    children,
  }: { checked: boolean; onchange: (checked: boolean) => void; disabled?: boolean; children: Snippet } = $props();
</script>

<label class:disabled>
  <span class="text">{@render children()}</span>
  <input type="checkbox" role="switch" {checked} {disabled} onchange={(event) => onchange(event.currentTarget.checked)} />
  <span class="track" aria-hidden="true"></span>
</label>

<style>
  label {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 34px;
    cursor: pointer;
  }

  /* Muted, not faint: a switched-off row often says why (an encoder that
     doesn't work here), and that has to stay readable. */
  label.disabled {
    cursor: default;
    color: var(--muted);
  }

  .text {
    flex: 1;
  }

  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .track {
    flex: none;
    width: 34px;
    height: 20px;
    border-radius: 10px;
    background: var(--line-strong);
    position: relative;
    transition: background 140ms;
  }

  .track::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--surface);
    transition: transform 140ms var(--ease);
  }

  input:checked + .track {
    background: var(--trim);
    box-shadow: inset 0 0 0 1px var(--trim-edge);
  }

  input:checked + .track::after {
    transform: translateX(14px);
    background: var(--trim-ink);
  }

  /* The switch carries "can't be changed", so the words don't have to. */
  label.disabled .track {
    opacity: 0.45;
  }

  input:focus-visible + .track {
    outline: 2px solid var(--trim);
    outline-offset: 2px;
  }
</style>
