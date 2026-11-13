// The desktop app's planner, compiled to WebAssembly (`crates/plan-wasm`),
// so Sub20 Lite plans with the same rule and the same tests. Build it with
// `node scripts/build-plan-wasm.mjs` (`npm run build:web` does).

import type { EncodeOptions, MediaInfo, Plan, Trim } from "../types";

interface Exports {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  dealloc(ptr: number, len: number): void;
  plan(ptr: number, len: number): bigint;
  lower_bitrate(videoKbps: number, size: number, target: number, duration: number): number;
  size_retries(): number;
}

export interface Planner {
  /** The plan for a clip; throws the planner's advice when it can't fit. */
  plan(source: MediaInfo, trim: Trim | null, options: EncodeOptions): Plan;
  /** The next video bitrate after a result landed `size` bytes against `target`. */
  lowerBitrate(videoKbps: number, size: number, target: number, duration: number): number;
  /** How many times a too-large result is encoded again. */
  sizeRetries: number;
}

let loading: Promise<Planner> | null = null;

/** Loads the planner once; later calls share it. */
export function loadPlanner(bytes?: BufferSource): Promise<Planner> {
  loading ??= instantiate(bytes);
  return loading;
}

async function instantiate(bytes?: BufferSource): Promise<Planner> {
  let source: BufferSource;
  if (bytes) {
    source = bytes;
  } else {
    const response = await fetch(new URL("./plan.wasm", import.meta.url));
    if (!response.ok) throw new Error("The planner isn't built: run node scripts/build-plan-wasm.mjs.");
    source = await response.arrayBuffer();
  }
  const { instance } = await WebAssembly.instantiate(source, {});
  const wasm = instance.exports as unknown as Exports;
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();

  return {
    plan(source, trim, options) {
      const request = encoder.encode(JSON.stringify({ source, trim, options }));
      const ptr = wasm.alloc(request.length);
      new Uint8Array(wasm.memory.buffer, ptr, request.length).set(request);
      const packed = wasm.plan(ptr, request.length);
      const at = Number(packed >> 32n);
      const len = Number(packed & 0xffffffffn);
      const text = decoder.decode(new Uint8Array(wasm.memory.buffer, at, len));
      wasm.dealloc(at, len);
      const answer = JSON.parse(text) as { plan: Plan } | { error: string };
      if ("error" in answer) throw new Error(answer.error);
      return answer.plan;
    },
    lowerBitrate: (videoKbps, size, target, duration) => wasm.lower_bitrate(videoKbps, size, target, duration),
    sizeRetries: wasm.size_retries(),
  };
}
