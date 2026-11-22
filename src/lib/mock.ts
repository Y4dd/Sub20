// A stand-in backend for running the interface in a plain browser
// (`npm run dev`), with invented clips and simulated encodes. Nothing here
// ships behaviour: the desktop app always uses `tauri.ts`.

import type {
  Added, Backend, CompressRequest, EncodeOptions, EncoderStatus, FfmpegOffer, FfmpegStatus, Frame, HistoryEntry, JobEvent, NoticeClick,
  JobResult, MediaInfo, Plan, Quality, Report, Settings, Trim,
} from "./types";
import { DESKTOP } from "./types";

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
    existing: "newOnly", recursive: false, stableSeconds: 5, pollSeconds: 5, notify: true, compress: false,
    keepWatching: false,
  },
};

// The demo files live where Windows keeps videos: the site's screenshots show
// these paths, and the folder watch is Windows's.
const VIDEOS = String.raw`C:\Users\demo\Videos`;
/** A path in `folder`, the way Windows writes it. */
const inFolder = (folder: string, name: string) => `${folder}\\${name}`;

// `?watch` sets up a watch on a demo clip folder that finds a clip now and
// then and puts it on the bench; `?watch=compress` compresses it straight
// away instead, `?watch=pile` adds a pile that arrived while the app was
// closed, and `?watch=notice` clicks the notice for the first watched clip a
// second after it is ready (all three compressing). `?watch=bench-notice`
// clicks the notice for the first clip put on the bench, and
// `?watch=pile-notice` the one about a pile, as the tray's notices would.
const WATCH = new URLSearchParams(location.search).get("watch");
/** Who hears a click on a watch notice. */
const noticeHandlers = new Set<(notice: NoticeClick) => void>();
/** Who hears clips the watch put on the bench. */
const benchedHandlers = new Set<(paths: string[]) => void>();
let noticeClicked = false;
const watching = { inProgress: 0, done: 0, failed: 0, benched: 0, waiting: WATCH === "pile" || WATCH === "pile-notice" ? 12 : 0, paused: false };
if (WATCH !== null) {
  const compress = WATCH === "compress" || WATCH === "pile" || WATCH === "notice" || WATCH === "pile-notice";
  settings.watch = { ...settings.watch, enabled: true, folder: inFolder(VIDEOS, "NVIDIA"), recursive: true, compress };
}

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
/** Finished files, for cutting them. */
const made = new Map<string, { sizeBytes: number; duration: number }>();

/** The demo a source or a finished file was made from. */
function demoOf(path: string): Demo {
  return added.get(path) ?? DEMOS.find((d) => path.includes(d.name.replace(/\.[^.]+$/, "")))!;
}

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
  const end = Math.min(trim?.end ?? source.duration, source.duration);
  const duration = end - start;
  const trimmed = !!trim && !(start < 0.02 && source.duration - end < 0.02);
  const total = (options.targetMb * 8192) / duration;
  let audioKbps = options.audio === "none" ? 0 : options.audioKbps;
  const notes: string[] = [];
  if (audioKbps > total * 0.34) {
    // Opus in MP4 becomes AAC, whose floor is higher.
    const floor = options.audio === "opus" && options.container !== "mp4" ? 24 : 48;
    const step = [256, 192, 160, 128, 96, 64, 48, 32, 24].find((s) => s <= total * 0.34 && s < audioKbps) ?? 0;
    audioKbps = Math.min(audioKbps, Math.max(floor, step));
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

function simulate(id: string, job: CompressRequest, origin: "manual" | "watch" = "manual") {
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
  emit({ type: "queued", id, input: job.path, origin });
  if (origin === "watch") watching.inProgress++;
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
        output: inFolder(VIDEOS, `${name}.${job.options.container}`),
        sizeBytes: Math.round(plan.expectedBytes * 0.972), originalBytes: demo.info.sizeBytes, duration: plan.duration,
        width: plan.width, height: plan.height, videoKbps: plan.videoKbps, audioKbps: plan.audioKbps, encoder,
        hardware: encoder.endsWith("nvenc"), fellBackFrom: null, attempts: 1, elapsedSeconds: seconds,
        targetMb: job.options.targetMb, replacedOriginal: replace,
      };
      made.set(result.output, { sizeBytes: result.sizeBytes, duration: result.duration });
      history = [{ id, finishedAt: Date.now() / 1000, input: job.path, trimmed: plan.trimmed, origin, result }, ...history];
      if (origin === "watch") {
        watching.inProgress--;
        watching.done++;
      }
      emit({ type: "done", id, result });
      if (origin === "watch" && WATCH === "notice" && !noticeClicked) {
        noticeClicked = true;
        window.setTimeout(() => noticeHandlers.forEach((handler) => handler({ kind: "results", paths: [result.output] })), 1000);
      }
      return;
    }
    emit({
      type: "progress", id, phase: "encoding", percent: fraction * 100, speed: plan.duration / seconds,
      etaSeconds: seconds * (1 - fraction), outBytes: plan.expectedBytes * fraction,
    });
  }, 160);
  timers.set(id, timer);
}

/** The demo watch finds a clip a few seconds in, then one every 20 seconds,
 * the game clip first, as an NVIDIA folder would. */
function findClips() {
  let next = DEMOS.findIndex((demo) => demo.name.startsWith("clutch"));
  const find = () => {
    if (settings.watch.enabled && settings.watch.folder && !watching.waiting && !watching.paused) {
      const demo = DEMOS[next++ % DEMOS.length];
      const path = inFolder(settings.watch.folder, demo.name);
      added.set(path, demo);
      if (!settings.watch.compress) {
        watching.benched++;
        benchedHandlers.forEach((handler) => handler([path]));
        if (WATCH === "bench-notice" && !noticeClicked) {
          noticeClicked = true;
          window.setTimeout(() => noticeHandlers.forEach((handler) => handler({ kind: "bench", paths: [path] })), 1000);
        }
        window.setTimeout(find, 20000);
        return;
      }
      const profile = settings.profiles.find((p) => p.name === settings.watch.profile);
      const options = profile?.options ?? settings.lastOptions ?? DEFAULT_OPTIONS;
      simulate(`demo-${nextId++}`, { path, trim: null, options, outputName: null }, "watch");
    }
    window.setTimeout(find, 20000);
  };
  window.setTimeout(find, 4000);
}
if (WATCH !== null) findClips();

const DEMO_FFMPEG: FfmpegStatus = { found: true, version: "ffmpeg version 7.1 (demo)", folder: String.raw`C:\Users\demo\scoop\shims`, problem: null };
let noFfmpeg = new URLSearchParams(location.search).has("noffmpeg");
// `?noffmpeg=broken`: an FFmpeg is there but won't start.
const NO_FFMPEG: FfmpegStatus =
  new URLSearchParams(location.search).get("noffmpeg") === "broken"
    ? { found: false, version: null, folder: "C:\\Users\\you\\scoop\\shims", problem: "it stopped with exit code 0xC0000142" }
    : { found: false, version: null, folder: null, problem: null };
const DEMO_OFFER: FfmpegOffer = {
  version: "9.0.2",
  builder: "Sub20 (scripts/ffmpeg-build)",
  source: "https://github.com/Y4dd/Sub20/releases/tag/ffmpeg-9.0.2-1",
  files: [
    {
      name: "sub20-ffmpeg-9.0.2-1-x86_64-pc-windows-msvc.zip",
      url: "https://github.com/Y4dd/Sub20/releases/download/ffmpeg-9.0.2-1/sub20-ffmpeg-9.0.2-1-x86_64-pc-windows-msvc.zip",
      sha256: "0".repeat(64),
    },
  ],
};

// Windows by default; `?frame=native` shows Linux's frame, without the tray.
const linux = new URLSearchParams(location.search).get("frame") === "native";

export const mockBackend: Backend = {
  can: { ...DESKTOP, tray: !linux },
  demo: true,
  frame: ((): Frame => (linux ? "native" : "custom"))(),
  minimize: async () => {},
  toggleMaximize: async () => {},
  closeWindow: async () => {},
  onMaximized: async (handler) => {
    handler(false);
    return () => {};
  },
  bootstrap: async () => ({
    settings: structuredClone(settings),
    // `?noffmpeg` starts as a lean build on a computer without FFmpeg.
    ffmpeg: noFfmpeg ? NO_FFMPEG : DEMO_FFMPEG,
    version: "0.1.0",
    bundled: false,
    ffmpegNotice: null,
    ffmpegOffer: DEMO_OFFER,
    watched: [],
    notice: null,
    bench: [],
    // `?clips` opens the demo clips at startup, so a headless screenshot can
    // show the bench without clicking.
    openedWith: new URLSearchParams(location.search).has("clips")
      ? DEMOS.slice(0, 3).map((demo) => inFolder(VIDEOS, demo.name))
      : [],
  }),
  ffmpegStatus: async () => (noFfmpeg ? NO_FFMPEG : DEMO_FFMPEG),
  // `?noffmpeg=fail` makes the download fail halfway.
  downloadFfmpeg: async (onProgress) => {
    const size = 84 * 1048576;
    for (let step = 0; step <= 40; step++) {
      await new Promise((resolve) => setTimeout(resolve, 80));
      onProgress({ stage: "downloading", fraction: step / 40, received: (size * step) / 40 });
      if (step === 20 && new URLSearchParams(location.search).get("noffmpeg") === "fail") {
        throw "The FFmpeg download stopped: connection reset (demo). Try again.";
      }
    }
    onProgress({ stage: "checking", fraction: null, received: size });
    await new Promise((resolve) => setTimeout(resolve, 600));
    onProgress({ stage: "installing", fraction: null, received: size });
    await new Promise((resolve) => setTimeout(resolve, 600));
    noFfmpeg = false;
    return { ffmpeg: DEMO_FFMPEG, notice: String.raw`C:\Users\demo\AppData\Local\io.github.y4dd.sub20\ffmpeg\FFMPEG.txt` };
  },
  // A folder (a path with no extension) holds the demo clips.
  findVideos: async (paths) => ({
    files: paths.flatMap((path) => (/\.[a-z0-9]+$/i.test(path) ? [path] : DEMOS.map((demo) => inFolder(path, demo.name)))),
    capped: false,
  }),
  addVideos: async (paths): Promise<Added[]> => {
    await new Promise((resolve) => setTimeout(resolve, 250));
    return paths.map((path) => {
      const demo = DEMOS.find((d) => path.endsWith(d.name));
      if (!demo) return { path, name: path.split(/[\\/]/).pop()!, info: null, playUrl: null, error: "This file has no video track." };
      added.set(path, demo);
      return { path, name: demo.name, info: demo.info, playUrl: null, error: null };
    });
  },
  filmstrip: async (path, duration, count, onThumb) => {
    const demo = demoOf(path);
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
    const demo = demoOf(path);
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
    running: settings.watch.enabled && !!settings.watch.folder && !watching.paused, folder: settings.watch.folder,
    lastScan: Date.now() / 1000, ...watching,
    problem: settings.watch.enabled && !settings.watch.folder ? "Choose a folder that exists." : null,
  }),
  watchDecide: async (compress) => {
    const pile = watching.waiting;
    watching.waiting = 0;
    if (compress) {
      for (let i = 0; i < Math.min(pile, 3); i++) {
        const demo = DEMOS[i % DEMOS.length];
        const path = inFolder(settings.watch.folder!, demo.name);
        added.set(path, demo);
        const id = `demo-${nextId++}`;
        window.setTimeout(() => simulate(id, { path, trim: null, options: settings.lastOptions ?? DEFAULT_OPTIONS, outputName: null }, "watch"), i * 900);
      }
    }
    return mockBackend.watchStatus();
  },
  watchPause: async (paused) => {
    watching.paused = paused;
    return mockBackend.watchStatus();
  },
  clipFolders: async () => [
    { app: "NVIDIA app", path: inFolder(VIDEOS, "NVIDIA"), recursive: true },
    { app: "Xbox Game Bar", path: inFolder(VIDEOS, "Captures"), recursive: false },
  ],
  reveal: async () => {},
  openFile: async () => {},
  resultUrl: async () => null,
  copyFile: () => new Promise((resolve) => setTimeout(resolve, 120)),
  // As the engine: an MP4 can be cut anywhere (none listed); an MKV only on
  // its keyframes, every two seconds as the real results have them.
  keyframes: async (path) => {
    if (!path.toLowerCase().endsWith(".mkv")) return [];
    const duration = made.get(path)?.duration ?? 0;
    return Array.from({ length: Math.ceil(duration / 2) }, (_, i) => i * 2);
  },
  cutResult: async (path, start, end) => {
    await new Promise((resolve) => setTimeout(resolve, 700));
    const uncut = made.get(path);
    if (!uncut) throw "That file is no longer there.";
    const from = Math.floor(start / 2 + 0.0005) * 2;
    const duration = Math.min(end, uncut.duration) - from;
    const sizeBytes = Math.round((uncut.sizeBytes * duration) / uncut.duration);
    made.set(path, { sizeBytes, duration });
    return { output: path, sizeBytes, duration, start: from, note: null };
  },
  renameResult: async (path, name) => {
    const extension = path.slice(path.lastIndexOf("."));
    let clean = name.trim().replace(/[<>:"/\\|?*]/g, "");
    if (clean.toLowerCase().endsWith(extension.toLowerCase())) clean = clean.slice(0, -extension.length);
    if (!clean) throw "That name has no letters a file name can use.";
    const renamed = path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")) + 1) + clean + extension;
    const facts = made.get(path);
    if (facts) {
      made.delete(path);
      made.set(renamed, facts);
    }
    return renamed;
  },
  // `?update` pretends a newer release is out, to show how that looks.
  checkUpdate: async () =>
    new URLSearchParams(location.search).has("update") ? { version: "0.2.0", notes: "A demo update." } : null,
  installUpdate: () => new Promise((resolve) => setTimeout(() => (location.search = "", resolve()), 1500)),
  copyDiagnostics: async (log) => {
    const text = [
      "**Sub20** 0.1.0 (demo)",
      "**System** a browser",
      `**FFmpeg** ffmpeg version 7.1 (demo), in ${DEMO_FFMPEG.folder}`,
      ...(log.length ? ["", "**FFmpeg log of the last job**", "```", ...log, "```"] : []),
    ].join("\n");
    // A browser refuses the clipboard without focus; the demo shows the text instead.
    await navigator.clipboard.writeText(text).catch(() => console.info(text));
  },
  dragOut: async () => {},
  pickVideos: async () => {
    const fresh = DEMOS.filter((demo) => ![...added.values()].includes(demo));
    return (fresh.length ? fresh : DEMOS).slice(0, 3).map((demo) => inFolder(VIDEOS, demo.name));
  },
  pickFolder: async () => inFolder(VIDEOS, "Compressed"),
  onJob: async (handler) => {
    listeners.add(handler);
    return () => listeners.delete(handler);
  },
  onDrop: async () => () => {},
  onOpened: async () => () => {},
  onNotice: async (handler) => {
    noticeHandlers.add(handler);
    if (WATCH === "pile-notice") window.setTimeout(() => handler({ kind: "pile" }), 1000);
    return () => noticeHandlers.delete(handler);
  },
  onBenched: async (handler) => {
    benchedHandlers.add(handler);
    return () => benchedHandlers.delete(handler);
  },
  // The demo bench isn't kept between page loads.
  saveBench: async () => {},
};
