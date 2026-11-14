// The latest release, read from GitHub when the site is built. The Pages
// workflow builds again whenever a release is published, so the download
// links follow it. Without a release (or offline) the links go to the
// releases page instead.
import { REPO, RELEASES } from "./site";

export interface Asset { name: string; url: string; bytes: number }
export interface Release { version: string; url: string; assets: Asset[] }

let cached: Promise<Release | null> | undefined;

export function latestRelease(): Promise<Release | null> {
  cached ??= (async () => {
    try {
      const headers: Record<string, string> = { accept: "application/vnd.github+json" };
      if (process.env.GITHUB_TOKEN) headers.authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
      const response = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers });
      if (!response.ok) return null;
      const json = await response.json();
      return {
        version: String(json.tag_name).replace(/^v/, ""),
        url: json.html_url,
        assets: json.assets.map((a: { name: string; browser_download_url: string; size: number }) => ({
          name: a.name, url: a.browser_download_url, bytes: a.size,
        })),
      };
    } catch {
      return null;
    }
  })();
  return cached;
}

/** One line of the download table: the same rows as the release notes (scripts/collect-release.mjs). */
/** `os` matches the `data-os` the page's head script sets on `<html>` for the visitor's system. */
export const ROWS = [
  { os: "windows", system: "Windows 10 and 11", detail: "64-bit", kind: "Installer (.exe)", file: /-setup\.exe$/ },
  { os: "mac", system: "macOS 11 or newer", detail: "Apple Silicon, M1 or newer", kind: "Disk image (.dmg)", file: /\.dmg$/ },
  { os: "linux", system: "Ubuntu, Debian, Mint", detail: "64-bit", kind: "Package (.deb)", file: /\.deb$/ },
  { os: "linux", system: "Fedora, openSUSE", detail: "64-bit", kind: "Package (.rpm)", file: /\.rpm$/ },
  { os: "linux", system: "Other Linux", detail: "64-bit", kind: "AppImage", file: /\.AppImage$/ },
];

/** A row's kind in running text: "installer (.exe)", but AppImage keeps its name. */
export function kindInText(kind: string): string {
  return kind === "AppImage" ? kind : kind[0].toLowerCase() + kind.slice(1);
}

/** The checksums file on a release, or the releases page. */
export function sums(release: Release | null): string {
  return release?.assets.find((a) => a.name === "SHA256SUMS.txt")?.url ?? RELEASES;
}

export interface Cell { label: string; href: string; size: string | null }

/** The file for a row in one of the two builds, or the releases page when there is none. */
export function cell(release: Release | null, row: (typeof ROWS)[number], lean: boolean): Cell {
  const asset = release?.assets.find((a) => row.file.test(a.name) && a.name.startsWith("Sub20-without-FFmpeg") === lean);
  if (!asset) return { label: row.kind, href: RELEASES, size: null };
  return { label: row.kind, href: asset.url, size: `${Math.round(asset.bytes / (1024 * 1024))} MB` };
}
