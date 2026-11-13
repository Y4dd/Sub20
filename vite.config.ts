import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
// @ts-expect-error: a plain script, typed by its use here.
import { lite } from "./scripts/vite-lite.mjs";

// Root-relative, which Vite resolves from the project folder.
const absent = "/src/lib/absent.ts";

// `--mode web` (`npm run build:web`) builds Sub20 Lite into dist-web/, with
// relative paths so it can be served from any folder (the site's /app).
// .env.web sets VITE_TARGET=web for the app code in that mode.
export default defineConfig(({ command, mode }) => {
  const web = mode === "web";
  const build = command === "build";
  return {
    plugins: [svelte(), ...(web ? [lite()] : [])],
    base: web ? "./" : "/",
    resolve: {
      // Each build leaves out the backend it never uses, with what it brings
      // (Lite's planner and Mediabunny; the mock's demo clips). Development
      // keeps all three, for \`?lite\`.
      alias: build ? [{ find: web ? /^\.\/mock$/ : /^\.\/web\/backend$/, replacement: absent }] : [],
    },
    // Tauri expects a fixed port and shows Rust errors itself.
    clearScreen: false,
    server: {
      port: 1420,
      strictPort: true,
      watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
    },
    build: {
      target: "es2022",
      outDir: web ? "dist-web" : "dist",
      emptyOutDir: true,
      // Lite's media chunk (Mediabunny, about 570 kB) loads with the first video.
      chunkSizeWarningLimit: web ? 600 : 500,
    },
  };
});
