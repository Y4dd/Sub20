// Small helpers for showing numbers the way people read them.

/** 83.46 → "1:23.5", 3725 → "1:02:05.0". */
export function clock(seconds: number, tenths = true): string {
  const safe = Number.isFinite(seconds) && seconds > 0 ? seconds : 0;
  const total = tenths ? Math.round(safe * 10) : Math.floor(safe) * 10;
  const h = Math.floor(total / 36000);
  const m = Math.floor(total / 600) % 60;
  const s = Math.floor(total / 10) % 60;
  const d = total % 10;
  const tail = tenths ? `.${d}` : "";
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}${tail}` : `${m}:${ss}${tail}`;
}

/** "90", "1:30", "0:01:30.5" → seconds. Returns null for anything else. */
export function parseClock(text: string): number | null {
  const parts = text.trim().split(":");
  if (parts.length === 0 || parts.length > 3 || parts.some((p) => p.trim() === "")) return null;
  let total = 0;
  for (const part of parts) {
    const value = Number(part);
    if (!Number.isFinite(value) || value < 0) return null;
    total = total * 60 + value;
  }
  return total;
}

/** A length in words: "26.8 s", "1 min 12 s", "1 h 4 min". */
export function span(seconds: number): string {
  if (seconds < 60) return `${trimZero(seconds.toFixed(1))} s`;
  const whole = Math.round(seconds);
  const h = Math.floor(whole / 3600);
  const m = Math.floor((whole % 3600) / 60);
  const s = whole % 60;
  if (h > 0) return m > 0 ? `${h} h ${m} min` : `${h} h`;
  return s > 0 ? `${m} min ${s} s` : `${m} min`;
}

/** Time left, rounded the way a person would say it. */
export function remaining(seconds: number): string {
  if (seconds < 5) return "a few seconds left";
  if (seconds < 90) return `about ${Math.round(seconds / 5) * 5} s left`;
  if (seconds < 3600) return `about ${Math.round(seconds / 60)} min left`;
  return `about ${trimZero((seconds / 3600).toFixed(1))} h left`;
}

function trimZero(text: string): string {
  return text.includes(".") ? text.replace(/0+$/, "").replace(/\.$/, "") : text;
}

/** 9.7 → "9.7", 10 → "10". */
export function plain(value: number, digits = 2): string {
  return trimZero(value.toFixed(digits));
}

/**
 * The Discord upload limit a size matches, for the size picker. Discord sets
 * these; check them when it changes its plans.
 */
const DISCORD_TIERS: Record<number, string> = {
  20: "Discord",
  50: "Nitro Basic",
  100: "Boost level 3",
  500: "Nitro",
};

export function discordTier(mb: number): string | null {
  return DISCORD_TIERS[mb] ?? null;
}

/** Bytes as MB (1024 × 1024), matching the size limit the user typed. */
export function megabytes(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  if (mb >= 1000) return `${plain(mb / 1024, 2)} GB`;
  if (mb >= 100) return `${Math.round(mb)} MB`;
  if (mb >= 10) return `${plain(mb, 1)} MB`;
  if (mb >= 0.1) return `${plain(mb, 2)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

/** 2936 → "2.9 Mbps", 640 → "640 kbps". */
export function bitrate(kbps: number): string {
  return kbps >= 1000 ? `${plain(kbps / 1000, 1)} Mbps` : `${Math.round(kbps)} kbps`;
}

/** The short side as people name it: 1280×720 → "720p". */
export function resolution(width: number, height: number): string {
  if (!width || !height) return "";
  return `${Math.min(width, height)}p`;
}

/** File name without its folder. Handles both slash styles. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function folderOf(path: string): string {
  const index = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return index > 0 ? path.slice(0, index) : path;
}

/** A date the way a list wants it: "Today 14:02", "Yesterday", "3 Nov". */
export function when(unixSeconds: number): string {
  const date = new Date(unixSeconds * 1000);
  const now = new Date();
  const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((startOfDay(now) - startOfDay(date)) / 86_400_000);
  const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (days === 0) return `Today ${time}`;
  if (days === 1) return `Yesterday ${time}`;
  const sameYear = date.getFullYear() === now.getFullYear();
  return date.toLocaleDateString([], { day: "numeric", month: "short", year: sameYear ? undefined : "numeric" });
}

export function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

const ENCODER_NAMES: Record<string, string> = {
  nvenc: "NVIDIA GPU",
  qsv: "Intel GPU",
  amf: "AMD GPU",
  mf: "GPU (Media Foundation)",
  videotoolbox: "Apple GPU",
  vaapi: "GPU (VAAPI)",
};

/** "hevc_nvenc" → "NVIDIA GPU"; software encoders → "CPU". */
export function engineOf(encoder: string): string {
  const family = encoder.split("_")[1];
  return (family && ENCODER_NAMES[family]) || "CPU";
}

export const CODEC_LABEL: Record<string, string> = { h264: "H.264", hevc: "HEVC", av1: "AV1" };
