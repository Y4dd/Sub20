// The source archives Sub20's FFmpeg is built from, pinned with SHA-256 in
// sources.json. These same files are published beside the binaries, as
// the GPL asks.
//
//   node scripts/ffmpeg-build/sources.mjs fetch DIR [--target T]
//       Downloads each archive the target needs into DIR, under its own
//       file name, and checks it. Without --target, every archive.
//   node scripts/ffmpeg-build/sources.mjs pin
//       Downloads the archives whose sha256 is empty and writes it in.
//   node scripts/ffmpeg-build/sources.mjs list [--target T]
//       Prints "name<TAB>file" per archive, for build.sh.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pinsFile = join(here, "sources.json");
const pins = JSON.parse(readFileSync(pinsFile, "utf8"));

const argv = process.argv.slice(2);
const targetAt = argv.indexOf("--target");
const target = targetAt >= 0 ? argv.splice(targetAt, 2)[1] : null;
const [command, dir] = argv;

const wanted = pins.sources.filter((s) => !target || !s.targets || s.targets.includes(target));

if (command === "fetch") {
  if (!dir) fail("fetch needs a folder.");
  mkdirSync(dir, { recursive: true });
  for (const source of wanted) {
    if (!source.sha256) fail(`${source.name} has no sha256 yet. Run: node scripts/ffmpeg-build/sources.mjs pin`);
    const file = join(dir, fileName(source));
    if (existsSync(file) && hash(file) === source.sha256) continue;
    await download(source.url, file);
    const got = hash(file);
    if (got !== source.sha256) {
      rmSync(file);
      fail(`${source.url}: SHA-256 is ${got}, expected ${source.sha256}`);
    }
    console.log(`${fileName(source)}  ${got}`);
  }
} else if (command === "pin") {
  const scratch = join(tmpdir(), "sub20-ffmpeg-sources");
  mkdirSync(scratch, { recursive: true });
  for (const source of pins.sources.filter((s) => !s.sha256)) {
    const file = join(scratch, fileName(source));
    await download(source.url, file);
    source.sha256 = hash(file);
    console.log(`${source.name}  ${source.sha256}`);
    writeFileSync(pinsFile, JSON.stringify(pins, null, 2) + "\n");
  }
} else if (command === "list") {
  for (const source of wanted) console.log(`${source.name}\t${fileName(source)}`);
} else {
  fail("Usage: sources.mjs fetch DIR [--target T] | pin | list [--target T]");
}

/** The archive's file name: the URL's last part, prefixed with the name when that doesn't say what it is. */
function fileName(source) {
  const last = new URL(source.url).pathname.split("/").pop();
  return last.toLowerCase().startsWith(source.name.toLowerCase().slice(0, 3)) ? last : `${source.name}-${last}`;
}

async function download(url, file) {
  console.log(`Downloading ${url}`);
  // curl rather than fetch: code.videolan.org's bot check answers fetch
  // with HTTP 406.
  try {
    execFileSync("curl", ["-fsSL", "--retry", "3", "-o", `${file}.part`, url], { stdio: "inherit" });
  } catch {
    fail(`${url}: download failed`);
  }
  renameSync(`${file}.part`, file);
}

function hash(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
