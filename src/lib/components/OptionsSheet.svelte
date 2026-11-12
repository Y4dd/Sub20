<script lang="ts">
  // How the videos are compressed. Everything here applies to the whole list.
  import { app } from "../app.svelte";
  import { engineOf, CODEC_LABEL } from "../format";
  import type { Codec, Effort, EncoderStatus } from "../types";
  import Segmented from "./Segmented.svelte";
  import Sheet from "./Sheet.svelte";
  import Toggle from "./Toggle.svelte";

  const o = $derived(app.options);
  let naming = $state(false);
  let profileName = $state("");

  const disabled = $derived(new Set(app.settings?.disabledEncoders ?? []));
  const working = (codec: Codec): EncoderStatus[] =>
    (app.report?.encoders ?? []).filter((e) => e.codec === codec && e.state === "works");
  const usable = (codec: Codec): EncoderStatus[] => working(codec).filter((e) => !disabled.has(e.name));
  /** The encoder a job will really use. */
  const chosen = $derived(
    o.encoder ? (app.report?.encoders.find((e) => e.name === o.encoder) ?? null) : (usable(o.codec)[0] ?? working(o.codec)[0] ?? null),
  );

  const CODEC_NOTE: Record<Codec, string> = {
    h264: "Plays everywhere, including in Discord's chat.",
    hevc: "Sharper at the same size, but Discord won't play it in the chat: people have to download it to watch.",
    av1: "Sharpest at the same size. Needs a recent device to play, and is slow without a recent graphics card.",
  };

  const EFFORTS: Effort[] = ["p1", "p2", "p3", "p4", "p5", "p6", "p7", "max"];
  const effortIndex = $derived(EFFORTS.indexOf(o.effort));
  const EFFORT_NOTE = [
    "Fastest. Fine for a quick share.",
    "Very fast.",
    "Fast.",
    "Balanced.",
    "A little slower, a little sharper.",
    "Slower and sharper.",
    "Slow. Noticeably better on busy footage.",
    "Slowest. On the CPU it studies the whole video first, then encodes it.",
  ];

  type Res = "auto" | "original" | 1080 | 720 | 480 | "custom";
  const res: Res = $derived(
    o.autoResolution ? "auto"
    : o.maxShortSide === 1080 || o.maxShortSide === 720 || o.maxShortSide === 480 ? o.maxShortSide
    : o.maxShortSide || o.maxWidth || o.maxHeight ? "custom"
    : "original",
  );

  function setRes(value: Res) {
    o.autoResolution = value === "auto";
    o.maxWidth = null;
    o.maxHeight = null;
    o.maxShortSide = typeof value === "number" ? value : null;
    if (value === "custom") {
      o.maxWidth = 1280;
      o.maxHeight = 720;
    }
  }

  const number = (text: string): number | null => {
    const value = Number(text);
    return text.trim() !== "" && Number.isFinite(value) && value > 0 ? value : null;
  };

  async function saveProfile() {
    const name = profileName.trim();
    if (!name) return;
    const options = $state.snapshot(app.options);
    await app.saveSettings((s) => {
      const existing = s.profiles.find((p) => p.name === name);
      if (existing) existing.options = options;
      else s.profiles.push({ name, options });
    });
    app.profile = name;
    naming = false;
    profileName = "";
  }
</script>

<Sheet title="Options" onclose={() => (app.sheet = null)}>
  <div class="rows">
    <div class="row">
      <span class="label">Format</span>
      <div>
        <Segmented
          label="Format"
          value={o.codec}
          options={(["h264", "hevc", "av1"] as Codec[]).map((codec) => ({
            value: codec,
            label: CODEC_LABEL[codec],
            disabled: app.report !== null && working(codec).length === 0,
            hint: app.report !== null && working(codec).length === 0 ? "No encoder for this format works on this computer" : undefined,
          }))}
          onchange={(codec) => { o.codec = codec; o.encoder = null; }}
        />
        <p class="note">
          {CODEC_NOTE[o.codec]}
          {#if chosen}
            {chosen.family === "cpu" ? "Encoded on the CPU." : `Encoded on the ${engineOf(chosen.name)}.`}
          {/if}
        </p>
      </div>
    </div>

    <div class="row">
      <span class="label">Resolution</span>
      <div>
        <Segmented
          label="Resolution"
          value={res}
          options={[
            { value: "auto", label: "Auto" },
            { value: "original", label: "Original" },
            { value: 1080, label: "1080p" },
            { value: 720, label: "720p" },
            { value: 480, label: "480p" },
          ]}
          onchange={setRes}
        />
        <p class="note">
          {#if res === "auto"}
            Lowers the resolution only when the size limit would otherwise smear the picture.
          {:else if res === "custom"}
            Limited to {o.maxWidth ?? "any"} × {o.maxHeight ?? "any"}. Change it under More.
          {:else if res === "original"}
            Keeps every pixel, even when the size limit is tight.
          {:else}
            Videos larger than this are scaled down. Smaller ones are left alone.
          {/if}
        </p>
      </div>
    </div>

    <div class="row">
      <span class="label">Frame rate</span>
      <Segmented
        label="Frame rate"
        value={o.maxFps}
        options={[
          { value: null, label: "Original" },
          { value: 60, label: "60" },
          { value: 30, label: "30" },
          { value: 24, label: "24" },
        ]}
        onchange={(fps) => (o.maxFps = fps)}
      />
    </div>

    <div class="row">
      <span class="label">Sound</span>
      <div class="pair">
        <Segmented
          label="Sound"
          value={o.audio}
          options={[
            { value: "opus", label: "Opus" },
            { value: "aac", label: "AAC" },
            { value: "none", label: "None", disabled: o.audioOnly },
          ]}
          onchange={(audio) => (o.audio = audio)}
        />
        <select class="field" aria-label="Sound bitrate" disabled={o.audio === "none"} value={o.audioKbps} onchange={(event) => (o.audioKbps = Number(event.currentTarget.value))}>
          {#each [64, 96, 128, 160, 192, 256] as kbps (kbps)}
            <option value={kbps}>{kbps} kbps</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="row">
      <label for="opt-effort">Effort</label>
      <div>
        <input
          id="opt-effort"
          type="range"
          min="0"
          max="7"
          step="1"
          value={effortIndex}
          aria-valuetext={EFFORT_NOTE[effortIndex]}
          oninput={(event) => (o.effort = EFFORTS[Number(event.currentTarget.value)])}
        />
        <p class="note">{EFFORT_NOTE[effortIndex]}</p>
      </div>
    </div>

    <div class="row">
      <span class="label">File type</span>
      <div>
        <Segmented
          label="File type"
          value={o.container}
          options={[
            { value: "mp4", label: "MP4" },
            { value: "mkv", label: "MKV" },
          ]}
          onchange={(container) => (o.container = container)}
        />
        {#if o.container === "mp4" && o.audio === "opus" && !o.audioOnly}
          <p class="note">MP4 files get AAC sound, which more players understand. Choose MKV to keep Opus.</p>
        {/if}
      </div>
    </div>

    <!-- Profiles come last: they are for coming back to, not for each video. -->
    <div class="row profile">
      <label for="opt-profile">Profile</label>
      <div>
        <select id="opt-profile" class="field" value={app.profile ?? ""} onchange={(event) => app.loadProfile(event.currentTarget.value)}>
          {#if !app.profile}<option value="">Unsaved</option>{/if}
          {#each app.settings?.profiles ?? [] as profile (profile.name)}
            <option value={profile.name}>{profile.name}{profile.name === app.settings?.defaultProfile ? " (default)" : ""}</option>
          {/each}
        </select>
        {#if naming}
          <form class="inline" onsubmit={(event) => { event.preventDefault(); void saveProfile(); }}>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="field" placeholder="Name for these options" aria-label="Profile name" bind:value={profileName} autofocus />
            <button type="submit" class="link">Save</button>
          </form>
        {:else}
          <p class="note">
            <button type="button" class="link quiet" onclick={() => { naming = true; profileName = app.profile ?? ""; }}>Save these options as a profile</button>
          </p>
        {/if}
      </div>
    </div>
  </div>

  <details>
    <summary>More</summary>
    <div class="rows">
      <div class="row">
        <label for="opt-encoder">Encoder</label>
        <div>
          <select id="opt-encoder" class="field" value={o.encoder ?? ""} onchange={(event) => (o.encoder = event.currentTarget.value || null)}>
            <option value="">Best available</option>
            {#each working(o.codec) as encoder (encoder.name)}
              <option value={encoder.name}>{encoder.familyLabel} ({encoder.name})</option>
            {/each}
          </select>
        </div>
      </div>

      {#if chosen?.family === "nvenc"}
        <div class="row">
          <label for="opt-tune">NVIDIA tuning</label>
          <select id="opt-tune" class="field" bind:value={o.tune}>
            <option value="hq">High quality</option>
            <option value="ll">Low latency</option>
            <option value="ull">Ultra-low latency</option>
            <option value="lossless">Lossless</option>
          </select>
        </div>
      {/if}

      <div class="row">
        <label for="opt-kbps">Fixed bitrate</label>
        <div>
          <div class="suffix">
            <input id="opt-kbps" class="field" inputmode="numeric" placeholder="Off" value={o.videoKbps ?? ""} onchange={(event) => (o.videoKbps = number(event.currentTarget.value))} />
            <span>kbps</span>
          </div>
          <p class="note">Encodes the picture at this bitrate and ignores the size limit.</p>
        </div>
      </div>

      {#if res === "custom" || (!o.autoResolution && (o.maxWidth || o.maxHeight))}
        <div class="row">
          <span class="label">Largest size</span>
          <div class="pair even">
            <div class="suffix">
              <input class="field" inputmode="numeric" aria-label="Largest width" placeholder="Any" value={o.maxWidth ?? ""} onchange={(event) => (o.maxWidth = number(event.currentTarget.value))} />
              <span>wide</span>
            </div>
            <div class="suffix">
              <input class="field" inputmode="numeric" aria-label="Largest height" placeholder="Any" value={o.maxHeight ?? ""} onchange={(event) => (o.maxHeight = number(event.currentTarget.value))} />
              <span>tall</span>
            </div>
          </div>
        </div>
      {:else}
        <div class="row">
          <span class="label">Largest size</span>
          <p class="note flush"><button type="button" class="link quiet" onclick={() => setRes("custom")}>Set a width and height limit</button></p>
        </div>
      {/if}

      {#if o.autoResolution}
        <div class="row">
          <label for="opt-floor">Never below</label>
          <select id="opt-floor" class="field" value={o.minAutoHeight} onchange={(event) => (o.minAutoHeight = Number(event.currentTarget.value))}>
            {#each [240, 360, 480, 720] as side (side)}
              <option value={side}>{side}p</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>

    <div class="toggles">
      <Toggle checked={o.mixAudio} disabled={o.audio === "none" && !o.audioOnly} onchange={(on) => (o.mixAudio = on)}>
        Mix all sound tracks
        <small>Keeps a microphone that OBS or the NVIDIA app recorded on its own track.</small>
      </Toggle>
      <Toggle checked={o.audioOnly} onchange={(on) => { o.audioOnly = on; if (on && o.audio === "none") o.audio = "aac"; }}>
        Keep only the sound
        <small>Writes an audio file and drops the picture.</small>
      </Toggle>
      <Toggle checked={o.hardwareDecode} onchange={(on) => (o.hardwareDecode = on)}>
        Read videos with the graphics card
        <small>Faster on large files. Falls back by itself if it fails.</small>
      </Toggle>
      <Toggle checked={o.fastFinalize} disabled={o.container !== "mp4"} onchange={(on) => (o.fastFinalize = on)}>
        Skip the finishing step on MP4
        <small>Long videos save sooner. A few older players cannot open the result.</small>
      </Toggle>
    </div>
  </details>
</Sheet>

<style>
  .rows {
    display: grid;
    gap: 18px;
  }

  .row {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr);
    gap: 12px;
    align-items: start;
  }

  .row.profile {
    padding-top: 18px;
    border-top: 1px solid var(--line);
  }

  .row > label,
  .row > .label {
    padding-top: 8px;
    color: var(--muted);
  }

  .note {
    margin-top: 7px;
    color: var(--muted);
    font-size: var(--step--1);
    line-height: 1.45;
  }

  .note.flush {
    margin-top: 0;
    padding-top: 9px;
  }

  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 112px;
    gap: 8px;
  }

  .pair.even {
    grid-template-columns: 1fr 1fr;
  }

  .inline {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 8px;
  }

  .suffix {
    position: relative;
  }

  .suffix span {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    pointer-events: none;
  }

  input[type="range"] {
    width: 100%;
    height: 22px;
    margin: 6px 0 0;
    accent-color: var(--trim);
  }

  details {
    margin-top: 26px;
    border-top: 1px solid var(--line);
  }

  summary {
    padding: 14px 0;
    color: var(--muted);
    cursor: pointer;
    list-style-position: inside;
  }

  summary:hover {
    color: var(--text);
  }

  .toggles {
    display: grid;
    gap: 14px;
    margin-top: 22px;
  }

  .toggles small {
    display: block;
    color: var(--muted);
    font-size: var(--step--1);
  }
</style>
