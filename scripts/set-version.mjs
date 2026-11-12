// Sets the version everywhere it is written, so a release tag, the app's
// About line and the updater all agree.
// usage: npm run set-version -- 0.2.0
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version ?? "")) {
  console.error("usage: npm run set-version -- <major.minor.patch>, for example 0.2.0");
  process.exit(1);
}

function edit(path, pattern) {
  const text = readFileSync(path, "utf8");
  if (!pattern.test(text)) throw new Error(`${path}: version field not found`);
  writeFileSync(path, text.replace(pattern, `$1${version}$2`));
  console.log(`${path} → ${version}`);
}

const json = /^(\s*"version":\s*")[^"]*(")/m;
edit("package.json", json);
edit("src-tauri/tauri.conf.json", json);
// The workspace version, which both crates inherit.
edit("Cargo.toml", /(\[workspace\.package\][^[]*?\nversion\s*=\s*")[^"]*(")/);

// Bring the lock files in step without touching any other dependency.
execSync("npm install --package-lock-only --ignore-scripts", { stdio: "inherit" });
execSync("cargo update --workspace", { stdio: "inherit" });

console.log(`\nNow add a "## [${version}]" section to CHANGELOG.md, commit, and tag v${version}.`);
