// Sub20 Lite's encode path: trim, scale and encode a video in the browser
// with WebCodecs (through Mediabunny), then measure the result and encode
// again at a lower bitrate if it landed over the limit, the same rule as the
// desktop engine. Nothing leaves the machine.

import {
  ALL_FORMATS, BlobSource, BufferTarget, Conversion, Input, Mp4OutputFormat, Output, Quality, canEncodeAudio,
  type AudioCodec,
} from "mediabunny";
import type { MediaInfo, Plan } from "../types";

/** Matches `SIZE_RETRIES` in `crates/core/src/engine.rs`. */
export const SIZE_RETRIES = 2;

export interface FitSettings {
  bitrateMode: "variable" | "constant";
  hardware: "no-preference" | "prefer-hardware" | "prefer-software";
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
  /** The last attempt's file, whether or not it fits. */
  blob: Blob;
  under: boolean;
  audio: AudioCodec | null;
  /** What was changed from the plan, or why it missed, in words. */
  notes: string[];
}

const LADDER = [2160, 1440, 1080, 720, 480, 360, 240];

/** A retry that shrinks the file by less than this means the encoder is at its floor (as in engine.rs). */
const FLOOR_SHRINK = 0.95;

/**
 * Whether this browser's GPU can encode H.264 at this size and frame rate.
 * Asked of WebCodecs directly: Mediabunny's `canEncodeVideo` leaves the frame
 * rate out, and GPUs that take 1440p30 can refuse 1440p60. When the GPU says
 * no, the browser quietly uses its software encoder, which ignores low
 * bitrates.
 */
async function gpuTakes(width: number, height: number, kbps: number, fps: number): Promise<boolean> {
  if (!("VideoEncoder" in window)) return false;
  try {
    const { supported } = await VideoEncoder.isConfigSupported({
      codec: "avc1.640034", width, height, bitrate: kbps * 1000, framerate: fps, hardwareAcceleration: "prefer-hardware",
    });
    return supported ?? false;
  } catch {
    return false;
  }
}

/** The plan at the largest size the GPU can take, at the same frame rate. */
async function onTheGpu(source: MediaInfo, plan: Plan, fps: number): Promise<{ plan: Plan; note: string | null } | null> {
  if (await gpuTakes(plan.width, plan.height, plan.videoKbps, fps)) return { plan, note: null };
  const short = Math.min(source.displayWidth, source.displayHeight);
  const asked = Math.min(plan.width, plan.height);
  const even = (v: number) => Math.max(2, Math.floor(v / 2) * 2);
  for (const side of LADDER.filter((s) => s < asked)) {
    const width = even((source.displayWidth * side) / short);
    const height = even((source.displayHeight * side) / short);
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

/**
 * The next bitrate after a result came out `size` bytes against `target`.
 * A copy of `lower_bitrate` in `crates/core/src/engine.rs`.
 */
export function lowerBitrate(videoKbps: number, size: number, target: number, duration: number): number {
  const proportional = videoKbps * (target / size) * 0.97;
  const overKbps = ((size - target) * 8) / 1000 / Math.max(duration, 0.1);
  const subtractive = videoKbps - overKbps * 1.25;
  const lower = subtractive > proportional * 0.5 ? Math.min(proportional, subtractive) : proportional;
  return Math.max(16, Math.floor(lower));
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
  onProgress?: (attempt: number, fraction: number) => void,
): Promise<Fitted> {
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
  let blob: Blob | null = null;
  for (let n = 0; n <= SIZE_RETRIES; n++) {
    const began = performance.now();
    blob = await encode(file, plan, fps, videoKbps, audio, { ...settings, hardware }, (f) => onProgress?.(n, f));
    attempts.push({ videoKbps, bytes: blob.size, seconds: (performance.now() - began) / 1000 });
    if (blob.size <= target) break;
    const previous = attempts[attempts.length - 2];
    if (previous && blob.size > previous.bytes * FLOOR_SHRINK) {
      notes.push(`The encoder won't go below ${Math.round((blob.size * 8) / 1000 / plan.duration)} kbps here.`);
      break;
    }
    videoKbps = lowerBitrate(videoKbps, blob.size, target, plan.duration);
  }

  return { plan, fps, attempts, blob: blob!, under: blob!.size <= target, audio, notes };
}

async function encode(
  file: File,
  plan: Plan,
  fps: number,
  videoKbps: number,
  audio: AudioCodec | null,
  settings: FitSettings,
  onProgress: (fraction: number) => void,
): Promise<Blob> {
  const input = new Input({ source: new BlobSource(file), formats: ALL_FORMATS });
  const output = new Output({ format: new Mp4OutputFormat({ fastStart: "in-memory" }), target: new BufferTarget() });
  try {
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
      },
      audio: audio
        ? { codec: audio, forceTranscode: true, quality: new Quality({ bitrate: plan.audioKbps * 1000 }) }
        : { discard: true },
      showWarnings: false,
    });
    if (!conversion.isValid) {
      const reasons = conversion.discardedTracks.map((d) => `${d.track.type}: ${d.reason}`).join(", ");
      throw new Error(`This browser can't convert this file (${reasons}).`);
    }
    conversion.onProgress = onProgress;
    await conversion.execute();
    return new Blob([output.target.buffer!], { type: "video/mp4" });
  } finally {
    input.dispose();
  }
}

