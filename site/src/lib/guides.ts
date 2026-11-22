// The guides, in the order the guides page lists them. `checked` is the day each was last read against
// the app (shown on the page and sent as the Article's dateModified); change it only after reading the
// guide again.
export const GUIDES = [
  {
    slug: "compress-video-for-discord",
    title: "How to compress a video for Discord",
    summary: "The bitrate that fits, the quick way in Sub20, and the FFmpeg command to do it by hand.",
    checked: "2026-11-17",
  },
  {
    slug: "file-too-large",
    title: "Discord says your file is too large: what to do",
    summary: "Discord's limits, and three ways to get a clip under them.",
    checked: "2026-11-15",
  },
  {
    slug: "nvidia-instant-replay",
    title: "Send an NVIDIA Instant Replay clip on Discord",
    summary: "Where the NVIDIA app saves clips, and how to send one with your microphone in it.",
    checked: "2026-11-22",
  },
  {
    slug: "iphone-video",
    title: "Send an iPhone video on Discord",
    summary: "Why iPhone videos are big and won't always play in the chat, and how to fix both.",
    checked: "2026-11-15",
  },
  {
    slug: "hevc",
    title: "Why your HEVC video may not play on Discord",
    summary: "What HEVC is, why Discord may not play it, and which format plays everywhere.",
    checked: "2026-11-15",
  },
] as const;
