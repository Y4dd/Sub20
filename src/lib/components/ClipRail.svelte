<script lang="ts">
  // The list of videos on the bench.
  import { untrack } from "svelte";
  import { app, type Clip } from "../app.svelte";
  import { megabytes, span } from "../format";
  import Icon from "./Icon.svelte";

  function line(clip: Clip): string {
    switch (clip.status) {
      case "queued":
        return "Waiting";
      case "running":
        return `Compressing ${Math.floor(clip.percent)}%`;
      case "done":
        return clip.result ? megabytes(clip.result.sizeBytes) : "Done";
      case "failed":
        return "Did not work";
      default:
        if (clip.planError) return "Too long for this size";
        if (clip.leftOut) return "Left out";
        return clip.trimmed ? `${span(clip.kept)} of ${span(clip.duration)}` : span(clip.duration);
    }
  }

  // A clip shows its own name until its result is given one.
  function named(clip: Clip): string {
    const renamed = clip.outputName ?? (clip.result ? app.outputStem(clip) : null);
    return renamed && renamed !== app.defaultStem(clip) ? `${renamed}${app.extensionOf(clip)}` : clip.name;
  }

  // What the name field holds while renaming; filled when it opens.
  let draft = $state("");
  $effect(() => {
    const clip = app.renaming;
    if (clip) draft = untrack(() => app.outputStem(clip));
  });
</script>

<aside>
  <ul>
    {#each app.clips as clip (clip.key)}
      {@const selected = app.selected === clip}
      <li class:selected class:left-out={clip.leftOut && clip.compressible} class={clip.status}>
        <button
          type="button"
          class="pick"
          onclick={() => app.select(clip)}
          ondblclick={() => !clip.busy && (app.renaming = clip)}
          aria-current={selected ? "true" : undefined}
        >
          <span class="thumb">
            {#if clip.thumbs[Math.floor(clip.thumbs.length / 2)]}
              <img src={clip.thumbs[Math.floor(clip.thumbs.length / 2)]} alt="" draggable="false" />
            {/if}
            {#if clip.status === "running"}
              <span class="meter" style:width="{clip.percent}%"></span>
            {/if}
          </span>
          <span class="words">
            {#if app.renaming !== clip}
              <span class="name" title="{clip.name}, double-click to rename the result">{named(clip)}</span>
            {/if}
            <span class="line" class:warn={clip.status === "failed" || (clip.planError && !clip.busy && clip.status !== "done")}>
              {#if clip.status === "done"}<Icon name="check" size={14} />{/if}
              {line(clip)}
            </span>
          </span>
        </button>
        {#if app.renaming === clip}
          <!-- Sits over the name; the extension stays as it is. -->
          <form class="rename" onsubmit={(event) => { event.preventDefault(); void app.rename(clip, draft); }}>
            <!-- svelte-ignore a11y_autofocus -->
            <input
              aria-label="Name of the result"
              bind:value={draft}
              autofocus
              onfocus={(event) => event.currentTarget.select()}
              onblur={() => app.renaming === clip && void app.rename(clip, draft)}
              onkeydown={(event) => {
                if (event.key === "Escape") {
                  event.stopPropagation();
                  app.renaming = null;
                }
              }}
            />
            <span class="extension">{app.extensionOf(clip)}</span>
          </form>
        {:else}
          <div class="actions">
            {#if !clip.busy}
              <button type="button" onclick={() => (app.renaming = clip)} aria-label="Rename the result of {clip.name}" title="Rename (F2)">
                <Icon name="pencil" size={14} />
              </button>
            {/if}
            <button type="button" onclick={() => app.remove(clip)} aria-label="Remove {clip.name}" title="Remove (Delete)">
              <Icon name="close" size={14} />
            </button>
          </div>
        {/if}
      </li>
    {/each}
  </ul>
  <button type="button" class="add" onclick={() => app.choose()}>
    <Icon name="plus" size={16} />
    Add videos
  </button>
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 4px 8px 8px 4px;
  }

  ul {
    margin: 0;
    /* Room on the left for the marker of the selected clip. */
    padding: 0 0 0 8px;
    list-style: none;
    overflow-y: auto;
    min-height: 0;
  }

  li {
    position: relative;
    border-radius: 8px;
  }

  li.selected {
    background: var(--surface);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  /* The trim color marks the clip you are working on. */
  li.selected::before {
    content: "";
    position: absolute;
    left: -7px;
    top: 13px;
    bottom: 13px;
    width: 3px;
    border-radius: 2px;
    background: var(--trim-mark);
  }

  .pick {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px;
    border-radius: 8px;
    text-align: left;
  }

  li:not(.selected) .pick:hover {
    background: color-mix(in srgb, var(--surface) 60%, transparent);
  }

  .thumb {
    position: relative;
    flex: none;
    width: 60px;
    height: 40px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--well);
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .meter {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 3px;
    background: var(--trim);
    transition: width 300ms linear;
  }

  .words {
    display: grid;
    min-width: 0;
    padding-right: 8px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: var(--step--1);
  }

  .done .line {
    color: var(--ok);
  }

  /* Left out of Compress: the picture fades, the words stay readable. */
  .left-out .thumb {
    opacity: 0.4;
  }

  .left-out .name {
    color: var(--muted);
  }

  .line.warn {
    color: var(--bad);
  }

  .actions {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    gap: 2px;
    opacity: 0;
  }

  /* The selected clip keeps room for its buttons; other rows show them on
     hover over their own background, so names are not cut short for them. */
  li.selected .words {
    padding-right: 62px;
  }

  li:not(.selected) .actions {
    border-radius: 6px;
    background: color-mix(in srgb, var(--surface) 60%, var(--bench));
  }

  /* Shown on hover, on focus, and always on the clip you are working on. */
  li:hover .actions,
  li.selected .actions,
  .actions:focus-within {
    opacity: 1;
  }

  .actions button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--muted);
  }

  .actions button:hover {
    background: var(--raised);
    color: var(--text);
  }

  /* The name field, laid over the name line. */
  .rename {
    position: absolute;
    top: 6px;
    left: 78px;
    right: 6px;
    display: flex;
    align-items: center;
    height: 28px;
    padding: 0 8px 0 6px;
    border: 1px solid var(--trim-mark);
    border-radius: 6px;
    background: var(--bench);
  }

  .rename input {
    flex: 1;
    min-width: 0;
    padding: 0;
    border: 0;
    outline: none;
    background: none;
    font-weight: 550;
  }

  .extension {
    color: var(--muted);
  }

  .add {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 0 8px;
    padding: 10px 12px;
    border-radius: 8px;
    color: var(--muted);
    text-align: left;
  }

  .add:hover {
    background: var(--surface);
    color: var(--text);
  }
</style>
