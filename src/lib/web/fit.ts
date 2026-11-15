// Sub20 Lite's encode path: trim, scale and encode a video in the browser
// with WebCodecs (through Mediabunny), then measure the result and encode
// again at a lower bitrate if it landed over the limit, the same rule as the
// desktop engine. Nothing leaves the machine.

import {
  ALL_FORMATS, BlobSource, BufferTarget, Conversion, Input, Mp4OutputFormat, Output, Quality, StreamTarget, canEncodeAudio,
  type AudioCodec, type VideoSample,
} from "mediabunny";
import type { MediaInfo, Plan } from "../types";
import { LADDER, gpuTakes, scaled } from "./gpu";
import { Mixer, RATE } from "./mix";
import { DISK_FROM_BYTES, newResult, resultsFolder } from "./store";
import { loadPlanner } from "./planner";

export interface FitSettings {
  bitrateMode: "variable" | "constant";
  hardware: "no-preference" | "prefer-hardware" | "prefer-software";
  /** Mix every sound track into one (`mix.ts`); off keeps the first. */
  mixAudio: boolean;
  /** Convert an HDR source (PQ or HLG) to ordinary colour. */
  toneMap: boolean;
}

/**
 * Draws each frame of an HDR video into an ordinary-colour (sRGB) canvas,
 * which makes the browser convert it on the way, and hands the canvas to
 * the encoder in its place. Mediabunny's resize already draws through a
 * canvas, so a scaled clip came out converted anyway; this does the same at
 * the source's size. Measured November 2026 on a generated clip with white
 * at 203 nits (as phones and games master it): saturation 80 against the
 * desktop's Mobius 97 and the original's 114 (untreated 25), right in hue
 * and about a fifth darker. The decoded frames are opaque GPU frames
 * (`format` null), so the desktop's own curve can't be run on their values.
 */
function toOrdinaryColour(width: number, height: number) {
  const canvas = new OffscreenCanvas(width, height);
  const context = canvas.getContext("2d", { colorSpace: "srgb", alpha: false })!;
  return (sample: VideoSample) => {
    sample.draw(context, 0, 0, width, height);
    return canvas;
  };
}

export interface Attempt {
  videoKbps: number;
  bytes: number;
  seconds: number;
}

export interface Fitted {
  /** The plan as encoded: smaller than asked when the GPU couldn't take the size. */
  plan: Plan;
  fps: number;
  attempts: Attempt[];
  /** The last attempt's file, whether or not it fits: in memory, or read from disk (`store.ts`). */
  blob: Blob;
  /** Deletes the file from disk when it was written there; call it for a result that isn't kept. */
  drop: () => Promise<void>;
  under: boolean;
  audio: AudioCodec | null;
  /** The browser's GPU encoder did the work, not its CPU one. */
  hardware: boolean;
  /** What was changed from the plan, or why it missed, in words. */
  notes: string[];
}

export interface FitEvents {
  /** Each attempt's progress, 0 to 1; attempts count from 0. */
  onProgress?: (attempt: number, fraction: number) => void;
  /** An attempt came out over and the next one starts at `videoKbps`. */
  onRetry?: (attempt: number, overPercent: number, videoKbps: number) => void;
  /** Stops the encode; `fit` then throws an `AbortError`. */
  signal?: AbortSignal;
}

/** A retry that shrinks the file by less than this means the encoder is at its floor (as in `crates/core/src/engine.rs`). */
const FLOOR_SHRINK = 0.95;

/**
 * The plan at the largest size the GPU can take, at the same frame rate.
 * The backend plans within what the GPU takes (`gpu.ts`), so this changes
 * nothing then; it is the safety net for a GPU that refuses the exact
 * bitrate, or a plan made without asking.
 */
async function onTheGpu(source: MediaInfo, plan: Plan, fps: number): Promise<{ plan: Plan; note: string | null } | null> {
  if (await gpuTakes(plan.width, plan.height, plan.videoKbps, fps)) return { plan, note: null };
  const asked = Math.min(plan.width, plan.height);
  for (const side of LADDER.filter((s) => s < asked)) {
    const { width, height } = scaled(source, side);
    if (await gpuTakes(width, height, plan.videoKbps, fps)) {
      return {
        plan: { ...plan, width, height, scaled: true },
        note: `Scaled to ${side}p: this GPU can't encode ${asked}p at ${Math.round(fps)} fps.`,
      };
    }
  }
  return null;
}

/** Reads what the planner needs from a file. */
export async function probe(file: File): Promise<MediaInfo> {
  const input = new Input({ source: new BlobSource(file), formats: ALL_FORMATS });
  try {
    const video = await input.getPrimaryVideoTrack();
    const audio = await input.getPrimaryAudioTrack();
    const duration = await input.computeDuration();
    const stats = video ? await video.computePacketStats(240) : null;
    // A plain string: TypeScript's DOM types list only the SDR transfer names.
    const transfer: string | null = video ? ((await video.getColorSpace()).transfer ?? null) : null;
    return {
      duration,
      sizeBytes: file.size,
      hasVideo: !!video,
      hasAudio: !!audio,
      width: video?.codedWidth ?? 0,
      height: video?.codedHeight ?? 0,
      displayWidth: video?.displayWidth ?? 0,
      displayHeight: video?.displayHeight ?? 0,
      rotation: video?.rotation ?? 0,
      fps: stats ? Math.round(stats.averagePacketRate * 100) / 100 : null,
      videoCodec: video?.codec ?? null,
      pixelFormat: null,
      videoKbps: stats ? Math.round(stats.averageBitrate / 1000) : null,
      audioCodec: audio?.codec ?? null,
      audioKbps: null,
      audioChannels: audio?.numberOfChannels ?? null,
      audioTracks: (await input.getAudioTracks()).length,
      hdr: transfer === "pq" ? "pq" : transfer === "hlg" ? "hlg" : null,
    };
  } finally {
    input.dispose();
  }
}

/** AAC where this browser can encode it (Discord plays it everywhere), else Opus. */
async function audioCodec(kbps: number): Promise<AudioCodec | null> {
  for (const codec of ["aac", "opus"] as const) {
    if (await canEncodeAudio(codec, { bitrate: kbps * 1000 })) return codec;
  }
  return null;
}

export async function fit(
  file: File,
  source: MediaInfo,
  plan: Plan,
  settings: FitSettings,
  events: FitEvents = {},
): Promise<Fitted> {
  const { onProgress, onRetry, signal } = events;
  const planner = await loadPlanner();
  const target = plan.targetBytes ?? Infinity;
  const audio = source.hasAudio && plan.audio ? await audioCodec(plan.audioKbps) : null;
  const fps = plan.fpsCap ?? source.fps ?? 30;
  const notes: string[] = [];
  let hardware = settings.hardware;
  if (hardware !== "prefer-software") {
    const gpu = await onTheGpu(source, plan, fps);
    if (gpu) {
      plan = gpu.plan;
      if (gpu.note) notes.push(gpu.note);
      hardware = "prefer-hardware";
    } else {
      notes.push("No GPU encoder for this video here; the CPU encoder may not reach low limits.");
    }
  }

  const attempts: Attempt[] = [];
  let videoKbps = plan.videoKbps;
  let made: Made | null = null;
  // Large results go to disk, so a 1 GB limit doesn't hold a 1 GB file in memory.
  const folder = (plan.targetBytes ?? plan.expectedBytes) >= DISK_FROM_BYTES ? await resultsFolder() : null;
  for (let n = 0; n <= planner.sizeRetries; n++) {
    const began = performance.now();
    const over = made;
    made = await encode(file, source, plan, fps, videoKbps, audio, { ...settings, hardware }, folder, (f) => onProgress?.(n, f), signal);
    // An attempt that came out over is never kept.
    await over?.drop();
    const blob = made.blob;
    attempts.push({ videoKbps, bytes: blob.size, seconds: (performance.now() - began) / 1000 });
    if (blob.size <= target) break;
    const previous = attempts[attempts.length - 2];
    if (previous && blob.size > previous.bytes * FLOOR_SHRINK) {
      notes.push(`The encoder won't go below ${Math.round((blob.size * 8) / 1000 / plan.duration)} kbps here.`);
      break;
    }
    if (n === planner.sizeRetries) break;
    videoKbps = planner.lowerBitrate(videoKbps, blob.size, target, plan.duration);
    onRetry?.(n + 1, (blob.size / target - 1) * 100, videoKbps);
  }

  const gpu = hardware === "prefer-hardware";
  const { blob, drop } = made!;
  return { plan, fps, attempts, blob, drop, under: blob.size <= target, audio, hardware: gpu, notes };
}

/** One attempt's file, and how to delete it. */
interface Made {
  blob: Blob;
  drop: () => Promise<void>;
}

const keep = async () => {};

async function encode(
  file: File,
  source: MediaInfo,
  plan: Plan,
  fps: number,
  videoKbps: number,
  audio: AudioCodec | null,
  settings: FitSettings,
  folder: FileSystemDirectoryHandle | null,
  onProgress: (fraction: number) => void,
  signal?: AbortSignal,
): Promise<Made> {
  signal?.throwIfAborted();
  const input = new Input({ source: new BlobSource(file), formats: ALL_FORMATS });
  // On disk, written as it is made, with the index at the end (`fastStart: false`)
  // so nothing is held back in memory; in memory, with the index at the front.
  const disk = folder ? await newResult(folder) : null;
  const output = disk
    ? new Output({ format: new Mp4OutputFormat({ fastStart: false }), target: new StreamTarget(disk.writable, { chunked: true }) })
    : new Output({ format: new Mp4OutputFormat({ fastStart: "in-memory" }), target: new BufferTarget() });
  let mixer: Mixer | null = null;
  let done = false;
  try {
    if (audio && settings.mixAudio) {
      const primary = await input.getPrimaryAudioTrack();
      const others = (await input.getAudioTracks()).filter((track) => track.id !== primary?.id);
      const start = plan.trimmed ? plan.start : 0;
      if (others.length) mixer = new Mixer(others, start, start + plan.duration);
    }
    const width = plan.scaled ? plan.width : source.displayWidth;
    const height = plan.scaled ? plan.height : source.displayHeight;
    const colour = settings.toneMap && source.hdr && width && height
      ? { process: toOrdinaryColour(width, height), processedWidth: width, processedHeight: height }
      : {};
    const conversion = await Conversion.init({
      input,
      output,
      tracks: "primary",
      trim: plan.trimmed ? { start: plan.start, end: plan.start + plan.duration } : undefined,
      video: {
        codec: "avc",
        forceTranscode: true,
        ...(plan.scaled ? { width: plan.width, height: plan.height, fit: "fill" as const } : {}),
        // Always set: without it the encoder is told no frame rate, assumes
        // about 30, and a 60 fps clip comes out near twice the size.
        frameRate: fps,
        quality: new Quality({ bitrate: videoKbps * 1000, bitrateMode: settings.bitrateMode }),
        hardwareAcceleration: settings.hardware,
        ...colour,
      },
      audio: audio
        ? {
            codec: audio,
            forceTranscode: true,
            quality: new Quality({ bitrate: plan.audioKbps * 1000 }),
            ...(mixer
              ? { numberOfChannels: 2, sampleRate: RATE, process: mixer.process, processedNumberOfChannels: 2, processedSampleRate: RATE }
              : {}),
          }
        : { discard: true },
      showWarnings: false,
    });
    if (!conversion.isValid) {
      const reasons = conversion.discardedTracks.map((d) => `${d.track.type}: ${d.reason}`).join(", ");
      throw new Error(`This browser can't convert this file (${reasons}).`);
    }
    conversion.onProgress = onProgress;
    const stop = () => void conversion.cancel();
    signal?.addEventListener("abort", stop, { once: true });
    try {
      await conversion.execute();
    } finally {
      signal?.removeEventListener("abort", stop);
    }
    signal?.throwIfAborted();
    done = true;
    if (disk) return { blob: await disk.read(), drop: disk.drop };
    return { blob: new Blob([(output.target as BufferTarget).buffer!], { type: "video/mp4" }), drop: keep };
  } finally {
    await mixer?.close().catch(() => {});
    input.dispose();
    // A canceled or failed attempt leaves no file behind.
    if (disk && !done) {
      await disk.writable.abort().catch(() => {});
      await disk.drop();
    }
  }
}

