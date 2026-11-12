<script lang="ts">
  // A small set of line icons, drawn on a 20 px grid.
  const PATHS = {
    plus: "M10 4v12M4 10h12",
    close: "M5 5l10 10M15 5L5 15",
    history: "M3.5 10a6.5 6.5 0 1 0 2-4.7M3.5 4v3.2h3.2M10 6.5V10l2.4 1.6",
    sliders: "M4 6h7M15 6h1M4 14h1M9 14h7M13 4v4M7 12v4",
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
  } as const;

  let { name, size = 18 }: { name: keyof typeof PATHS; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 20 20" fill={name === "play" ? "currentColor" : "none"}
  stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    display: block;
    flex: none;
  }
</style>
