// Facts the pages share, so each is written once.

export const REPO = "Y4dd/Sub20";
export const GITHUB = `https://github.com/${REPO}`;
export const RELEASES = `${GITHUB}/releases`;

const BASE = import.meta.env.BASE_URL.replace(/\/$/, "");

/** A path on this site, with the base it is served under. */
export function u(path = ""): string {
  return `${BASE}/${path.replace(/^\//, "")}`;
}

/** Discord's upload limits, as the app's size picker names them (src/lib/format.ts). */
export const DISCORD_LIMITS = [
  { mb: "20 MB", who: "a free account" },
  { mb: "50 MB", who: "Nitro Basic" },
  { mb: "500 MB", who: "a boosted server" },
  { mb: "1 GB", who: "Nitro" },
];
