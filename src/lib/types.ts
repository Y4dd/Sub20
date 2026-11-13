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
  /** How many audio tracks; the facts above describe the first. */
  audioTracks: number;
  /** High dynamic range, turned into ordinary colour when compressed. */
  hdr: "pq" | "hlg" | null;
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
  /** Mix every audio track into one, so a mic on its own track is kept. */
  mixAudio: boolean;
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
  /** The source went to the Recycle Bin and this file took its place. */
  replacedOriginal: boolean;
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

export type OutputMode = "sameFolder" | "folder" | "replace";
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
  checkUpdates: boolean;
  lastOptions: EncodeOptions | null;
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
  /** This FFmpeg can turn HDR into ordinary colour. */
  toneMap: boolean;
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

/** A recording app's clip folder, offered as a one-click watch. */
export interface ClipFolder {
  app: string;
  path: string;
  /** Clips land in a subfolder per game. */
  recursive: boolean;
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
  /** FFmpeg ships inside this build. */
  bundled: boolean;
  /** The bundled FFmpeg's licence and origin, as a file. */
  ffmpegNotice: string | null;
  /** The FFmpeg this system can download when it has none. */
  ffmpegOffer: FfmpegOffer | null;
}

/** An FFmpeg Sub20 can download (`fetch::Offer`). */
export interface FfmpegOffer {
  version: string;
  builder: string;
  source: string;
  files: { name: string; url: string; sha256: string }[];
}

/** How a download of FFmpeg is going. */
export interface Fetching {
  stage: "downloading" | "checking" | "installing";
  /** 0 to 1, when the size is known. */
  fraction: number | null;
  /** Bytes so far. */
  received: number;
}

export interface Fetched {
  ffmpeg: FfmpegStatus;
  /** FFmpeg's licence and origin, as a file. */
  notice: string;
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
  /** What to call the result, as typed; null uses name-20mb. */
  outputName: string | null;
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

/** A newer release on the update feed. */
export interface UpdateInfo {
  version: string;
  notes: string | null;
}

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
  /** Downloads the offered FFmpeg, checks it against its pin and installs it. */
  downloadFfmpeg(onProgress: (progress: Fetching) => void): Promise<Fetched>;
  addVideos(paths: string[]): Promise<Added[]>;
  /** Thumbnail URLs, in order. `onThumb` hears each one as soon as it is made. */
  filmstrip(path: string, duration: number, count: number, onThumb: (index: number, url: string) => void): Promise<string[]>;
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
  /** Recording apps' clip folders found on this machine (Windows only). */
  clipFolders(): Promise<ClipFolder[]>;
  reveal(path: string): Promise<void>;
  openFile(path: string): Promise<void>;
  /** A URL the stage can play a finished video from; null shows still frames. */
  resultUrl(path: string): Promise<string | null>;
  /** Puts a finished video on the clipboard as a file, ready to paste into Discord. */
  copyFile(path: string): Promise<void>;
  /** Renames a finished video, keeping its extension; returns the new path. */
  renameResult(path: string, name: string): Promise<string>;
  /** Asks the release feed for a newer version; null when this one is the latest. */
  checkUpdate(): Promise<UpdateInfo | null>;
  /** Installs the update `checkUpdate` found and restarts the app. Running jobs are stopped. */
  installUpdate(): Promise<void>;
  /** Puts what a bug report needs on the clipboard, as Markdown with the home folder hidden. `log` is the shown job's FFmpeg log. */
  copyDiagnostics(log: string[]): Promise<void>;
  /** Drags a finished video out of the window, a filmstrip thumbnail under the pointer. */
  dragOut(path: string, thumb: string | null): Promise<void>;
  pickVideos(): Promise<string[]>;
  pickFolder(): Promise<string | null>;
  onJob(handler: (event: JobEvent) => void): Promise<() => void>;
  onDrop(handler: (event: DropEvent) => void): Promise<() => void>;
  /** Videos opened with Sub20 ("Open with", Explorer, Finder) while it is already running. */
  onOpened(handler: (paths: string[]) => void): Promise<() => void>;
}
