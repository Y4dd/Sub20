// Sub20's website. Served from GitHub Pages under /Sub20 until the project
// has a domain of its own: then set SITE_URL to it and BASE to "/".
import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";

export default defineConfig({
  site: process.env.SITE_URL ?? "https://y4dd.github.io",
  base: process.env.BASE ?? "/Sub20",
  trailingSlash: "always",
  integrations: [sitemap()],
  // The changelog page reads ../CHANGELOG.md, outside this folder.
  vite: { server: { fs: { allow: [".."] } } },
});
