<script lang="ts">
  // The list of videos on the bench.
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
        return clip.trimmed ? `${span(clip.kept)} of ${span(clip.duration)}` : span(clip.duration);
    }
  }
</script>

<aside>
  <ul>
    {#each app.clips as clip (clip.key)}
      {@const selected = app.selected === clip}
      <li class:selected class={clip.status}>
        <button type="button" class="pick" onclick={() => app.select(clip)} aria-current={selected ? "true" : undefined}>
          <span class="thumb">
            {#if clip.thumbs.length}
              <img src={clip.thumbs[Math.floor(clip.thumbs.length / 2)]} alt="" draggable="false" />
            {/if}
            {#if clip.status === "running"}
              <span class="meter" style:width="{clip.percent}%"></span>
            {/if}
          </span>
          <span class="words">
            <span class="name" title={clip.name}>{clip.name}</span>
            <span class="line" class:warn={clip.status === "failed" || (clip.planError && !clip.busy && clip.status !== "done")}>
              {#if clip.status === "done"}<Icon name="check" size={14} />{/if}
              {line(clip)}
            </span>
          </span>
        </button>
        <button type="button" class="remove" onclick={() => app.remove(clip)} aria-label="Remove {clip.name}" title="Remove (Delete)">
          <Icon name="close" size={14} />
        </button>
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
    padding-right: 30px;
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

  .line.warn {
    color: var(--bad);
  }

  .remove {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--muted);
    opacity: 0;
  }

  /* Shown on hover, on focus, and always on the clip you are working on. */
  li:hover .remove,
  li.selected .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:hover {
    background: var(--raised);
    color: var(--text);
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
