<script lang="ts" generics="T extends string | number | boolean | null">
  // One choice out of a few, all visible at once.
  let {
    value,
    options,
    label,
    onchange,
  }: {
    value: T;
    options: { value: T; label: string; disabled?: boolean; hint?: string }[];
    label: string;
    onchange: (value: T) => void;
  } = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={option.value === value}
      disabled={option.disabled}
      title={option.hint}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: flex;
    padding: 3px;
    background: var(--bench);
    border: 1px solid var(--line);
    border-radius: 8px;
  }

  button {
    flex: 1;
    height: 28px;
    padding: 0 8px;
    border-radius: 5px;
    color: var(--muted);
    white-space: nowrap;
    transition: background 120ms, color 120ms;
  }

  button:hover:not(:disabled) {
    color: var(--text);
  }

  button[aria-checked="true"] {
    background: var(--raised);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--line-strong);
  }

  button:disabled {
    color: var(--faint);
    text-decoration: line-through;
  }
</style>
