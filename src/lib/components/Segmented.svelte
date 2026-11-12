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

  // A radio group: one tab stop, and the arrow keys move the choice.
  const chosen = $derived(Math.max(0, options.findIndex((option) => option.value === value && !option.disabled)));

  function arrows(event: KeyboardEvent) {
    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    if (!step) return;
    event.preventDefault();
    event.stopPropagation();
    const group = event.currentTarget as HTMLElement;
    for (let i = 1; i <= options.length; i++) {
      const next = (chosen + step * i + options.length * i) % options.length;
      if (options[next].disabled) continue;
      onchange(options[next].value);
      group.querySelectorAll<HTMLElement>("[role=radio]")[next]?.focus();
      return;
    }
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="segmented" role="radiogroup" aria-label={label} onkeydown={arrows}>
  {#each options as option, i (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={option.value === value}
      tabindex={i === chosen ? 0 : -1}
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
