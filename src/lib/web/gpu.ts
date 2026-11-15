// What the browser's GPU encoder takes, asked before planning, so the size
// and verdict the stage shows are those of the encode that will run. GPUs
// that take 1440p30 can refuse 1440p60, and when the GPU says no the browser
// quietly uses its software encoder, which ignores low bitrates.

import type { Codec, EncodeOptions, MediaInfo } from "../types";

/** Short sides the planner steps through (`LADDER` in `plan.rs`). */
export const LADDER = [2160, 1440, 1080, 720, 480, 360, 240];

/** The bitrate the question is asked at: support is about size and frame rate. */
const ASK_KBPS = 4000;

const even = (value: number) => Math.max(2, Math.floor(value / 2) * 2);

/** WebCodecs strings to ask about each codec with: high profiles, levels that hold 4K. */
const PROBE: Record<Codec, string> = { h264: "avc1.640034", hevc: "hev1.1.6.L153.90", av1: "av01.0.12M.08" };

/** Fallbacks in order: AV1 before HEVC. Discord's desktop app plays AV1 on any computer, while HEVC needs the computer to decode it; phones are the other way round. */
const ORDER: Codec[] = ["h264", "av1", "hevc"];

let encodable: Promise<Codec[]> | null = null;

/** The codecs this browser can encode at all (on the GPU or not), in fallback order. Asked once. */
export function codecsHere(): Promise<Codec[]> {
  encodable ??= (async () => {
    if (!("VideoEncoder" in window)) return [];
    const ask = (codec: Codec) =>
      VideoEncoder.isConfigSupported({ codec: PROBE[codec], width: 1280, height: 720, bitrate: 2e6 })
        .then((r) => r.supported ?? false, () => false);
    const answers = await Promise.all(ORDER.map(ask));
    return ORDER.filter((_, i) => answers[i]);
  })();
  return encodable;
}

/** The codec to encode with: the one asked for where this browser has it, else the first it has. */
export async function codecFor(wanted: Codec): Promise<Codec | null> {
  const here = await codecsHere();
  return here.includes(wanted) ? wanted : (here[0] ?? null);
}

/** Whether the GPU can encode `codec` at this size, bitrate and frame rate. */
export async function gpuTakes(codec: Codec, width: number, height: number, kbps: number, fps: number): Promise<boolean> {
  if (!("VideoEncoder" in window)) return false;
  try {
    // Asked of WebCodecs directly: Mediabunny's `canEncodeVideo` leaves the frame rate out.
    const { supported } = await VideoEncoder.isConfigSupported({
      codec: PROBE[codec], width, height, bitrate: kbps * 1000, framerate: fps, hardwareAcceleration: "prefer-hardware",
    });
    return supported ?? false;
  } catch {
    return false;
  }
}

/** The source scaled so its short side is `side`, in even pixels. */
export function scaled(source: MediaInfo, side: number): { width: number; height: number } {
  const short = Math.min(source.displayWidth, source.displayHeight);
  return { width: even((source.displayWidth * side) / short), height: even((source.displayHeight * side) / short) };
}

/** The frame rate the plan will encode at (`plan.rs`: the cap only when the source is faster). */
export function outputFps(source: MediaInfo, options: EncodeOptions): number {
  const cap = options.maxFps && options.maxFps > 0 ? options.maxFps : null;
  const fps = source.fps ?? 30;
  return cap && fps > cap + 0.01 ? cap : fps;
}

const asked = new Map<string, Promise<number | null>>();

/**
 * The largest short side the GPU takes for this source at `fps`, or `null`
 * when it takes the source's own size, or nothing at all (the CPU encoder
 * then has no size limit to plan within). Asked once per size and rate.
 */
export function gpuMostShortSide(source: MediaInfo, fps: number, codec: Codec): Promise<number | null> {
  const key = `${codec} ${source.displayWidth}x${source.displayHeight}@${fps}`;
  let answer = asked.get(key);
  if (!answer) {
    answer = ask(source, fps, codec);
    asked.set(key, answer);
  }
  return answer;
}

async function ask(source: MediaInfo, fps: number, codec: Codec): Promise<number | null> {
  const short = Math.min(source.displayWidth, source.displayHeight);
  if (!short) return null;
  if (await gpuTakes(codec, even(source.displayWidth), even(source.displayHeight), ASK_KBPS, fps)) return null;
  for (const side of LADDER.filter((s) => s < short)) {
    const { width, height } = scaled(source, side);
    if (await gpuTakes(codec, width, height, ASK_KBPS, fps)) return side;
  }
  return null;
}
