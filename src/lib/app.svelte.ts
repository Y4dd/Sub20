// The window's state: the clips on the bench, what is selected, the options
// in force, and the bridge to the backend.

import { mockBackend } from "./mock";
import { tauriBackend } from "./tauri";
import { baseName, plain } from "./format";
import type {
  Backend, EncodeOptions, FfmpegStatus, JobEvent, JobResult, MediaInfo, Phase, Plan, Report, Settings, Trim,
  UpdateInfo,
} from "./types";

export const backend: Backend = "__TAURI_INTERNALS__" in window ? tauriBackend : mockBackend;

export type ClipStatus = "ready" | "queued" | "running" | "done" | "failed" | "canceled";

/** Shortest piece a trim may keep, in seconds. */
export const MIN_KEEP = 0.5;

const LOG_LINES = 400;

export class Clip {
  readonly key = crypto.randomUUID();
  readonly path: string;
  readonly name: string;
  readonly info: MediaInfo;

  start = $state(0);
  end = $state(0);
  thumbs = $state.raw<string[]>([]);
  /** What to call the result, as typed in the clip list; null uses the default. */
  outputName = $state<string | null>(null);
  status = $state<ClipStatus>("ready");
  jobId = $state<string | null>(null);
  phase = $state<Phase | null>(null);
  percent = $state(0);
  eta = $state<number | null>(null);
  encoder = $state<string | null>(null);
  retries = $state(0);
  fallback = $state<{ from: string; to: string } | null>(null);
  result = $state<JobResult | null>(null);
  /** Just copied to the clipboard; the Copy button says so for a moment. */
  copied = $state(false);
  error = $state<string | null>(null);
  log = $state.raw<string[]>([]);
  plan = $state.raw<Plan | null>(null);
  planError = $state<string | null>(null);

  constructor(path: string, name: string, info: MediaInfo) {
    this.path = path;
    this.name = name;
    this.info = info;
    this.end = info.duration;
  }

  get duration(): number {
    return this.info.duration;
  }

  get trimmed(): boolean {
    return this.start > 0.02 || this.duration - this.end > 0.02;
  }

  get trim(): Trim | null {
    return this.trimmed ? { start: this.start, end: this.end } : null;
  }

  get kept(): number {
    return this.end - this.start;
  }

  /** Queued or encoding: the trim is locked. */
  get busy(): boolean {
    return this.status === "queued" || this.status === "running";
  }

  /** Will be included the next time "Compress" is pressed. */
  get pending(): boolean {
    return (this.status === "ready" || this.status === "failed" || this.status === "canceled") && !this.planError;
  }
}

export type Sheet = "options" | "settings" | "history" | "keys" | null;

const FALLBACK_OPTIONS: EncodeOptions = {
  targetMb: 10, videoKbps: null, codec: "h264", encoder: null, audio: "opus", audioKbps: 128, effort: "p4",
  tune: "hq", container: "mp4", autoResolution: true, minAutoHeight: 240, maxShortSide: null, maxWidth: null,
  maxHeight: null, maxFps: null, audioOnly: false, fastFinalize: false, hardwareDecode: false, mixAudio: true,
};

class App {
  ready = $state(false);
  version = $state("");
  /** FFmpeg ships inside this build. */
  bundled = $state(false);
  ffmpegNotice = $state<string | null>(null);
  clips = $state<Clip[]>([]);
  selectedKey = $state<string | null>(null);
  options = $state<EncodeOptions>({ ...FALLBACK_OPTIONS });
  /** The profile the options were last loaded from. */
  profile = $state<string | null>(null);
  settings = $state<Settings | null>(null);
  ffmpeg = $state<FfmpegStatus>({ found: true, version: null, folder: null });
  report = $state.raw<Report | null>(null);
  sheet = $state<Sheet>(null);
  /** Files are being dragged over the window. */
  dragging = $state(false);
  /** The clip whose output name is being edited in the list. */
  renaming = $state<Clip | null>(null);
  /** Dropped files are being read. */
  adding = $state(false);
  notices = $state<{ id: number; text: string }[]>([]);
  /** A newer release, found at startup when the settings allow it. */
  update = $state<UpdateInfo | null>(null);
  /** The update is downloading and about to restart the app. */
  updating = $state(false);

  #byJob = new Map<string, Clip>();
  #early: JobEvent[] = [];
  #planTimers = new Map<string, number>();
  #noticeId = 0;
  #saving: Promise<void> = Promise.resolve();

  get selected(): Clip | null {
    return this.clips.find((clip) => clip.key === this.selectedKey) ?? this.clips[0] ?? null;
  }

  get pending(): Clip[] {
    return this.clips.filter((clip) => clip.pending);
  }

  get busy(): Clip[] {
    return this.clips.filter((clip) => clip.busy);
  }

  get finished(): Clip[] {
    return this.clips.filter((clip) => clip.status === "done");
  }

  async init() {
    const boot = await backend.bootstrap();
    this.settings = boot.settings;
    this.ffmpeg = boot.ffmpeg;
    this.version = boot.version;
    this.bundled = boot.bundled;
    this.ffmpegNotice = boot.ffmpegNotice;
    this.loadProfile(boot.settings.defaultProfile);
    if (boot.settings.lastOptions) this.#restore(boot.settings.lastOptions);
    this.#rememberOptions();
    await backend.onJob((event) => this.#onJob(event));
    await backend.onDrop((event) => {
      if (event.type === "drop") {
        this.dragging = false;
        void this.add(event.paths);
      } else {
        this.dragging = event.type === "over";
      }
    });
    this.ready = true;
    if (boot.ffmpeg.found) void this.loadReport(false);
    if (boot.openedWith.length) void this.add(boot.openedWith);
    if (boot.settings.checkUpdates) void this.checkUpdate(true);
  }

  /** Looks for a newer release. At startup (`quietly`) a failure says nothing: being offline is normal. */
  async checkUpdate(quietly = false) {
    try {
      this.update = await backend.checkUpdate();
      if (!quietly && !this.update) this.notify(`Sub20 ${this.version} is the latest version.`);
    } catch (error) {
      if (!quietly) this.notify(`Could not check for updates: ${error}`);
    }
  }

  /** Downloads the update, stops any running jobs and restarts into the new version. */
  async installUpdate() {
    if (!this.update || this.updating) return;
    this.updating = true;
    try {
      await backend.installUpdate();
    } catch (error) {
      this.updating = false;
      this.notify(`The update did not install: ${error}. Try again, or download it from the Releases page.`);
    }
  }

  async loadReport(refresh: boolean) {
    try {
      this.report = await backend.encoders(refresh);
    } catch (error) {
      this.notify(String(error));
    }
  }

  loadProfile(name: string) {
    const profile = this.settings?.profiles.find((p) => p.name === name);
    if (!profile) return;
    this.options = structuredClone($state.snapshot(profile.options));
    this.profile = name;
  }

  /** Starts from the options the app last closed with. */
  #restore(options: EncodeOptions) {
    this.options = { ...FALLBACK_OPTIONS, ...options };
    const same = JSON.stringify($state.snapshot(this.options));
    this.profile = this.settings?.profiles.find((p) => JSON.stringify({ ...FALLBACK_OPTIONS, ...p.options }) === same)?.name ?? null;
  }

  /** Saves the options a second after they stop changing, for the next launch. */
  #rememberOptions() {
    let timer = 0;
    let first = true;
    $effect.root(() => {
      $effect(() => {
        const options = $state.snapshot(this.options);
        if (first) {
          first = false;
          return;
        }
        window.clearTimeout(timer);
        timer = window.setTimeout(() => void this.saveSettings((s) => (s.lastOptions = options)), 1000);
      });
    });
  }

  /** Puts what a bug report needs on the clipboard: versions, encoders, crashes and the shown job's log. */
  async copyDiagnostics() {
    try {
      await backend.copyDiagnostics($state.snapshot(this.selected?.log ?? []));
      this.notify("Diagnostics copied. Paste them into a bug report on GitHub.");
    } catch (error) {
      this.notify(`Could not copy the diagnostics: ${error}`);
    }
  }

  notify(text: string) {
    const id = ++this.#noticeId;
    this.notices.push({ id, text });
    window.setTimeout(() => this.dismiss(id), 7000);
  }

  dismiss(id: number) {
    this.notices = this.notices.filter((notice) => notice.id !== id);
  }

  async choose() {
    const paths = await backend.pickVideos();
    if (paths.length) await this.add(paths);
  }

  async add(paths: string[]) {
    if (!paths.length) return;
    this.adding = true;
    try {
      const results = await backend.addVideos(paths);
      let first: Clip | null = null;
      const rejected: string[] = [];
      for (const item of results) {
        if (!item.info) {
          rejected.push(item.name);
          continue;
        }
        if (this.clips.some((clip) => clip.path === item.path)) continue;
        const clip = new Clip(item.path, item.name, item.info);
        this.clips.push(clip);
        first ??= clip;
        this.replan(clip, 0);
        void this.#loadThumbs(clip);
      }
      if (first) this.selectedKey = first.key;
      if (rejected.length === 1) this.notify(`${rejected[0]} is not a video this app can read.`);
      else if (rejected.length > 1) this.notify(`${rejected.length} files are not videos this app can read.`);
    } catch (error) {
      this.notify(String(error));
      this.ffmpeg = await backend.ffmpegStatus();
    } finally {
      this.adding = false;
    }
  }

  async #loadThumbs(clip: Clip) {
    const ratio = clip.info.displayWidth / Math.max(1, clip.info.displayHeight);
    // Enough tiles to fill a wide timeline: fewer for landscape, more for portrait.
    const count = Math.round(Math.min(32, Math.max(10, 22 / ratio)));
    // Empty places fill in as each thumbnail arrives.
    clip.thumbs = Array(count).fill("");
    try {
      clip.thumbs = await backend.filmstrip(clip.path, clip.duration, count, (index, url) => {
        const next = [...clip.thumbs];
        next[index] = url;
        clip.thumbs = next;
      });
    } catch {
      clip.thumbs = [];
    }
  }

  select(clip: Clip) {
    this.selectedKey = clip.key;
  }

  /** Moves the selection up or down the list. */
  step(offset: number) {
    const current = this.selected;
    if (!current) return;
    const index = this.clips.indexOf(current) + offset;
    if (index >= 0 && index < this.clips.length) this.selectedKey = this.clips[index].key;
  }

  remove(clip: Clip) {
    if (clip.busy && clip.jobId) void backend.cancel(clip.jobId);
    const index = this.clips.indexOf(clip);
    this.clips = this.clips.filter((other) => other !== clip);
    if (this.selectedKey === clip.key) {
      this.selectedKey = (this.clips[index] ?? this.clips[index - 1])?.key ?? null;
    }
  }

  clearFinished() {
    this.clips = this.clips.filter((clip) => clip.status !== "done");
    if (!this.clips.some((clip) => clip.key === this.selectedKey)) this.selectedKey = this.clips[0]?.key ?? null;
  }

  /** Sets a clip's trim, keeping it inside the video and at least MIN_KEEP long. */
  setTrim(clip: Clip, start: number, end: number) {
    // A replaced original is in the bin: there is nothing left to trim.
    if (clip.busy || clip.result?.replacedOriginal) return;
    const least = Math.min(MIN_KEEP, clip.duration);
    start = Math.min(Math.max(0, start), clip.duration - least);
    end = Math.max(Math.min(clip.duration, end), start + least);
    if (start === clip.start && end === clip.end) return;
    clip.start = start;
    clip.end = end;
    // Changing the cut makes an earlier result stale.
    if (clip.status !== "ready") this.reset(clip);
    this.replan(clip);
  }

  /** Puts a finished or failed clip back in line. */
  reset(clip: Clip) {
    clip.status = "ready";
    clip.result = null;
    clip.error = null;
    clip.percent = 0;
    clip.eta = null;
    clip.phase = null;
    clip.retries = 0;
    clip.fallback = null;
    clip.jobId = null;
  }

  /** Recomputes what the encode will look like, shortly after the last change. */
  replan(clip: Clip, delay = 60) {
    window.clearTimeout(this.#planTimers.get(clip.key));
    const run = async () => {
      const { start, end } = clip;
      try {
        const plan = await backend.estimate($state.snapshot(clip.info), clip.trim, $state.snapshot(this.options));
        if (clip.start !== start || clip.end !== end) return; // Stale answer.
        clip.plan = plan;
        clip.planError = null;
      } catch (error) {
        clip.plan = null;
        clip.planError = String(error);
      }
    };
    this.#planTimers.set(clip.key, window.setTimeout(run, delay));
  }

  replanAll() {
    for (const clip of this.clips) if (!clip.busy) this.replan(clip);
  }

  async compress() {
    const clips = this.pending;
    if (!clips.length) return;
    const options = $state.snapshot(this.options);
    for (const clip of clips) {
      this.reset(clip);
      clip.status = "queued";
      clip.log = [];
    }
    try {
      const ids = await backend.compress(
        clips.map((clip) => ({ path: clip.path, trim: clip.trim, options, outputName: clip.outputName })),
      );
      ids.forEach((id, index) => {
        clips[index].jobId = id;
        this.#byJob.set(id, clips[index]);
      });
      const early = this.#early;
      this.#early = [];
      for (const event of early) this.#onJob(event);
    } catch (error) {
      for (const clip of clips) {
        clip.status = "failed";
        clip.error = String(error);
      }
      this.ffmpeg = await backend.ffmpegStatus();
    }
  }

  async cancel(clip: Clip) {
    if (clip.jobId) await backend.cancel(clip.jobId);
  }

  async cancelAll() {
    await backend.cancelAll();
  }

  /** Puts a finished clip on the clipboard, for pasting into Discord. */
  async copy(clip: Clip) {
    if (!clip.result) return;
    try {
      await backend.copyFile(clip.result.output);
      clip.copied = true;
      window.setTimeout(() => (clip.copied = false), 2400);
    } catch (error) {
      this.notify(String(error));
    }
  }

  /** Drags a finished clip out of the window, into Discord or a folder. */
  async dragOut(clip: Clip) {
    if (!clip.result) return;
    const thumb = clip.thumbs[Math.floor(clip.thumbs.length / 2)] || null;
    try {
      await backend.dragOut(clip.result.output, thumb);
    } catch (error) {
      this.notify(String(error));
    }
  }

  /** The extension the result has or will have, with its dot. */
  extensionOf(clip: Clip): string {
    if (clip.result) return clip.result.output.slice(clip.result.output.lastIndexOf("."));
    if (this.options.audioOnly) return this.options.audio === "opus" && this.options.container === "mkv" ? ".opus" : ".m4a";
    return `.${this.options.container}`;
  }

  /** The name the result has or will get, without its extension. */
  outputStem(clip: Clip): string {
    if (clip.result) {
      const file = baseName(clip.result.output);
      return file.slice(0, file.lastIndexOf("."));
    }
    return clip.outputName ?? this.defaultStem(clip);
  }

  /** The name a result gets when none was typed. */
  defaultStem(clip: Clip): string {
    const stem = clip.name.replace(/\.[^.]+$/, "");
    if (this.settings?.outputMode === "replace") return stem;
    if (this.options.audioOnly) return `${stem}-audio`;
    if (this.options.videoKbps) return `${stem}-${this.options.videoKbps}k`;
    return `${stem}-${plain(this.options.targetMb)}mb`;
  }

  /**
   * Sets what a clip's result is called. A finished clip's file is renamed
   * on disk; otherwise the name is used when it is compressed. An empty name
   * goes back to the default.
   */
  async rename(clip: Clip, typed: string) {
    this.renaming = null;
    const name = typed.trim();
    if (clip.result) {
      if (!name || name === this.outputStem(clip)) return;
      try {
        const output = await backend.renameResult(clip.result.output, name);
        clip.result = { ...clip.result, output };
        clip.outputName = this.outputStem(clip);
      } catch (error) {
        this.notify(String(error));
      }
      return;
    }
    if (clip.busy) return;
    clip.outputName = name && name !== this.defaultStem(clip) ? name : null;
  }

  async saveSettings(change: (settings: Settings) => void) {
    // One save at a time, each starting from what the last one stored, so
    // two quick changes can't overwrite each other.
    const turn = this.#saving.then(() => this.#save(change));
    this.#saving = turn.catch(() => {});
    return turn;
  }

  async #save(change: (settings: Settings) => void) {
    if (!this.settings) return;
    const next = structuredClone($state.snapshot(this.settings));
    change(next);
    try {
      const before = this.settings.ffmpegFolder;
      this.settings = await backend.saveSettings(next);
      if (this.settings.ffmpegFolder !== before) {
        this.ffmpeg = await backend.ffmpegStatus();
        if (this.ffmpeg.found) void this.loadReport(true);
      }
    } catch (error) {
      this.notify(String(error));
    }
  }

  #onJob(event: JobEvent) {
    const clip = this.#byJob.get(event.id);
    if (!clip) {
      // Either a folder-watch job, or an event that beat the reply to
      // `compress`. Keep the latter until the ids are known.
      if (this.clips.some((c) => c.status === "queued" && !c.jobId)) this.#early.push(event);
      return;
    }
    if (clip.jobId !== event.id) return; // From an earlier run of this clip.
    switch (event.type) {
      case "queued":
        clip.status = "queued";
        break;
      case "started":
        clip.status = "running";
        clip.encoder = event.encoder;
        break;
      case "progress":
        clip.status = "running";
        clip.phase = event.phase;
        clip.percent = event.percent;
        clip.eta = event.etaSeconds;
        break;
      case "log":
        clip.log = [...clip.log.slice(-(LOG_LINES - 1)), event.line];
        break;
      case "fallback":
        clip.fallback = { from: event.from, to: event.to };
        clip.encoder = event.to;
        break;
      case "retry":
        clip.retries = event.attempt;
        clip.percent = 0;
        clip.eta = null;
        break;
      case "done":
        clip.status = "done";
        clip.percent = 100;
        clip.result = event.result;
        this.#byJob.delete(event.id);
        break;
      case "failed":
        clip.status = "failed";
        clip.error = event.error;
        this.#byJob.delete(event.id);
        break;
      case "canceled":
        clip.status = "canceled";
        clip.percent = 0;
        this.#byJob.delete(event.id);
        break;
    }
  }
}

export const app = new App();
