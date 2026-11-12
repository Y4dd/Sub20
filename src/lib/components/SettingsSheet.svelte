<script lang="ts">
  import { app, backend } from "../app.svelte";
  import { plain, when, BIN, CODEC_LABEL } from "../format";
  import type { EncoderStatus, Family, Profile, WatchStatus } from "../types";
  import Icon from "./Icon.svelte";
  import Segmented from "./Segmented.svelte";
  import Sheet from "./Sheet.svelte";
  import Toggle from "./Toggle.svelte";

  const s = $derived(app.settings!);
  let newSize = $state("");
  let testing = $state(false);
  let watch = $state.raw<WatchStatus | null>(null);

  // Keep the folder-watch status fresh while the panel is open.
  $effect(() => {
    const refresh = async () => (watch = await backend.watchStatus());
    void refresh();
    const timer = window.setInterval(refresh, 3000);
    return () => window.clearInterval(timer);
  });

  function describe(profile: Profile): string {
    const o = profile.options;
    const parts = [`${plain(o.targetMb)} MB`, CODEC_LABEL[o.codec]];
    if (o.audioOnly) parts.push("sound only");
    else if (o.autoResolution) parts.push("auto resolution");
    else if (o.maxShortSide) parts.push(`${o.maxShortSide}p`);
    return parts.join(", ");
  }

  // Encoders grouped by what does the work, ones for other systems left out.
  const families = $derived.by(() => {
    const groups = new Map<Family, { label: string; encoders: EncoderStatus[] }>();
    for (const encoder of app.report?.encoders ?? []) {
      if (encoder.state === "elsewhere") continue;
      if (!groups.has(encoder.family)) groups.set(encoder.family, { label: encoder.familyLabel, encoders: [] });
      groups.get(encoder.family)!.encoders.push(encoder);
    }
    return [...groups.values()];
  });

  async function testAgain() {
    testing = true;
    await app.loadReport(true);
    testing = false;
  }

  async function pick(apply: (folder: string) => void) {
    const folder = await backend.pickFolder();
    if (folder) apply(folder);
  }

  const STATE_WORD = { works: "Works", failed: "Not working", missing: "Not in this FFmpeg", elsewhere: "" } as const;
</script>

<Sheet title="Settings" onclose={() => (app.sheet = null)}>
  <section>
    <h3>Sizes</h3>
    <p class="muted lede">The sizes offered when you click the number at the bottom of the window.</p>
    <div class="chips">
      {#each s.sizes as size (size)}
        <span class="chip">
          {plain(size)} MB
          <button type="button" aria-label="Remove {plain(size)} MB" disabled={s.sizes.length === 1}
            onclick={() => app.saveSettings((n) => (n.sizes = n.sizes.filter((x) => x !== size)))}>
            <Icon name="close" size={12} />
          </button>
        </span>
      {/each}
      <form onsubmit={(event) => {
        event.preventDefault();
        const value = Number(newSize);
        if (Number.isFinite(value) && value > 0) void app.saveSettings((n) => n.sizes.push(value));
        newSize = "";
      }}>
        <input class="field add" inputmode="decimal" placeholder="Add a size" aria-label="Add a size in MB" bind:value={newSize} />
      </form>
    </div>
  </section>

  <section>
    <h3>Profiles</h3>
    <p class="muted lede">Saved sets of options. The default one is loaded when the app opens. Save new ones from Options.</p>
    <ul class="list">
      {#each s.profiles as profile (profile.name)}
        <li>
          <label class="radio">
            <input type="radio" name="default-profile" checked={profile.name === s.defaultProfile}
              onchange={() => app.saveSettings((n) => (n.defaultProfile = profile.name))} />
            <span>
              <strong>{profile.name}</strong>
              <small class="muted">{describe(profile)}</small>
            </span>
          </label>
          <button type="button" class="icon" aria-label="Delete {profile.name}" title="Delete" disabled={s.profiles.length === 1}
            onclick={() => app.saveSettings((n) => (n.profiles = n.profiles.filter((p) => p.name !== profile.name)))}>
            <Icon name="trash" size={16} />
          </button>
        </li>
      {/each}
    </ul>
  </section>

  <section>
    <h3>Where results go</h3>
    <Segmented
      label="Where results go"
      value={s.outputMode}
      options={[
        { value: "sameFolder", label: "Next to the original" },
        { value: "folder", label: "One folder" },
        { value: "replace", label: "Replace the original" },
      ]}
      onchange={(mode) => {
        if (mode === "folder" && !s.outputFolder) void pick((folder) => app.saveSettings((n) => { n.outputMode = "folder"; n.outputFolder = folder; }));
        else void app.saveSettings((n) => (n.outputMode = mode));
      }}
    />
    {#if s.outputMode === "folder"}
      <p class="path">
        <span title={s.outputFolder}>{s.outputFolder}</span>
        <button type="button" class="link" onclick={() => pick((folder) => app.saveSettings((n) => (n.outputFolder = folder)))}>Change</button>
      </p>
    {:else if s.outputMode === "replace"}
      <p class="muted note">The result takes the original's place, as clip.{app.options.container} or the name you gave it. The original goes to the {BIN}, so it can be put back. Watched folders still save beside the original.</p>
    {:else}
      <p class="muted note">A video called clip.mov is saved beside it as clip-{plain(app.options.targetMb)}mb.{app.options.container}. Originals are never changed.</p>
    {/if}
  </section>

  <section>
    <h3>Encoders</h3>
    <p class="muted lede">
      Each one was tested by encoding a few frames. The app uses the first working one for a format, graphics cards before the CPU. Switch one off to skip it.
    </p>
    {#if app.report}
      {#each families as family (family.label)}
        <h4>{family.label}</h4>
        <ul class="list tight">
          {#each family.encoders as encoder (encoder.name)}
            <li>
              <Toggle
                checked={encoder.state === "works" && !s.disabledEncoders.includes(encoder.name)}
                disabled={encoder.state !== "works"}
                onchange={(on) => app.saveSettings((n) => {
                  n.disabledEncoders = n.disabledEncoders.filter((name) => name !== encoder.name);
                  if (!on) n.disabledEncoders.push(encoder.name);
                })}
              >
                {CODEC_LABEL[encoder.codec]}{encoder.name === "libaom-av1" ? " (libaom, very slow)" : ""}
                <small class={encoder.state === "works" ? "works" : "muted"} title={encoder.detail ?? undefined}>
                  {STATE_WORD[encoder.state]}{encoder.detail ? `: ${encoder.detail}` : ""}
                </small>
              </Toggle>
            </li>
          {/each}
        </ul>
      {/each}
      <p class="note">
        <button type="button" class="link" onclick={testAgain} disabled={testing}>{testing ? "Testing…" : "Test again"}</button>
        <span class="muted">Last tested {when(app.report.checkedAt).toLowerCase()}.</span>
      </p>
    {:else if app.ffmpeg.found}
      <p class="muted">Testing encoders…</p>
    {:else}
      <p class="muted">FFmpeg has to be found before encoders can be tested.</p>
    {/if}
  </section>

  <section>
    <h3>Watch a folder</h3>
    <Toggle checked={s.watch.enabled} onchange={(on) => {
      if (on && !s.watch.folder) void pick((folder) => app.saveSettings((n) => { n.watch.enabled = true; n.watch.folder = folder; }));
      else void app.saveSettings((n) => (n.watch.enabled = on));
    }}>
      Compress videos that appear in a folder
      <small class="muted">Runs while the app is open.</small>
    </Toggle>
    {#if s.watch.enabled}
      <div class="rows">
        <div class="row">
          <span class="label">Folder</span>
          <p class="path flush">
            <span title={s.watch.folder}>{s.watch.folder ?? "None chosen"}</span>
            <button type="button" class="link" onclick={() => pick((folder) => app.saveSettings((n) => (n.watch.folder = folder)))}>Change</button>
          </p>
        </div>
        <div class="row">
          <label for="watch-profile">Profile</label>
          <select id="watch-profile" class="field" value={s.watch.profile ?? ""} onchange={(event) => app.saveSettings((n) => (n.watch.profile = event.currentTarget.value || null))}>
            <option value="">The default profile</option>
            {#each s.profiles as profile (profile.name)}
              <option value={profile.name}>{profile.name}</option>
            {/each}
          </select>
        </div>
        <div class="row">
          <label for="watch-output">Results</label>
          <div>
            <select id="watch-output" class="field" value={s.watch.outputMode} onchange={(event) => {
              const mode = event.currentTarget.value as "sameFolder" | "folder";
              if (mode === "folder" && !s.watch.outputFolder) void pick((folder) => app.saveSettings((n) => { n.watch.outputMode = "folder"; n.watch.outputFolder = folder; }));
              else void app.saveSettings((n) => (n.watch.outputMode = mode));
            }}>
              <option value="sameFolder">Next to the original</option>
              <option value="folder">In another folder</option>
            </select>
            {#if s.watch.outputMode === "folder"}
              <p class="path">
                <span title={s.watch.outputFolder}>{s.watch.outputFolder ?? "None chosen"}</span>
                <button type="button" class="link" onclick={() => pick((folder) => app.saveSettings((n) => (n.watch.outputFolder = folder)))}>Change</button>
              </p>
            {/if}
          </div>
        </div>
        <div class="row">
          <label for="watch-originals">Originals</label>
          <select id="watch-originals" class="field" value={s.watch.originals} onchange={(event) => app.saveSettings((n) => (n.watch.originals = event.currentTarget.value as "keep" | "delete" | "move"))}>
            <option value="keep">Leave them where they are</option>
            <option value="move">Move them into a "processed" folder</option>
            <option value="delete">Delete them once the result is saved</option>
          </select>
        </div>
        <div class="row">
          <label for="watch-existing">Already there</label>
          <select id="watch-existing" class="field" value={s.watch.existing} onchange={(event) => app.saveSettings((n) => (n.watch.existing = event.currentTarget.value as "newOnly" | "processExisting"))}>
            <option value="newOnly">Skip videos already in the folder</option>
            <option value="processExisting">Compress those too</option>
          </select>
        </div>
        <div class="row">
          <label for="watch-stable">Wait</label>
          <div class="pair">
            <div class="suffix">
              <input id="watch-stable" class="field" type="number" min="2" max="60" value={s.watch.stableSeconds}
                onchange={(event) => app.saveSettings((n) => (n.watch.stableSeconds = Number(event.currentTarget.value)))} />
              <span>s unchanged</span>
            </div>
            <div class="suffix">
              <input class="field" type="number" min="2" max="300" aria-label="Seconds between checks" value={s.watch.pollSeconds}
                onchange={(event) => app.saveSettings((n) => (n.watch.pollSeconds = Number(event.currentTarget.value)))} />
              <span>s between checks</span>
            </div>
          </div>
        </div>
      </div>
      <Toggle checked={s.watch.recursive} onchange={(on) => app.saveSettings((n) => (n.watch.recursive = on))}>
        Look inside subfolders too
      </Toggle>
      {#if watch}
        <p class="note" class:bad={!!watch.problem}>
          {#if watch.problem}
            {watch.problem}
          {:else if watch.running}
            Watching. {watch.done} compressed{watch.failed ? `, ${watch.failed} failed` : ""}{watch.inProgress ? `, ${watch.inProgress} in progress` : ""} since the app opened.
          {:else}
            Not watching.
          {/if}
        </p>
      {/if}
    {/if}
  </section>

  <section>
    <h3>Working</h3>
    <div class="rows">
      <div class="row wide">
        <label for="set-parallel">Videos at once</label>
        <select id="set-parallel" class="field" value={s.parallelJobs} onchange={(event) => app.saveSettings((n) => (n.parallelJobs = Number(event.currentTarget.value)))}>
          <option value={0}>Decide for me</option>
          {#each [1, 2, 3, 4, 6, 8] as count (count)}
            <option value={count}>{count}</option>
          {/each}
        </select>
      </div>
      <div class="row wide">
        <label for="set-keep">Keep history</label>
        <select id="set-keep" class="field" value={s.historyEnabled ? s.historyKeepDays : -1} onchange={(event) => {
          const days = Number(event.currentTarget.value);
          void app.saveSettings((n) => { n.historyEnabled = days >= 0; if (days >= 0) n.historyKeepDays = days; });
        }}>
          <option value={0}>Forever</option>
          <option value={7}>For a week</option>
          <option value={30}>For 30 days</option>
          <option value={90}>For 90 days</option>
          <option value={-1}>Never</option>
        </select>
      </div>
    </div>
  </section>

  <section>
    <h3>FFmpeg</h3>
    {#if app.ffmpeg.found}
      <p class="path flush">
        <span title={app.ffmpeg.folder}>{app.ffmpeg.version ?? "Found"} in {app.ffmpeg.folder}</span>
      </p>
    {:else}
      <p class="bad">Not found. Install FFmpeg, or choose the folder that holds ffmpeg and ffprobe.</p>
    {/if}
    <p class="note">
      <button type="button" class="link" onclick={() => pick((folder) => app.saveSettings((n) => (n.ffmpegFolder = folder)))}>Use FFmpeg from another folder</button>
      {#if s.ffmpegFolder}
        <button type="button" class="link quiet" onclick={() => app.saveSettings((n) => (n.ffmpegFolder = null))}>Find it automatically</button>
      {/if}
    </p>
  </section>

  <p class="about muted">
    <button type="button" class="link quiet" onclick={() => (app.sheet = "keys")}>Keyboard shortcuts (?)</button><br />
    Sub20 {app.version}, free and open source under the MIT licence.
  </p>
</Sheet>

<style>
  section {
    padding: 20px 0 22px;
    border-top: 1px solid var(--line);
  }

  section:first-child {
    border-top: 0;
    padding-top: 6px;
  }

  h3 {
    margin-bottom: 10px;
    font-family: var(--display);
    font-stretch: 88%;
    font-size: var(--step-2);
    font-weight: 600;
  }

  h4 {
    margin: 14px 0 2px;
    color: var(--muted);
    font-weight: 500;
  }

  .lede {
    margin: -4px 0 12px;
    font-size: var(--step--1);
    max-width: 44em;
  }

  .note {
    margin-top: 10px;
    font-size: var(--step--1);
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
  }

  .bad {
    color: var(--bad);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 32px;
    padding: 0 6px 0 12px;
    border: 1px solid var(--line);
    border-radius: 16px;
    font-weight: 600;
  }

  .chip button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: var(--muted);
  }

  .chip button:hover:not(:disabled) {
    background: var(--raised);
    color: var(--text);
  }

  .chip button:disabled {
    visibility: hidden;
  }

  .add {
    width: 104px;
    height: 32px;
    border-radius: 16px;
    padding: 0 12px;
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .list li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 0;
  }

  .list.tight li {
    padding: 2px 0;
    display: block;
  }

  .radio {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
  }

  .radio input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--trim);
  }

  .radio span {
    display: grid;
  }

  small {
    display: block;
    font-size: var(--step--1);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  small.works {
    color: var(--ok);
  }

  .list.tight small {
    max-width: 300px;
    white-space: nowrap;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    color: var(--muted);
  }

  .icon:hover:not(:disabled) {
    background: var(--raised);
    color: var(--text);
  }

  .icon:disabled {
    visibility: hidden;
  }

  .path {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-top: 10px;
  }

  .path.flush {
    margin-top: 0;
    padding-top: 7px;
  }

  .path span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
    user-select: text;
    -webkit-user-select: text;
  }

  .rows {
    display: grid;
    gap: 12px;
    margin: 14px 0;
  }

  .row {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr);
    gap: 12px;
    align-items: start;
  }

  .row.wide {
    grid-template-columns: 140px minmax(0, 1fr);
  }

  .row > label,
  .row > .label {
    padding-top: 8px;
    color: var(--muted);
  }

  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .suffix {
    position: relative;
  }

  .suffix input {
    padding-right: 4px;
  }

  .suffix span {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    font-size: var(--step--1);
    pointer-events: none;
  }

  .about {
    padding-top: 18px;
    border-top: 1px solid var(--line);
    font-size: var(--step--1);
  }
</style>
