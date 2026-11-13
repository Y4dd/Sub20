// Builds the planner for WebAssembly (crates/plan-wasm) and puts it where
// Sub20 Lite loads it from: src/lib/web/plan.wasm (not committed). Needs the
// target once: `rustup target add wasm32-unknown-unknown`.

import { execFileSync } from "node:child_process";
import { copyFileSync, statSync } from "node:fs";
import { join } from "node:path";

const target = process.env.CARGO_TARGET_DIR ?? "target";
execFileSync("cargo", ["build", "-p", "sub20-plan-wasm", "--release", "--target", "wasm32-unknown-unknown"], {
  stdio: "inherit",
});
const built = join(target, "wasm32-unknown-unknown", "release", "sub20_plan_wasm.wasm");
const placed = join("src", "lib", "web", "plan.wasm");
copyFileSync(built, placed);
console.log(`${placed}: ${Math.round(statSync(placed).size / 1024)} KB`);
