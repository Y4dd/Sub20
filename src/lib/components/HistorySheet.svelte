<script lang="ts">
  import { app, backend } from "../app.svelte";
  import { baseName, megabytes, when, plural, CODEC_LABEL, engineOf } from "../format";
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

  // Opening History is seeing what the watch did.
  $effect(() => {
    app.unseen = 0;
  });

  // A finished watched clip is new history: read it again.
  const finished = $derived(app.watched.filter((job) => job.status === "done").length);
  $effect(() => {
    if (finished) void load();
  });

  function watchLine(): string {
    const w = app.watch;
    if (!w) return "Starting…";
    if (w.problem) return w.problem;
    if (w.paused) return "Paused until you resume it or Sub20 starts again.";
    if (!w.running) return "Not running.";
    const parts = [];
    if (w.inProgress) parts.push(`${w.inProgress} compressing or waiting`);
    if (w.done) parts.push(`${plural(w.done, "clip")} done`);
    if (w.failed) parts.push(`${w.failed} failed`);
    if (w.benched) parts.push(`${plural(w.benched, "clip")} put on the bench`);
    return parts.length ? `${parts.join(", ")}.` : "Waiting for new clips.";
  }

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
  {#if app.settings?.watch.enabled}
    <section class="watch" aria-label="Watched folder">
      <h3>Watched folder</h3>
      <p class="where" title={app.watch?.folder ?? app.settings.watch.folder ?? ""}>
        {app.watch?.folder ?? app.settings.watch.folder ?? "No folder chosen"}
      </p>
      <p class="muted" class:bad={!!app.watch?.problem}>
        {watchLine()}
        {#if app.watch?.paused}
          <button type="button" class="link" onclick={() => app.pauseWatch(false)}>Resume</button>
        {:else if app.watch?.running}
          <button type="button" class="link" onclick={() => app.pauseWatch(true)}>Pause</button>
        {/if}
      </p>
      {#if app.watched.length}
        <ul class="watched">
          {#each app.watched as job (job.id)}
            <li>
              <div class="words">
                <p class="name" title={job.result?.output ?? job.input}>{job.result ? baseName(job.result.output) : job.name}</p>
                <p class="muted detail" class:bad={job.status === "failed"}>
                  {#if job.status === "queued"}
                    Waiting its turn
                  {:else if job.status === "running"}
                    Compressing, {Math.floor(job.percent)}%
                  {:else if job.status === "done" && job.result}
                    {megabytes(job.result.originalBytes)} to <strong>{megabytes(job.result.sizeBytes)}</strong>
                  {:else if job.status === "failed"}
                    {job.error ?? "Couldn't be compressed."}
                  {:else}
                    Stopped
                  {/if}
                </p>
              </div>
              {#if job.status === "done"}
                <div class="links">
                  <button type="button" class="link" onclick={() => app.openWatched(job)}>Open</button>
                  <button type="button" class="link" onclick={() => app.copyWatched(job)}>{job.copied ? "Copied" : "Copy"}</button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>
    <h3>Everything compressed</h3>
  {/if}
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
            {#if backend.can.files}
              <button type="button" aria-label="Show {baseName(entry.result.output)} in its folder" title="Show in folder"
                onclick={() => backend.reveal(entry.result.output).catch((e) => app.notify(String(e)))}>
                <Icon name="folder" />
              </button>
            {/if}
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

  h3 {
    margin: 0 0 6px;
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--muted);
  }

  .watch {
    margin-bottom: 26px;
  }

  .where {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .watch > .muted {
    font-size: var(--step--1);
  }

  .bad {
    color: var(--bad);
  }

  .watched {
    margin-top: 8px;
  }

  .links {
    display: flex;
    gap: 14px;
    font-size: var(--step--1);
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin-top: 18px;
    font-size: var(--step--1);
  }
</style>
