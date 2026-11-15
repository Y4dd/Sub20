// Sub20 Lite: the same window, run in a browser. Videos are read and encoded
// on this device with WebCodecs (through Mediabunny); nothing is uploaded.
// The browser has files, not paths, so this backend gives each file and each
// result a made-up path ("lite-3/clip.mp4") and keeps the real thing here.

import { version } from "../../../package.json";
import {
  type Added, type Backend, type Bootstrap, type Capabilities, type CompressRequest, type DropEvent, type EncodeOptions,
  type EncoderStatus, type HistoryEntry, type JobEvent, type JobResult, type Report, type Settings, type WatchStatus,
} from "../types";

// Mediabunny and the encode path load with the first video, not the page.
const media = () => import("./media");
import { gpuMostShortSide, outputFps } from "./gpu";
import { loadPlanner } from "./planner";

const LITE: Capabilities = {
  watch: false, replace: false, files: false, encoders: false, ffmpeg: false, copyFile: false, dragOut: false,
  updates: false, download: true, history: false, cut: false, toneMap: false, mixAudio: true,
  // Phones and tablets hand a file straight to the Discord app. A computer
  // downloads it instead, and the file drags into Discord from there.
  share: matchMedia("(pointer: coarse)").matches && typeof navigator.canShare === "function" && navigator.canShare({ files: [new File([], "x.mp4", { type: "video/mp4" })] }),
};

const OPTIONS: EncodeOptions = {
  targetMb: 20, videoKbps: null, codec: "h264", encoder: null, audio: "aac", audioKbps: 128, effort: "p4", tune: "hq",
  container: "mp4", autoResolution: true, minAutoHeight: 240, maxShortSide: null, maxWidth: null, maxHeight: null,
  maxFps: null, audioOnly: false, fastFinalize: false, hardwareDecode: false, mixAudio: true,
};

const SETTINGS: Settings = {
  profiles: [{ name: "Everyday", options: OPTIONS }],
  defaultProfile: "Everyday",
  sizes: [10, 20, 50, 500],
  disabledEncoders: [],
  outputMode: "sameFolder",
  outputFolder: null,
  historyEnabled: false,
  historyKeepDays: 0,
  parallelJobs: 1,
  ffmpegFolder: null,
  checkUpdates: false,
  lastOptions: null,
  watch: {
    enabled: false, folder: null, profile: null, outputMode: "sameFolder", outputFolder: null, originals: "keep",
    existing: "newOnly", recursive: false, stableSeconds: 5, pollSeconds: 5, notify: false,
  },
};

/** The browser's own encoders, named so \`engineOf\` says "GPU" or "CPU". */
const GPU = "h264_browser";
const CPU = "h264_browsercpu";

const STORE = "sub20-lite-settings";

const files = new Map<string, File>();
const results = new Map<string, Blob>();
const urls = new Map<string, string>();
const listeners = new Set<(event: JobEvent) => void>();
const opened = new Set<(paths: string[]) => void>();
let next = 1;
let history: HistoryEntry[] = [];

function emit(event: JobEvent) {
  for (const listener of listeners) listener(event);
}

function register(file: File): string {
  const path = `lite-${next++}/${file.name}`;
  files.set(path, file);
  return path;
}

/** The video or result behind a made-up path. */
function blobAt(path: string): Blob {
  const blob = files.get(path) ?? results.get(path);
  if (!blob) throw new Error("That video isn't open any more. Add it again.");
  return blob;
}

function urlOf(path: string): string {
  let url = urls.get(path);
  if (!url) {
    url = URL.createObjectURL(blobAt(path));
    urls.set(path, url);
  }
  return url;
}

function loadSettings(): Settings {
  try {
    const saved = localStorage.getItem(STORE);
    if (saved) return { ...SETTINGS, ...JSON.parse(saved), watch: SETTINGS.watch };
  } catch {
    // Private windows may refuse storage; the defaults will do.
  }
  return structuredClone(SETTINGS);
}

/** Why Sub20 Lite can't run in this browser, or null when it can. */
async function blocked(): Promise<string | null> {
  if (!("VideoEncoder" in window) || !("VideoDecoder" in window)) {
    return "This browser can't encode video. Use a recent Chrome, Edge or Safari, or the Sub20 desktop app.";
  }
  const { supported } = await VideoEncoder.isConfigSupported({ codec: "avc1.640028", width: 1280, height: 720, bitrate: 2e6 })
    .catch(() => ({ supported: false }));
  return supported ? null : "This browser can't encode H.264. Use a recent Chrome, Edge or Safari, or the Sub20 desktop app.";
}

async function canvasUrl(canvas: HTMLCanvasElement | OffscreenCanvas): Promise<string> {
  const blob =
    "convertToBlob" in canvas
      ? await canvas.convertToBlob({ type: "image/jpeg", quality: 0.8 })
      : await new Promise<Blob>((resolve, reject) =>
          canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("no frame"))), "image/jpeg", 0.8));
  return URL.createObjectURL(blob);
}

/** Frames at the given times, drawn `height` pixels tall. */
async function* frames(path: string, times: number[], height: number) {
  const { ALL_FORMATS, BlobSource, CanvasSink, Input } = await media();
  const input = new Input({ source: new BlobSource(blobAt(path)), formats: ALL_FORMATS });
  try {
    const track = await input.getPrimaryVideoTrack();
    if (!track) return;
    const sink = new CanvasSink(track, { height, fit: "contain" });
    for await (const wrapped of sink.canvasesAtTimestamps(times)) {
      yield wrapped ? await canvasUrl(wrapped.canvas) : "";
    }
  } finally {
    input.dispose();
  }
}

// Jobs run one at a time: a browser has one GPU encoder to share.
interface Job {
  id: string;
  request: CompressRequest;
  stop: AbortController;
}
const queue: Job[] = [];
let running: Job | null = null;

function stem(name: string): string {
  return name.replace(/\.[^.]+$/, "");
}

async function run(job: Job) {
  const { id, request } = job;
  const file = files.get(request.path);
  if (!file) {
    emit({ type: "failed", id, error: "That video isn't open any more. Add it again." });
    return;
  }
  const began = performance.now();
  try {
    const { fit, probe } = await media();
    const planner = await loadPlanner();
    const source = await probe(file);
    const plan = planner.plan(source, request.trim, request.options, await gpuMostShortSide(source, outputFps(source, request.options)));
    emit({ type: "progress", id, phase: "analyzing", percent: 0, speed: null, etaSeconds: null, outBytes: 0 });
    let started = false;
    const fitted = await fit(file, source, plan, { bitrateMode: "constant", hardware: "no-preference", mixAudio: request.options.mixAudio }, {
      signal: job.stop.signal,
      onProgress: (_attempt, fraction) => {
        if (!started) {
          started = true;
          emit({ type: "started", id, encoder: GPU, hardware: true, videoKbps: plan.videoKbps, width: plan.width, height: plan.height });
        }
        const seconds = (performance.now() - began) / 1000;
        const eta = fraction > 0.02 ? (seconds / fraction) * (1 - fraction) : null;
        emit({ type: "progress", id, phase: "encoding", percent: fraction * 100, speed: null, etaSeconds: eta, outBytes: 0 });
      },
      onRetry: (attempt, overPercent, videoKbps) => emit({ type: "retry", id, attempt, overPercent, videoKbps }),
    });
    if (!fitted.under) {
      const over = ((fitted.blob.size / (plan.targetBytes ?? fitted.blob.size)) - 1) * 100;
      throw new Error(
        `Could not get under the size limit: still ${over.toFixed(1)}% over after ${fitted.attempts.length} tries, so nothing was saved. ` +
          `Pick a larger size, keep a shorter part, or use the Sub20 desktop app.`,
      );
    }
    const name = `${request.outputName?.trim() || `${stem(file.name)}-${request.options.targetMb}mb`}.mp4`;
    const output = `result-${id}/${name}`;
    results.set(output, fitted.blob);
    const last = fitted.attempts[fitted.attempts.length - 1];
    const result: JobResult = {
      output, sizeBytes: fitted.blob.size, originalBytes: file.size, duration: fitted.plan.duration,
      width: fitted.plan.width, height: fitted.plan.height, videoKbps: last.videoKbps, audioKbps: fitted.audio ? plan.audioKbps : 0,
      encoder: fitted.hardware ? GPU : CPU, hardware: fitted.hardware, fellBackFrom: null, attempts: fitted.attempts.length,
      elapsedSeconds: (performance.now() - began) / 1000, targetMb: request.options.targetMb, replacedOriginal: false,
    };
    for (const note of fitted.notes) emit({ type: "log", id, line: note });
    history = [{ id, finishedAt: Date.now() / 1000, input: request.path, trimmed: plan.trimmed, origin: "manual", result }, ...history];
    emit({ type: "done", id, result });
  } catch (error) {
    if (job.stop.signal.aborted) emit({ type: "canceled", id });
    else emit({ type: "failed", id, error: error instanceof Error ? error.message : String(error) });
  }
}

async function drain() {
  if (running) return;
  while (queue.length) {
    running = queue.shift()!;
    await run(running);
    running = null;
  }
}

function pick(accept: string, multiple: boolean): Promise<File[]> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.multiple = multiple;
    input.addEventListener("change", () => resolve([...(input.files ?? [])]));
    input.addEventListener("cancel", () => resolve([]));
    input.click();
  });
}

/** Files handed over by the system: an installed Lite opened from "Open with" or a share. */
function listenForLaunches() {
  const queueOfLaunches = (window as unknown as { launchQueue?: { setConsumer(f: (p: { files: FileSystemFileHandle[] }) => void): void } }).launchQueue;
  queueOfLaunches?.setConsumer(async ({ files: handles }) => {
    const paths = await Promise.all(handles.map(async (h) => register(await h.getFile())));
    if (paths.length) for (const handler of opened) handler(paths);
  });
}

/** Videos another app shared to Lite (Android's share sheet): the service
 *  worker keeps them in a cache until the page opens and takes them. */
async function takeShared(): Promise<string[]> {
  if (!new URLSearchParams(location.search).has("shared") || !("caches" in window)) return [];
  window.history.replaceState(null, "", location.pathname);
  const cache = await caches.open("sub20-shared");
  const paths: string[] = [];
  for (const request of await cache.keys()) {
    const response = await cache.match(request);
    if (response) {
      const name = decodeURIComponent(request.url.split("/").pop() ?? "shared.mp4");
      const blob = await response.blob();
      paths.push(register(new File([blob], name, { type: blob.type || "video/mp4" })));
    }
    await cache.delete(request);
  }
  return paths;
}

async function report(): Promise<Report> {
  const status = (codec: "h264" | "hevc" | "av1", name: string, works: boolean): EncoderStatus => ({
    name, label: "Browser encoder", codec, family: "cpu", familyLabel: "This browser", state: works ? "works" : "missing",
    detail: works ? null : "Sub20 Lite encodes H.264 only", device: null,
  });
  return {
    encoders: [status("h264", GPU, (await blocked()) === null), status("hevc", "hevc_browser", false), status("av1", "av1_browser", false)],
    ffmpegVersion: null, checkedAt: Date.now() / 1000, toneMap: false,
  };
}

const idle: WatchStatus = { running: false, folder: null, lastScan: 0, inProgress: 0, done: 0, failed: 0, problem: null, waiting: 0 };
const desktopOnly = () => Promise.reject(new Error("That needs the Sub20 desktop app."));

export const webBackend: Backend = {
  demo: false,
  frame: "native",
  can: LITE,
  minimize: async () => {},
  toggleMaximize: async () => {},
  closeWindow: async () => {},
  onMaximized: async () => () => {},
  bootstrap: async (): Promise<Bootstrap> => {
    listenForLaunches();
    // The built Lite works offline and can be installed; development doesn't
    // register, so Vite's own updates aren't cached.
    if (import.meta.env.PROD && "serviceWorker" in navigator) {
      navigator.serviceWorker.register("sw.js").catch(() => {});
    }
    const reason = await blocked();
    return {
      settings: loadSettings(),
      // Lite has no FFmpeg: "found" means this browser can do the work.
      ffmpeg: { found: reason === null, version: reason, folder: null },
      version,
      openedWith: await takeShared(),
      bundled: false,
      ffmpegNotice: null,
      ffmpegOffer: null,
    };
  },
  ffmpegStatus: async () => {
    const reason = await blocked();
    return { found: reason === null, version: reason, folder: null };
  },
  downloadFfmpeg: desktopOnly,
  // A browser hands over files, never folders.
  findVideos: async (paths) => ({ files: paths, capped: false }),
  addVideos: async (paths): Promise<Added[]> =>
    Promise.all(
      paths.map(async (path) => {
        const file = files.get(path);
        const name = file?.name ?? path.split("/").pop()!;
        if (!file) return { path, name, info: null, playUrl: null, error: "That video isn't open any more." };
        try {
          const info = await (await media()).probe(file);
          if (!info.hasVideo) return { path, name, info: null, playUrl: null, error: "This file has no video this browser can read." };
          // HDR is kept in the facts: the stage says Lite can't convert it
          // yet (`can.toneMap`). Extra sound tracks are mixed (`mix.ts`).
          return { path, name, info, playUrl: urlOf(path), error: null };
        } catch {
          return { path, name, info: null, playUrl: null, error: "This browser can't read this file." };
        }
      }),
    ),
  filmstrip: async (path, duration, count, onThumb) => {
    const times = Array.from({ length: count }, (_, i) => ((i + 0.5) / count) * duration);
    const strip: string[] = [];
    let index = 0;
    for await (const url of frames(path, times, 96)) {
      strip.push(url);
      onThumb(index++, url);
    }
    return strip;
  },
  videoUrl: (path) => (files.has(path) || results.has(path) ? urlOf(path) : null),
  previewFrame: async (path, at) => {
    for await (const url of frames(path, [at], 540)) return url;
    throw new Error("No frame there.");
  },
  // Planned within what the GPU encoder takes, so "1080p, good" is what will happen.
  estimate: async (info, trim, options) =>
    (await loadPlanner()).plan(info, trim, options, await gpuMostShortSide(info, outputFps(info, options))),
  compress: async (jobs) =>
    jobs.map((request) => {
      const id = `lite-job-${next++}`;
      queue.push({ id, request, stop: new AbortController() });
      emit({ type: "queued", id, input: request.path, origin: "manual" });
      void drain();
      return id;
    }),
  cancel: async (id) => {
    const index = queue.findIndex((j) => j.id === id);
    if (index >= 0) {
      queue.splice(index, 1);
      emit({ type: "canceled", id });
    } else if (running?.id === id) {
      running.stop.abort();
    }
  },
  cancelAll: async () => {
    for (const job of queue.splice(0)) emit({ type: "canceled", id: job.id });
    running?.stop.abort();
  },
  saveSettings: async (settings) => {
    try {
      localStorage.setItem(STORE, JSON.stringify(settings));
    } catch {
      // Not kept between visits; this session still has them.
    }
    return structuredClone(settings);
  },
  encoders: async () => report(),
  historyList: async () => history,
  historyRemove: async (id) => {
    history = history.filter((entry) => entry.id !== id);
  },
  historyClear: async () => {
    history = [];
  },
  watchStatus: async () => idle,
  watchDecide: async () => idle,
  clipFolders: async () => [],
  reveal: desktopOnly,
  openFile: async (path) => {
    window.open(urlOf(path), "_blank", "noopener");
  },
  resultUrl: async (path) => (results.has(path) ? urlOf(path) : null),
  copyFile: desktopOnly,
  renameResult: async (path, name) => {
    const blob = blobAt(path);
    const renamed = `${path.slice(0, path.lastIndexOf("/") + 1)}${name.replace(/\.mp4$/i, "")}.mp4`;
    results.delete(path);
    results.set(renamed, blob);
    const url = urls.get(path);
    if (url) {
      urls.delete(path);
      urls.set(renamed, url);
    }
    return renamed;
  },
  keyframes: async () => [],
  cutResult: desktopOnly,
  checkUpdate: async () => null,
  installUpdate: desktopOnly,
  copyDiagnostics: async (log) => {
    const lines = [`Sub20 Lite ${version}`, navigator.userAgent, "", ...log];
    await navigator.clipboard.writeText(lines.join("\n"));
  },
  dragOut: async () => {},
  download: async (path) => {
    const a = document.createElement("a");
    a.href = urlOf(path);
    a.download = path.split("/").pop()!;
    a.click();
  },
  share: async (path) => {
    const name = path.split("/").pop()!;
    const file = new File([blobAt(path)], name, { type: "video/mp4" });
    try {
      await navigator.share({ files: [file] });
    } catch (error) {
      // Closing the share sheet is not a failure.
      if (error instanceof DOMException && error.name === "AbortError") return;
      throw error;
    }
  },
  pickVideos: async () => (await pick("video/*,.mkv,.mov", true)).map(register),
  pickFolder: async () => null,
  onJob: async (handler) => {
    listeners.add(handler);
    return () => listeners.delete(handler);
  },
  onDrop: async (handler: (event: DropEvent) => void) => {
    const hasFiles = (event: DragEvent) => event.dataTransfer?.types.includes("Files") ?? false;
    const over = (event: DragEvent) => {
      if (!hasFiles(event)) return;
      event.preventDefault();
      handler({ type: "over" });
    };
    const leave = (event: DragEvent) => {
      if (event.relatedTarget === null) handler({ type: "leave" });
    };
    const drop = (event: DragEvent) => {
      if (!hasFiles(event)) return;
      event.preventDefault();
      handler({ type: "drop", paths: [...(event.dataTransfer?.files ?? [])].map(register) });
    };
    window.addEventListener("dragover", over);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("dragover", over);
      window.removeEventListener("dragleave", leave);
      window.removeEventListener("drop", drop);
    };
  },
  onOpened: async (handler) => {
    opened.add(handler);
    return () => opened.delete(handler);
  },
  // No folder watch, so no notices.
  onNotice: async () => () => {},
};
