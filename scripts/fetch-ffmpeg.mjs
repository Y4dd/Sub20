// Downloads the FFmpeg pinned in scripts/ffmpeg.json for one target, checks
// its SHA-256, and puts it where the bundled build expects it:
//
//   src-tauri/binaries/sub20-ffmpeg-<target>[.exe]
//   src-tauri/binaries/sub20-ffprobe-<target>[.exe]
//   src-tauri/ffmpeg-notice/FFMPEG.txt   (version, origin, checksums)
//
// With --keep DIR it also copies the checked archives, unchanged, into DIR
// under the names the lean build downloads them by from the release page
// (`asset_name` in crates/core/src/fetch.rs; change both together).
//
// Usage: node scripts/fetch-ffmpeg.mjs [target] [--keep DIR]
// The target defaults to this machine's Rust host triple.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pins = JSON.parse(readFileSync(join(root, "scripts", "ffmpeg.json"), "utf8"));

const argv = process.argv.slice(2);
const keepAt = argv.indexOf("--keep");
const keep = keepAt >= 0 ? argv.splice(keepAt, 2)[1] : null;
const target = argv[0] ?? hostTriple();
const pin = pins.targets[target];
if (!pin) {
  console.error(`No FFmpeg is pinned for ${target}. Pinned: ${Object.keys(pins.targets).join(", ")}`);
  process.exit(1);
}
const windows = target.includes("windows");
const ext = windows ? ".exe" : "";

const cache = join(tmpdir(), "sub20-ffmpeg");
mkdirSync(cache, { recursive: true });

const binaries = join(root, "src-tauri", "binaries");
const notice = join(root, "src-tauri", "ffmpeg-notice");
mkdirSync(binaries, { recursive: true });
mkdirSync(notice, { recursive: true });

const work = mkdtempSync(join(tmpdir(), "sub20-ffmpeg-x-"));
try {
  const downloads = [];
  if (pin.archive) {
    const file = await download(pin.archive);
    downloads.push({ ...pin.archive, file, tool: null });
    unpack(file, work);
  } else {
    for (const tool of ["ffmpeg", "ffprobe"]) {
      const file = await download(pin[tool]);
      downloads.push({ ...pin[tool], file, tool });
      unpack(file, work);
    }
  }

  for (const tool of ["ffmpeg", "ffprobe"]) {
    const found = find(work, tool + ext);
    if (!found) throw new Error(`${tool}${ext} is not in the download`);
    const dest = join(binaries, `sub20-${tool}-${target}${ext}`);
    copyFileSync(found, dest);
    if (!windows) chmodSync(dest, 0o755);
    console.log(`${dest} (${Math.round(statSync(dest).size / 1048576)} MB)`);
  }

  if (keep) {
    mkdirSync(keep, { recursive: true });
    for (const d of downloads) {
      const ext = d.url.endsWith(".tar.xz") ? ".tar.xz" : ".zip";
      const name = `FFmpeg-${pins.version}-${target}${d.tool ? "-" + d.tool : ""}${ext}`;
      copyFileSync(d.file, join(keep, name));
      console.log(join(keep, name));
    }
  }

  writeFileSync(
    join(notice, "FFMPEG.txt"),
    [
      `This copy of Sub20 includes FFmpeg ${pins.version} (the ffmpeg and ffprobe`,
      "programs). FFmpeg is free software, licensed under the GNU General Public",
      "License version 3 (LICENSE-GPL-3.0.txt beside this file). Sub20 runs",
      "them as separate programs; Sub20 itself is MIT licensed.",
      "",
      `Built by: ${pin.builder}`,
      `FFmpeg source: ${pin.source}`,
      "",
      ...downloads.flatMap((d) => [`Downloaded from: ${d.url}`, `SHA-256: ${d.sha256}`]),
      "",
    ].join("\n"),
  );
} finally {
  rmSync(work, { recursive: true, force: true });
}

async function download({ url, sha256 }) {
  const file = join(cache, sha256);
  if (existsSync(file) && hash(file) === sha256) return file;
  console.log(`Downloading ${url}`);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  writeFileSync(file, Buffer.from(await response.arrayBuffer()));
  const got = hash(file);
  if (got !== sha256) {
    rmSync(file);
    throw new Error(`${url}: SHA-256 is ${got}, expected ${sha256}`);
  }
  return file;
}

function hash(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

// bsdtar reads zip and tar.xz alike. On Windows, use the one Windows ships:
// the tar that Git Bash puts first on PATH is GNU tar, which can't read zip.
function unpack(file, into) {
  const tar = process.platform === "win32" ? join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe") : "tar";
  execFileSync(tar, ["-xf", file, "-C", into], { stdio: "inherit" });
}

function find(dir, name) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      const found = find(path, name);
      if (found) return found;
    } else if (entry.name === name) {
      return path;
    }
  }
  return null;
}

function hostTriple() {
  const out = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  return out.match(/^host: (.+)$/m)[1].trim();
}
