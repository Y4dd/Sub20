<script lang="ts">
  import { app, backend } from "../app.svelte";
  import { baseName, megabytes, when, CODEC_LABEL, engineOf } from "../format";
  import type { HistoryEntry } from "../types";
  import Icon from "./Icon.svelte";
  import Sheet from "./Sheet.svelte";

  let entries = $state.raw<HistoryEntry[] | null>(null);

  async function load() {
    entries = await backend.historyList();
  }

  $effect(() => {
    void load();
  });

  const codecOf = (encoder: string) =>
    encoder.includes("265") || encoder.includes("hevc") ? "hevc" : encoder.includes("av1") ? "av1" : encoder.includes("264") ? "h264" : null;

  function detail(entry: HistoryEntry): string {
    const r = entry.result;
    const codec = codecOf(r.encoder);
    const parts = [when(entry.finishedAt)];
    parts.push(codec ? `${CODEC_LABEL[codec]} on the ${engineOf(r.encoder)}` : "sound only");
    if (entry.trimmed) parts.push("trimmed");
    if (entry.origin === "watch") parts.push("from the watched folder");
    return parts.join(", ");
  }
</script>

<Sheet title="History" onclose={() => (app.sheet = null)}>
  {#if entries === null}
    <p class="muted">Loading…</p>
  {:else if entries.length === 0}
    <p class="muted empty">
      {app.settings?.historyEnabled === false
        ? "History is switched off. Turn it on in Settings to keep a record of what you compress."
        : "Nothing yet. Videos you compress are listed here, with what they shrank to."}
    </p>
  {:else}
    <ul>
      {#each entries as entry (entry.id)}
        <li>
          <div class="words">
            <p class="name" title={entry.result.output}>{baseName(entry.result.output)}</p>
            <p class="sizes">
              {megabytes(entry.result.originalBytes)} to <strong>{megabytes(entry.result.sizeBytes)}</strong>
            </p>
            <p class="muted detail">{detail(entry)}</p>
          </div>
          <div class="actions">
            <button type="button" aria-label="Show {baseName(entry.result.output)} in its folder" title="Show in folder"
              onclick={() => backend.reveal(entry.result.output).catch((e) => app.notify(String(e)))}>
              <Icon name="folder" />
            </button>
            <button type="button" aria-label="Remove from history" title="Remove from history"
              onclick={async () => { await backend.historyRemove(entry.id); await load(); }}>
              <Icon name="close" size={16} />
            </button>
          </div>
        </li>
      {/each}
    </ul>
    <p class="foot">
      <button type="button" class="link quiet" onclick={async () => { await backend.historyClear(); await load(); }}>Clear history</button>
      <span class="muted">The videos themselves stay where they are.</span>
    </p>
  {/if}
</Sheet>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 13px 0;
    border-bottom: 1px solid var(--line);
  }

  .words {
    flex: 1;
    min-width: 0;
  }

  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail {
    font-size: var(--step--1);
  }

  .actions {
    display: flex;
  }

  .actions button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    color: var(--muted);
  }

  .actions button:hover {
    background: var(--raised);
    color: var(--text);
  }

  .empty {
    max-width: 32em;
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin-top: 18px;
    font-size: var(--step--1);
  }
</style>
