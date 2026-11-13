// Checks that a built FFmpeg has everything Sub20 asks of it on its system:
// the encoders in crates/core/src/encoders.rs, the decoders for common
// recordings, and the filters the engine's commands use.
//
//   node scripts/ffmpeg-build/check.mjs DIR TARGET
//
// DIR holds ffmpeg and ffprobe. Exits non-zero, naming what is missing.

import { execFileSync } from "node:child_process";
import { join } from "node:path";

const [dir, target] = process.argv.slice(2);
if (!dir || !target) {
  console.error("Usage: check.mjs DIR TARGET");
  process.exit(1);
}
const exe = target.includes("windows") ? ".exe" : "";
const ffmpeg = join(dir, `ffmpeg${exe}`);

const common = {
  encoders: ["libx264", "libx265", "libsvtav1", "aac", "libopus", "mjpeg"],
  decoders: ["h264", "hevc", "vp8", "vp9", "libdav1d", "prores", "mpeg4", "mpeg2video", "wmv3", "vc1", "aac", "mp3", "opus", "vorbis", "ac3", "eac3", "flac", "pcm_s16le"],
  filters: ["scale", "fps", "zscale", "tonemap", "format", "amix", "aformat", "alimiter", "aresample", "volumedetect", "hue", "signalstats"],
  muxers: ["mp4", "matroska", "webm", "mov", "image2pipe", "null"],
  demuxers: ["mov", "matroska", "avi", "flv", "mpegts", "asf", "mpeg"],
  hwaccels: [],
};
const gpu = (family) => ["h264", "hevc", "av1"].map((codec) => `${codec}_${family}`);
const extra = {
  "x86_64-pc-windows-msvc": {
    encoders: [...gpu("nvenc"), ...gpu("amf"), ...gpu("qsv"), ...gpu("mf")],
    hwaccels: ["cuda", "d3d11va", "dxva2", "qsv"],
  },
  "x86_64-unknown-linux-gnu": {
    encoders: [...gpu("nvenc"), ...gpu("amf"), ...gpu("qsv"), ...gpu("vaapi")],
    hwaccels: ["cuda", "vaapi", "qsv"],
  },
  "aarch64-apple-darwin": { encoders: ["h264_videotoolbox", "hevc_videotoolbox"], hwaccels: ["videotoolbox"] },
  "x86_64-apple-darwin": { encoders: ["h264_videotoolbox", "hevc_videotoolbox"], hwaccels: ["videotoolbox"] },
}[target];
if (!extra) {
  console.error(`Unknown target ${target}`);
  process.exit(1);
}

console.log(execFileSync(ffmpeg, ["-hide_banner", "-version"], { encoding: "utf8" }).split("\n")[0]);

const missing = [];
for (const kind of Object.keys(common)) {
  const wanted = [...common[kind], ...(extra[kind] ?? [])];
  const listed = names(kind);
  for (const name of wanted) if (!listed.has(name)) missing.push(`${kind.replace(/s$/, "")} ${name}`);
}

// zscale and tonemap at work on an HDR10-tagged picture, with the chain in
// command::TONE_MAP (keep the two in step).
try {
  execFileSync(ffmpeg, ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=s=320x180:d=0.2",
    "-vf", "setparams=color_primaries=bt2020:color_trc=smpte2084:colorspace=bt2020nc:range=tv,format=yuv420p10le," +
      "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=mobius:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p",
    "-c:v", "libx264", "-f", "null", "-"], { stdio: "inherit" });
} catch {
  missing.push("a working zscale and tonemap chain");
}

if (missing.length) {
  console.error(`Missing from this FFmpeg:\n  ${missing.join("\n  ")}`);
  process.exit(1);
}
console.log(`Everything Sub20 needs is there for ${target}.`);

/** The names FFmpeg lists for -encoders, -filters, -hwaccels and so on. */
function names(kind) {
  const out = execFileSync(ffmpeg, ["-hide_banner", `-${kind}`], { encoding: "utf8" });
  const lines = out.split(/\r?\n/);
  if (kind === "hwaccels") return new Set(lines.slice(1).map((l) => l.trim()).filter(Boolean));
  // Codec and filter lists start after a "------" line, with flags then
  // the name; muxer lists after " --", the same way.
  const start = lines.findIndex((l) => /^\s*-{2,}\s*$/.test(l));
  const set = new Set();
  for (const line of lines.slice(start + 1)) {
    const parts = line.trim().split(/\s+/);
    if (parts.length < 2) continue;
    for (const name of parts[1].split(",")) set.add(name);
  }
  return set;
}
