// The parts of Sub20 Lite that only a web build has: an app manifest (so it
// can be installed, opened from "Open with", and shared to on Android), its
// icons, and a service worker that keeps the built files for offline use and
// receives shared videos. Used by vite.config.ts when VITE_TARGET=web.

import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";

const BACKGROUND = "#15171c"; // --bench in src/app.css
const DESCRIPTION = "Trim a video and fit it under Discord's size limit, in your browser. Nothing is uploaded.";

const manifest = {
  name: "Sub20 Lite",
  short_name: "Sub20",
  description: DESCRIPTION,
  start_url: "./",
  scope: "./",
  display: "standalone",
  background_color: BACKGROUND,
  theme_color: BACKGROUND,
  icons: [
    { src: "icon-256.png", sizes: "256x256", type: "image/png" },
    { src: "icon-512.png", sizes: "512x512", type: "image/png" },
    { src: "icon.svg", sizes: "any", type: "image/svg+xml" },
  ],
  // Android: Share → Sub20 from the gallery.
  share_target: {
    action: "./share",
    method: "POST",
    enctype: "multipart/form-data",
    params: { files: [{ name: "videos", accept: ["video/*"] }] },
  },
  // Installed on a computer: "Open with Sub20 Lite".
  file_handlers: [{ action: "./", accept: { "video/*": [".mp4", ".mov", ".mkv", ".webm", ".m4v"] } }],
};

/** The service worker: precached files (stale ones dropped on update), and shared videos kept until the page takes them. */
function worker(cache, files) {
  return `// Sub20 Lite's service worker, written by scripts/vite-lite.mjs.
const CACHE = ${JSON.stringify(cache)};
const FILES = ${JSON.stringify(files)};

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((c) => c.addAll(FILES)).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("sub20-lite-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (event.request.method === "POST" && url.pathname.endsWith("/share")) {
    event.respondWith(receive(event.request));
    return;
  }
  if (event.request.method !== "GET" || url.origin !== location.origin) return;
  event.respondWith(caches.match(event.request, { ignoreSearch: true, cacheName: CACHE }).then((hit) => hit ?? fetch(event.request)));
});

// Videos shared from another app wait in their own cache until the page
// opens and takes them (webBackend's bootstrap).
async function receive(request) {
  const form = await request.formData();
  const shared = await caches.open("sub20-shared");
  let n = 0;
  for (const file of form.getAll("videos")) {
    if (!(file instanceof File)) continue;
    const key = new Request(\`shared/\${Date.now()}-\${n++}/\${encodeURIComponent(file.name)}\`);
    await shared.put(key, new Response(file, { headers: { "content-type": file.type || "video/mp4" } }));
  }
  return Response.redirect("./?shared", 303);
}
`;
}

export function lite() {
  return {
    name: "sub20-lite",
    apply: "build",
    transformIndexHtml(html) {
      return html
        .replace("<title>Sub20</title>", "<title>Sub20 Lite</title>")
        .replace(
          "</head>",
          `    <meta name="description" content="${DESCRIPTION}" />
    <meta name="theme-color" content="${BACKGROUND}" />
    <link rel="manifest" href="./manifest.webmanifest" />
    <link rel="apple-touch-icon" href="./icon-256.png" />
  </head>`,
        );
    },
    generateBundle(_options, bundle) {
      const emit = (fileName, source) => this.emitFile({ type: "asset", fileName, source });
      emit("manifest.webmanifest", JSON.stringify(manifest, null, 2));
      emit("icon-256.png", readFileSync("src-tauri/icons/128x128@2x.png"));
      emit("icon-512.png", readFileSync("src-tauri/icons/icon.png"));
      // index.html is added to the bundle after this hook; it is cached as "./".
      const files = ["./", "manifest.webmanifest", "icon-256.png", "icon-512.png", "icon.svg", ...Object.keys(bundle)]
        .filter((f) => !f.endsWith(".map"));
      const hash = createHash("sha256").update(files.join("\n")).digest("hex").slice(0, 12);
      emit("sw.js", worker(`sub20-lite-${hash}`, [...new Set(files)]));
    },
  };
}
