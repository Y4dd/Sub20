// Gathers one build's installers for the release, under names that can't
// collide with the other builds', and writes what the update feed and the
// checksum list need to know about them.
//
// Usage:
//   node scripts/collect-release.mjs collect --target <triple> --flavor full|lean --out <dir>
//   node scripts/collect-release.mjs manifest --version <x.y.z> --in <dir> --out <dir>
//   node scripts/collect-release.mjs notes --version <x.y.z>
//
// `collect` copies the installers from target/<triple>/release/bundle and
// writes <out>/fragment-<platform>.json. `manifest` merges every fragment in
// <in> into latest.json (the updater feed) and SHA256SUMS.txt.

import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const REPO = "Y4dd/Sub20";

const [command, ...rest] = process.argv.slice(2);
const { values: args } = parseArgs({
  args: rest,
  options: {
    target: { type: "string" },
    flavor: { type: "string" },
    version: { type: "string" },
    in: { type: "string" },
    out: { type: "string" },
  },
});

if (command === "collect") collect();
else if (command === "manifest") manifest();
else if (command === "notes") console.log(notesFor(args.version ?? fail("notes needs --version.")));
else fail("Say collect, manifest or notes.");

function collect() {
  const { target, flavor, out } = args;
  if (!target || !out || !["full", "lean"].includes(flavor)) fail("collect needs --target, --flavor full|lean and --out.");
  const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
  const targetDir = process.env.CARGO_TARGET_DIR ?? join(root, "target");
  const bundle = join(targetDir, target, "release", "bundle");
  if (!existsSync(bundle)) fail(`No bundle folder at ${bundle}. Build first.`);

  const name = flavor === "lean" ? "Sub20-Lean" : "Sub20";
  const arch = target.startsWith("aarch64") ? "aarch64" : "x64";
  const os = target.includes("windows") ? "windows" : target.includes("apple") ? "darwin" : "linux";
  // The key the app asks the feed for (`update_target` in src-tauri/src/lib.rs).
  const platform = `${os}-${target.split("-")[0]}${flavor === "lean" ? "-lean" : ""}`;

  mkdirSync(out, { recursive: true });
  const files = [];
  let updater = null;

  for (const file of walk(bundle)) {
    const base = file.split(/[\\/]/).pop();
    let renamed;
    if (base.endsWith(".app.tar.gz")) renamed = `${name}_${version}_${arch}.app.tar.gz`;
    else if (/\.(exe|dmg|AppImage|deb|rpm)$/.test(base) && base.startsWith("Sub20")) renamed = base.replace(/^Sub20/, name);
    else continue;

    copyFileSync(file, join(out, renamed));
    files.push({ name: renamed, sha256: sha256(join(out, renamed)) });

    const updates = os === "windows" ? renamed.endsWith("-setup.exe") : os === "darwin" ? renamed.endsWith(".app.tar.gz") : renamed.endsWith(".AppImage");
    if (updates) {
      const sig = `${file}.sig`;
      if (!existsSync(sig)) fail(`${base} has no updater signature. Is TAURI_SIGNING_PRIVATE_KEY set?`);
      copyFileSync(sig, join(out, `${renamed}.sig`));
      files.push({ name: `${renamed}.sig`, sha256: sha256(sig) });
      updater = { file: renamed, signature: readFileSync(sig, "utf8").trim() };
    }
  }
  if (!updater) fail(`No installer the updater can use was found in ${bundle}.`);
  // The FFmpeg archives the lean build downloads (fetch-ffmpeg.mjs --keep),
  // listed with the full download so they land in SHA256SUMS.txt.
  if (flavor === "full") {
    for (const base of readdirSync(out)) {
      if (base.startsWith("FFmpeg-")) files.push({ name: base, sha256: sha256(join(out, base)) });
    }
  }

  writeFileSync(join(out, `fragment-${platform}.json`), JSON.stringify({ platform, updater, files }, null, 2));
  for (const file of files) console.log(`${file.name}  ${file.sha256}`);
}

function manifest() {
  const { version, in: from, out } = args;
  if (!version || !from || !out) fail("manifest needs --version, --in and --out.");
  const fragments = walk(from)
    .filter((file) => /fragment-.+\.json$/.test(file))
    .map((file) => JSON.parse(readFileSync(file, "utf8")));
  if (!fragments.length) fail(`No fragments in ${from}.`);

  const platforms = {};
  const sums = [];
  for (const fragment of fragments) {
    platforms[fragment.platform] = {
      signature: fragment.updater.signature,
      url: `https://github.com/${REPO}/releases/download/v${version}/${fragment.updater.file}`,
    };
    for (const file of fragment.files) sums.push(`${file.sha256}  ${file.name}`);
  }

  mkdirSync(out, { recursive: true });
  const notes = notesFor(version);
  writeFileSync(
    join(out, "latest.json"),
    JSON.stringify({ version, notes, pub_date: new Date().toISOString(), platforms }, null, 2) + "\n",
  );
  writeFileSync(join(out, "SHA256SUMS.txt"), sums.sort((a, b) => a.slice(66).localeCompare(b.slice(66))).join("\n") + "\n");
  console.log(`latest.json: ${Object.keys(platforms).sort().join(", ")}`);
}

/** The CHANGELOG.md section for this version, without its heading. */
function notesFor(version) {
  const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8");
  const heading = new RegExp(`^## \\[${version.replace(/\./g, "\\.")}\\][^\\n]*\\n`, "m");
  const start = changelog.search(heading);
  if (start < 0) return `Sub20 ${version}`;
  const body = changelog.slice(start).replace(heading, "");
  const next = body.search(/^## \[/m);
  const section = next < 0 ? body : body.slice(0, next);
  // Drop the link definitions at the foot of the file.
  return section.replace(/^\[[^\]]+\]: .*$/gm, "").trim();
}

function walk(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    // A macOS .app is a folder; only its .tar.gz is wanted.
    if (entry.isDirectory()) return entry.name.endsWith(".app") ? [] : walk(path);
    return [path];
  });
}

function sha256(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
