// Mirrors of the Rust types in `crates/core`. Field names follow serde's
// camelCase output.

export type Codec = "h264" | "hevc" | "av1";
export type AudioChoice = "opus" | "aac" | "none";
export type Container = "mp4" | "mkv";
export type Effort = "p1" | "p2" | "p3" | "p4" | "p5" | "p6" | "p7" | "max";
export type Tune = "hq" | "ll" | "ull" | "lossless";

export interface MediaInfo {
  duration: number;
  sizeBytes: number;
  hasVideo: boolean;
  hasAudio: boolean;
  width: number;
  height: number;
  displayWidth: number;
  displayHeight: number;
  rotation: number;
  fps: number | null;
  videoCodec: string | null;
  pixelFormat: string | null;
  videoKbps: number | null;
  audioCodec: string | null;
  audioKbps: number | null;
  audioChannels: number | null;
}

export interface Trim {
  start: number;
  end: number;
}

export interface EncodeOptions {
  targetMb: number;
  videoKbps: number | null;
  codec: Codec;
  encoder: string | null;
  audio: AudioChoice;
  audioKbps: number;
  effort: Effort;
  tune: Tune;
  container: Container;
  autoResolution: boolean;
  minAutoHeight: number;
  maxShortSide: number | null;
  maxWidth: number | null;
  maxHeight: number | null;
  maxFps: number | null;
  audioOnly: boolean;
  fastFinalize: boolean;
  hardwareDecode: boolean;
}

export type Quality = "sharp" | "good" | "soft" | "rough";

export interface Plan {
  start: number;
  duration: number;
  trimmed: boolean;
  targetBytes: number | null;
  videoKbps: number;
  audio: "opus" | "aac" | null;
  audioKbps: number;
  width: number;
  height: number;
  scaled: boolean;
  fpsCap: number | null;
  quality: Quality;
  expectedBytes: number;
  notes: string[];
}

export interface JobResult {
  output: string;
  sizeBytes: number;
  originalBytes: number;
  duration: number;
  width: number;
  height: number;
  videoKbps: number;
  audioKbps: number;
  encoder: string;
  hardware: boolean;
  fellBackFrom: string | null;
  attempts: number;
  elapsedSeconds: number;
  targetMb: number | null;
}

export type Phase = "analyzing" | "analysis" | "encoding" | "finalizing";
export type Origin = "manual" | "watch";

export type JobEvent =
  | { type: "queued"; id: string; input: string; origin: Origin }
  | { type: "started"; id: string; encoder: string; hardware: boolean; videoKbps: number; width: number; height: number }
  | { type: "progress"; id: string; phase: Phase; percent: number; speed: number | null; etaSeconds: number | null; outBytes: number }
  | { type: "log"; id: string; line: string }
  | { type: "fallback"; id: string; from: string; to: string; reason: string }
  | { type: "retry"; id: string; attempt: number; overPercent: number; videoKbps: number }
  | { type: "done"; id: string; result: JobResult }
  | { type: "failed"; id: string; error: string }
  | { type: "canceled"; id: string };

export interface Profile {
  name: string;
  options: EncodeOptions;
}

export type OutputMode = "sameFolder" | "folder";
export type Originals = "keep" | "delete" | "move";
export type Existing = "newOnly" | "processExisting";

export interface WatchSettings {
  enabled: boolean;
  folder: string | null;
  profile: string | null;
  outputMode: OutputMode;
  outputFolder: string | null;
  originals: Originals;
  existing: Existing;
  recursive: boolean;
  stableSeconds: number;
  pollSeconds: number;
}

export interface Settings {
  profiles: Profile[];
  defaultProfile: string;
  sizes: number[];
  disabledEncoders: string[];
  outputMode: OutputMode;
  outputFolder: string | null;
  historyEnabled: boolean;
  historyKeepDays: number;
  parallelJobs: number;
  ffmpegFolder: string | null;
  watch: WatchSettings;
}

export type Family = "nvenc" | "qsv" | "amf" | "mf" | "videotoolbox" | "vaapi" | "cpu";
export type EncoderState = "works" | "failed" | "missing" | "elsewhere";

export interface EncoderStatus {
  name: string;
  label: string;
  codec: Codec;
  family: Family;
  familyLabel: string;
  state: EncoderState;
  detail: string | null;
  device: string | null;
}

export interface Report {
  encoders: EncoderStatus[];
  ffmpegVersion: string | null;
  checkedAt: number;
}

export interface HistoryEntry {
  id: string;
  finishedAt: number;
  input: string;
  trimmed: boolean;
  origin: Origin;
  result: JobResult;
}

export interface WatchStatus {
  running: boolean;
  folder: string | null;
  lastScan: number;
  inProgress: number;
  done: number;
  failed: number;
  problem: string | null;
}

export interface FfmpegStatus {
  found: boolean;
  version: string | null;
  folder: string | null;
}

export interface Bootstrap {
  settings: Settings;
  ffmpeg: FfmpegStatus;
  version: string;
  /** Videos the app was started with ("Open with Sub20"). */
  openedWith: string[];
}

export interface Added {
  path: string;
  name: string;
  info: MediaInfo | null;
  /** Where to load the video from, when not through the asset protocol. */
  playUrl: string | null;
  error: string | null;
}

export interface CompressRequest {
  path: string;
  trim: Trim | null;
  options: EncodeOptions;
}

export type DropEvent = { type: "over" } | { type: "leave" } | { type: "drop"; paths: string[] };

/// Everything the window asks of the app. Implemented once for Tauri and
/// once as an in-browser stand-in for design work.
/**
 * Who draws the window frame. "custom": the app draws its own title bar
 * and window buttons (Windows). "overlay": the system's buttons sit over
 * the app's bar, on the left (macOS). "native": the system frame (Linux).
 */
export type Frame = "custom" | "overlay" | "native";

export interface Backend {
  readonly demo: boolean;
  readonly frame: Frame;
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  closeWindow(): Promise<void>;
  /** Calls back with whether the window is maximized, now and on each change. */
  onMaximized(handler: (maximized: boolean) => void): Promise<() => void>;
  bootstrap(): Promise<Bootstrap>;
  ffmpegStatus(): Promise<FfmpegStatus>;
  addVideos(paths: string[]): Promise<Added[]>;
  /** Thumbnail URLs, in order. */
  filmstrip(path: string, duration: number, count: number): Promise<string[]>;
  /** A URL the `<video>` element can try to play, or null when there is none. */
  videoUrl(path: string): string | null;
  /** One still frame as an image URL. */
  previewFrame(path: string, at: number): Promise<string>;
  estimate(info: MediaInfo, trim: Trim | null, options: EncodeOptions): Promise<Plan>;
  compress(jobs: CompressRequest[]): Promise<string[]>;
  cancel(id: string): Promise<void>;
  cancelAll(): Promise<void>;
  saveSettings(settings: Settings): Promise<Settings>;
  encoders(refresh: boolean): Promise<Report>;
  historyList(): Promise<HistoryEntry[]>;
  historyRemove(id: string): Promise<void>;
  historyClear(): Promise<void>;
  watchStatus(): Promise<WatchStatus>;
  reveal(path: string): Promise<void>;
  openFile(path: string): Promise<void>;
  /** A URL the stage can play a finished video from; null shows still frames. */
  resultUrl(path: string): Promise<string | null>;
  /** Puts a finished video on the clipboard as a file, ready to paste into Discord. */
  copyFile(path: string): Promise<void>;
  /** Drags a finished video out of the window, a filmstrip thumbnail under the pointer. */
  dragOut(path: string, thumb: string | null): Promise<void>;
  pickVideos(): Promise<string[]>;
  pickFolder(): Promise<string | null>;
  onJob(handler: (event: JobEvent) => void): Promise<() => void>;
  onDrop(handler: (event: DropEvent) => void): Promise<() => void>;
}
