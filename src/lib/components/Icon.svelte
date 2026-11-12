<script lang="ts">
  // A small set of line icons, drawn on a 20 px grid.
  const PATHS = {
    plus: "M10 4v12M4 10h12",
    close: "M5 5l10 10M15 5L5 15",
    history: "M3.5 10a6.5 6.5 0 1 0 2-4.7M3.5 4v3.2h3.2M10 6.5V10l2.4 1.6",
    sliders: "M4 6h7M15 6h1M4 14h1M9 14h7M13 4v4M7 12v4",
    gear: "M15.36 8.37L17.42 8.37L17.42 11.63L15.36 11.63L14.94 12.64L16.4 14.1L14.1 16.4L12.64 14.94L11.63 15.36L11.63 17.42L8.37 17.42L8.37 15.36L7.36 14.94L5.9 16.4L3.6 14.1L5.06 12.64L4.64 11.63L2.58 11.63L2.58 8.37L4.64 8.37L5.06 7.36L3.6 5.9L5.9 3.6L7.36 5.06L8.37 4.64L8.37 2.58L11.63 2.58L11.63 4.64L12.64 5.06L14.1 3.6L16.4 5.9L14.94 7.36zM12.4 10a2.4 2.4 0 1 1-4.8 0a2.4 2.4 0 1 1 4.8 0z",
    left: "M12 5l-5 5 5 5",
    right: "M8 5l5 5-5 5",
    down: "M5 8l5 5 5-5",
    play: "M7 4.5v11l9-5.5z",
    pause: "M7 4.5v11M13 4.5v11",
    check: "M4.5 10.5l3.5 3.5 7.5-8",
    alert: "M10 6v5M10 14v.01M10 2.5l8 14H2z",
    folder: "M2.5 5.5h5l1.5 2h8.5v8h-15z",
    refresh: "M16 10a6 6 0 1 1-1.8-4.3M16 4v3h-3",
    trash: "M4 6h12M8 6V4h4v2M6 6l.7 10h6.6L14 6",
    film: "M3 4.5h14v11H3zM3 8h14M3 12h14M7 4.5V8M13 4.5V8M7 12v3.5M13 12v3.5",
    copy: "M7.5 7.5h8.5v8.5H7.5zM4 12.5V4h8.5",
    grip: "M7.5 5.5h.01M12.5 5.5h.01M7.5 10h.01M12.5 10h.01M7.5 14.5h.01M12.5 14.5h.01",
  } as const;

  let { name, size = 18 }: { name: keyof typeof PATHS; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 20 20" fill={name === "play" ? "currentColor" : "none"}
  stroke="currentColor" stroke-width={name === "grip" ? 2.6 : 1.6} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    display: block;
    flex: none;
  }
</style>
