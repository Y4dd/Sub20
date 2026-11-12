// A stand-in backend for running the interface in a plain browser
// (`npm run dev`), with invented clips and simulated encodes. Nothing here
// ships behaviour: the desktop app always uses `tauri.ts`.

import type {
  Added, Backend, CompressRequest, EncodeOptions, EncoderStatus, Frame, HistoryEntry, JobEvent, JobResult, MediaInfo,
  Plan, Quality, Report, Settings, Trim,
} from "./types";

export const DEFAULT_OPTIONS: EncodeOptions = {
  targetMb: 20, videoKbps: null, codec: "h264", encoder: null, audio: "aac", audioKbps: 128, effort: "p4",
  tune: "hq", container: "mp4", autoResolution: true, minAutoHeight: 240, maxShortSide: null, maxWidth: null,
  maxHeight: null, maxFps: null, audioOnly: false, fastFinalize: false, hardwareDecode: false, mixAudio: true,
};

let settings: Settings = {
  profiles: [
    { name: "Everyday", options: { ...DEFAULT_OPTIONS } },
    { name: "Sharper (HEVC)", options: { ...DEFAULT_OPTIONS, codec: "hevc", effort: "p5" } },
    { name: "Smallest (AV1)", options: { ...DEFAULT_OPTIONS, codec: "av1", effort: "p5", container: "mkv", audio: "opus" } },
  ],
  defaultProfile: "Everyday",
  sizes: [20, 50, 500, 1024],
  disabledEncoders: ["libaom-av1"],
  outputMode: "sameFolder",
  outputFolder: null,
  historyEnabled: true,
  historyKeepDays: 0,
  parallelJobs: 0,
  ffmpegFolder: null,
  checkUpdates: true,
  lastOptions: null,
  watch: {
    enabled: false, folder: null, profile: null, outputMode: "sameFolder", outputFolder: null, originals: "keep",
    existing: "newOnly", recursive: false, stableSeconds: 5, pollSeconds: 5,
  },
};

interface Demo {
  name: string;
  info: MediaInfo;
  /** Hues the invented footage is painted in. */
  hue: number;
}

function info(width: number, height: number, duration: number, fps: number, mb: number, codec: string): MediaInfo {
  return {
    duration, sizeBytes: mb * 1024 * 1024, hasVideo: true, hasAudio: true, width, height, displayWidth: width,
    displayHeight: height, rotation: 0, fps, videoCodec: codec, pixelFormat: "yuv420p",
    videoKbps: (mb * 8192) / duration - 160, audioCodec: "aac", audioKbps: 160, audioChannels: 2, audioTracks: 1,
    hdr: null,
  };
}

const DEMOS: Demo[] = [
  // Drones and phones record HDR; it is tone-mapped on the way out.
  { name: "harbour-drone-take3.mov", info: { ...info(3840, 2160, 72.4, 60, 612, "hevc"), pixelFormat: "yuv420p10le", hdr: "hlg" }, hue: 205 },
  // Game and microphone on separate tracks, as OBS records them.
  { name: "clutch-1v4-ranked.mp4", info: { ...info(2560, 1440, 41.0, 60, 248, "h264"), audioTracks: 2 }, hue: 280 },
  { name: "birthday-candles.mp4", info: info(1080, 1920, 18.6, 30, 46.2, "h264"), hue: 28 },
  { name: "standup-recording-nov.mkv", info: info(1920, 1080, 1840, 30, 1320, "h264"), hue: 150 },
];

const added = new Map<string, Demo>();

/** Paints a frame of invented footage: a sky, a sun that crosses it, hills. */
function paint(demo: Demo, at: number, width: number, height: number): string {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const g = canvas.getContext("2d")!;
  const t = at / demo.info.duration;
  const sky = g.createLinearGradient(0, 0, 0, height);
  sky.addColorStop(0, `hsl(${demo.hue + t * 40} 55% ${18 + t * 14}%)`);
  sky.addColorStop(1, `hsl(${demo.hue - 30 + t * 60} 70% ${46 + t * 10}%)`);
  g.fillStyle = sky;
  g.fillRect(0, 0, width, height);
  g.fillStyle = `hsl(${demo.hue + 150} 90% 82% / 0.9)`;
  g.beginPath();
  g.arc(width * (0.12 + t * 0.76), height * (0.62 - Math.sin(t * Math.PI) * 0.36), height * 0.09, 0, Math.PI * 2);
  g.fill();
  for (let layer = 0; layer < 3; layer++) {
    g.fillStyle = `hsl(${demo.hue - 20} 40% ${20 - layer * 6}%)`;
    g.beginPath();
    g.moveTo(0, height);
    for (let x = 0; x <= width; x += width / 24) {
      const wave = Math.sin(x / width * (5 + layer * 3) + layer * 2 + t * (2 + layer)) * 0.07;
      g.lineTo(x, height * (0.66 + layer * 0.1 + wave));
    }
    g.lineTo(width, height);
    g.fill();
  }
  return canvas.toDataURL("image/jpeg", 0.8);
}

const LADDER = [2160, 1440, 1080, 720, 480, 360, 240];

/**
 * A trimmed-down copy of the Rust planner, enough to drive the interface.
 * The Lite size test (`src/lab`) borrows it until the planner is built to
 * WebAssembly.
 */
/** A length in words, as `timecode::spoken` says it in Rust. */
function spoken(seconds: number): string {
  const unit = (n: number, one: string) => (n === 1 ? `1 ${one}` : `${n} ${one}s`);
  if (!(seconds > 0)) return "0 seconds";
  if (seconds < 90) return unit(Math.round(seconds), "second");
  const minutes = Math.round(seconds / 60);
  const [h, m] = [Math.floor(minutes / 60), minutes % 60];
  if (h === 0) return unit(m, "minute");
  return m === 0 ? unit(h, "hour") : `${unit(h, "hour")} ${unit(m, "minute")}`;
}

export function planFor(source: MediaInfo, trim: Trim | null, options: EncodeOptions): Plan {
  const start = trim?.start ?? 0;
  const duration = (trim?.end ?? source.duration) - start;
  const trimmed = !!trim && !(start < 0.02 && source.duration - trim.end < 0.02);
  const total = (options.targetMb * 8192) / duration;
  let audioKbps = options.audio === "none" ? 0 : options.audioKbps;
  const notes: string[] = [];
  if (audioKbps > total * 0.34) {
    audioKbps = [256, 192, 160, 128, 96, 64, 48, 32, 24].find((s) => s <= total * 0.34) ?? 48;
  }
  const video = options.videoKbps ?? total - audioKbps;
  if (video < 24) {
    const fits = (options.targetMb * 8192) / (24 + audioKbps);
    const most = fits >= 90 ? Math.floor(fits / 60) * 60 : Math.floor(fits);
    throw `${options.targetMb} MB is too small for ${spoken(duration)} of video: it holds ${spoken(most)} at most. Trim it shorter or raise the size.`;
  }
  let [width, height] = [source.displayWidth, source.displayHeight];
  const short = Math.min(width, height);
  const fps = options.maxFps && source.fps && source.fps > options.maxFps ? options.maxFps : source.fps ?? 30;
  const kbps = Math.round(video);
  // Same rule as plan.rs: weighted density, a cap for pictures shrunk under
  // 720p, and the size with the best verdict (good is good enough).
  const RANK: Quality[] = ["sharp", "good", "soft", "rough"];
  const rate = (side: number): Quality => {
    const efficiency = { h264: 1, hevc: 1.5, av1: 1.8 }[options.codec];
    const megapixels = ((width * side) / short) * ((height * side) / short) / 1e6;
    const density = (kbps * efficiency) / (megapixels * Math.sqrt(Math.min(2, Math.max(0.8, fps / 30))));
    const byBits = density >= 1400 ? 0 : density >= 550 ? 1 : density >= 220 ? 2 : 3;
    const bySize = side === short ? 0 : side < 480 ? 3 : side < 720 ? 2 : 0;
    return RANK[Math.max(byBits, bySize)];
  };
  let side = short;
  if (options.autoResolution) {
    const sizes = [short, ...LADDER.filter((r) => r < short && r >= Math.min(options.minAutoHeight, short))];
    const score = (s: number) => Math.max(1, RANK.indexOf(rate(s)));
    const best = Math.min(...sizes.map(score));
    const matching = sizes.filter((s) => score(s) === best);
    side = best === 3 ? matching[matching.length - 1] : matching[0];
  } else if (options.maxShortSide && options.maxShortSide < short) {
    side = options.maxShortSide;
  }
  const quality = rate(side);
  if (side < short) {
    const even = (v: number) => Math.max(2, Math.floor(v / 2) * 2);
    [width, height] = [even((width * side) / short), even((height * side) / short)];
  }
  return {
    start, duration, trimmed, targetBytes: options.videoKbps ? null : options.targetMb * 1024 * 1024,
    videoKbps: Math.round(video), audio: options.audio === "none" ? null : options.container === "mp4" ? "aac" : "opus",
    audioKbps, width, height, scaled: side < short,
    fpsCap: fps !== source.fps ? fps : null, quality,
    expectedBytes: options.targetMb * 1024 * 1024, notes,
  };
}

const ENCODERS: EncoderStatus[] = (
  [
    ["h264_nvenc", "H.264, NVIDIA NVENC", "h264", "nvenc", "NVIDIA NVENC", "works"],
    ["hevc_nvenc", "HEVC, NVIDIA NVENC", "hevc", "nvenc", "NVIDIA NVENC", "works"],
    ["av1_nvenc", "AV1, NVIDIA NVENC", "av1", "nvenc", "NVIDIA NVENC", "failed"],
    ["h264_qsv", "H.264, Intel Quick Sync", "h264", "qsv", "Intel Quick Sync", "failed"],
    ["hevc_qsv", "HEVC, Intel Quick Sync", "hevc", "qsv", "Intel Quick Sync", "failed"],
    ["av1_qsv", "AV1, Intel Quick Sync", "av1", "qsv", "Intel Quick Sync", "missing"],
    ["libx264", "H.264, CPU", "h264", "cpu", "CPU", "works"],
    ["libx265", "HEVC, CPU", "hevc", "cpu", "CPU", "works"],
    ["libsvtav1", "AV1, CPU (SVT-AV1)", "av1", "cpu", "CPU", "works"],
    ["libaom-av1", "AV1, CPU (libaom)", "av1", "cpu", "CPU", "works"],
  ] as const
).map(([name, label, codec, family, familyLabel, state]) => ({
  name, label, codec, family, familyLabel, state,
  detail: state === "failed"
    ? name === "av1_nvenc" ? "This GPU does not support AV1 encoding (needs RTX 40 series or newer)." : "No Intel graphics device was found."
    : null,
  device: null,
}));

let history: HistoryEntry[] = [];
const listeners = new Set<(event: JobEvent) => void>();
const timers = new Map<string, number>();
let nextId = 1;

function emit(event: JobEvent) {
  for (const listener of listeners) listener(event);
}

function simulate(id: string, job: CompressRequest) {
  const demo = added.get(job.path)!;
  let plan: Plan;
  try {
    plan = planFor(demo.info, job.trim, job.options);
  } catch (error) {
    window.setTimeout(() => emit({ type: "failed", id, error: String(error) }), 300);
    return;
  }
  const encoder = job.options.codec === "av1" ? "libsvtav1" : `${job.options.codec}_nvenc`;
  const seconds = Math.min(14, 3 + plan.duration / 8);
  const begun = performance.now() + 500;
  emit({ type: "queued", id, input: job.path, origin: "manual" });
  window.setTimeout(() => {
    emit({ type: "started", id, encoder, hardware: encoder.endsWith("nvenc"), videoKbps: plan.videoKbps, width: plan.width, height: plan.height });
    emit({ type: "log", id, line: `ffmpeg -y -i "${demo.name}" -c:v ${encoder} -b:v ${plan.videoKbps}k …` });
  }, 450);
  const timer = window.setInterval(() => {
    const fraction = (performance.now() - begun) / (seconds * 1000);
    if (fraction < 0) return;
    if (fraction >= 1) {
      window.clearInterval(timer);
      timers.delete(id);
      const stem = demo.name.replace(/\.[^.]+$/, "");
      const replace = settings.outputMode === "replace";
      const name = job.outputName?.trim() || (replace ? stem : `${stem}-${job.options.targetMb}mb`);
      const result: JobResult = {
        output: `/Users/demo/Videos/${name}.${job.options.container}`,
        sizeBytes: Math.round(plan.expectedBytes * 0.972), originalBytes: demo.info.sizeBytes, duration: plan.duration,
        width: plan.width, height: plan.height, videoKbps: plan.videoKbps, audioKbps: plan.audioKbps, encoder,
        hardware: encoder.endsWith("nvenc"), fellBackFrom: null, attempts: 1, elapsedSeconds: seconds,
        targetMb: job.options.targetMb, replacedOriginal: replace,
      };
      history = [{ id, finishedAt: Date.now() / 1000, input: job.path, trimmed: plan.trimmed, origin: "manual", result }, ...history];
      emit({ type: "done", id, result });
      return;
    }
    emit({
      type: "progress", id, phase: "encoding", percent: fraction * 100, speed: plan.duration / seconds,
      etaSeconds: seconds * (1 - fraction), outBytes: plan.expectedBytes * fraction,
    });
  }, 160);
  timers.set(id, timer);
}

export const mockBackend: Backend = {
  demo: true,
  // The Windows frame by default; `?frame=overlay` or `?frame=native` shows the others.
  frame: ((): Frame => {
    const asked = new URLSearchParams(location.search).get("frame");
    return asked === "overlay" || asked === "native" ? asked : "custom";
  })(),
  minimize: async () => {},
  toggleMaximize: async () => {},
  closeWindow: async () => {},
  onMaximized: async (handler) => {
    handler(false);
    return () => {};
  },
  bootstrap: async () => ({
    settings: structuredClone(settings),
    ffmpeg: { found: true, version: "ffmpeg version 7.1 (demo)", folder: "/usr/local/bin" },
    version: "0.1.0",
    bundled: false,
    ffmpegNotice: null,
    // `?clips` opens the demo clips at startup, so a headless screenshot can
    // show the bench without clicking.
    openedWith: new URLSearchParams(location.search).has("clips")
      ? DEMOS.slice(0, 3).map((demo) => `/Users/demo/Videos/${demo.name}`)
      : [],
  }),
  ffmpegStatus: async () => ({ found: true, version: "ffmpeg version 7.1 (demo)", folder: "/usr/local/bin" }),
  addVideos: async (paths): Promise<Added[]> => {
    await new Promise((resolve) => setTimeout(resolve, 250));
    return paths.map((path) => {
      const demo = DEMOS.find((d) => path.endsWith(d.name));
      if (!demo) return { path, name: path.split("/").pop()!, info: null, playUrl: null, error: "This file has no video track." };
      added.set(path, demo);
      return { path, name: demo.name, info: demo.info, playUrl: null, error: null };
    });
  },
  filmstrip: async (path, duration, count, onThumb) => {
    const demo = added.get(path)!;
    const ratio = demo.info.displayWidth / demo.info.displayHeight;
    // Arrive one at a time, as the real ones do.
    const strip: string[] = [];
    for (let i = 0; i < count; i++) {
      await new Promise((resolve) => setTimeout(resolve, 40));
      strip.push(paint(demo, ((i + 0.5) / count) * duration, Math.round(96 * ratio), 96));
      onThumb(i, strip[i]);
    }
    return strip;
  },
  videoUrl: () => null,
  previewFrame: async (path, at) => {
    // A finished file is painted from the clip it was made from.
    const demo = added.get(path) ?? DEMOS.find((d) => path.includes(d.name.replace(/\.[^.]+$/, "")))!;
    const ratio = demo.info.displayWidth / demo.info.displayHeight;
    return paint(demo, at, Math.round(540 * ratio), 540);
  },
  estimate: async (source, trim, options) => planFor(source, trim, options),
  compress: async (jobs) =>
    jobs.map((job, index) => {
      const id = `demo-${nextId++}`;
      // Stagger the starts so the queue reads as a queue.
      window.setTimeout(() => simulate(id, job), index * 900);
      return id;
    }),
  cancel: async (id) => {
    window.clearInterval(timers.get(id));
    timers.delete(id);
    emit({ type: "canceled", id });
  },
  cancelAll: async () => {
    for (const [id, timer] of timers) {
      window.clearInterval(timer);
      emit({ type: "canceled", id });
    }
    timers.clear();
  },
  saveSettings: async (next) => {
    settings = structuredClone(next);
    return structuredClone(settings);
  },
  encoders: async (refresh): Promise<Report> => {
    if (refresh) await new Promise((resolve) => setTimeout(resolve, 900));
    return { encoders: ENCODERS, ffmpegVersion: "ffmpeg version 7.1 (demo)", checkedAt: Date.now() / 1000, toneMap: true };
  },
  historyList: async () => history,
  historyRemove: async (id) => {
    history = history.filter((entry) => entry.id !== id);
  },
  historyClear: async () => {
    history = [];
  },
  watchStatus: async () => ({
    running: settings.watch.enabled && !!settings.watch.folder, folder: settings.watch.folder,
    lastScan: Date.now() / 1000, inProgress: 0, done: 0, failed: 0,
    problem: settings.watch.enabled && !settings.watch.folder ? "Choose a folder that exists." : null,
  }),
  clipFolders: async () => [
    { app: "NVIDIA app", path: "/Users/demo/Videos/NVIDIA", recursive: true },
    { app: "Xbox Game Bar", path: "/Users/demo/Videos/Captures", recursive: false },
  ],
  reveal: async () => {},
  openFile: async () => {},
  resultUrl: async () => null,
  copyFile: () => new Promise((resolve) => setTimeout(resolve, 120)),
  renameResult: async (path, name) => {
    const extension = path.slice(path.lastIndexOf("."));
    let clean = name.trim().replace(/[<>:"/\\|?*]/g, "");
    if (clean.toLowerCase().endsWith(extension.toLowerCase())) clean = clean.slice(0, -extension.length);
    if (!clean) throw "That name has no letters a file name can use.";
    return path.slice(0, path.lastIndexOf("/") + 1) + clean + extension;
  },
  // `?update` pretends a newer release is out, to show how that looks.
  checkUpdate: async () =>
    new URLSearchParams(location.search).has("update") ? { version: "0.2.0", notes: "A demo update." } : null,
  installUpdate: () => new Promise((resolve) => setTimeout(() => (location.search = "", resolve()), 1500)),
  copyDiagnostics: async (log) => {
    const text = [
      "**Sub20** 0.1.0 (demo)",
      "**System** a browser",
      "**FFmpeg** ffmpeg version 7.1 (demo), in /usr/local/bin",
      ...(log.length ? ["", "**FFmpeg log of the last job**", "```", ...log, "```"] : []),
    ].join("\n");
    // A browser refuses the clipboard without focus; the demo shows the text instead.
    await navigator.clipboard.writeText(text).catch(() => console.info(text));
  },
  dragOut: async () => {},
  pickVideos: async () => {
    const fresh = DEMOS.filter((demo) => ![...added.values()].includes(demo));
    return (fresh.length ? fresh : DEMOS).slice(0, 3).map((demo) => `/Users/demo/Videos/${demo.name}`);
  },
  pickFolder: async () => "/Users/demo/Videos/Compressed",
  onJob: async (handler) => {
    listeners.add(handler);
    return () => listeners.delete(handler);
  },
  onDrop: async () => () => {},
};
