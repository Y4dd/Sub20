// What the browser's GPU encoder takes, asked before planning, so the size
// and verdict the stage shows are those of the encode that will run. GPUs
// that take 1440p30 can refuse 1440p60, and when the GPU says no the browser
// quietly uses its software encoder, which ignores low bitrates.

import type { EncodeOptions, MediaInfo } from "../types";

/** Short sides the planner steps through (`LADDER` in `plan.rs`). */
export const LADDER = [2160, 1440, 1080, 720, 480, 360, 240];

/** The bitrate the question is asked at: support is about size and frame rate. */
const ASK_KBPS = 4000;

const even = (value: number) => Math.max(2, Math.floor(value / 2) * 2);

/** Whether the GPU can encode H.264 at this size, bitrate and frame rate. */
export async function gpuTakes(width: number, height: number, kbps: number, fps: number): Promise<boolean> {
  if (!("VideoEncoder" in window)) return false;
  try {
    // Asked of WebCodecs directly: Mediabunny's `canEncodeVideo` leaves the frame rate out.
    const { supported } = await VideoEncoder.isConfigSupported({
      codec: "avc1.640034", width, height, bitrate: kbps * 1000, framerate: fps, hardwareAcceleration: "prefer-hardware",
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
export function gpuMostShortSide(source: MediaInfo, fps: number): Promise<number | null> {
  const key = `${source.displayWidth}x${source.displayHeight}@${fps}`;
  let answer = asked.get(key);
  if (!answer) {
    answer = ask(source, fps);
    asked.set(key, answer);
  }
  return answer;
}

async function ask(source: MediaInfo, fps: number): Promise<number | null> {
  const short = Math.min(source.displayWidth, source.displayHeight);
  if (!short) return null;
  if (await gpuTakes(even(source.displayWidth), even(source.displayHeight), ASK_KBPS, fps)) return null;
  for (const side of LADDER.filter((s) => s < short)) {
    const { width, height } = scaled(source, side);
    if (await gpuTakes(width, height, ASK_KBPS, fps)) return side;
  }
  return null;
}
