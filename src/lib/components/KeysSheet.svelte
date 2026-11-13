<script lang="ts">
  // Every key the window answers to. Opened with "?".
  import { app } from "../app.svelte";
  import Sheet from "./Sheet.svelte";

  const mac = navigator.platform.toLowerCase().includes("mac");
  const mod = mac ? "⌘" : "Ctrl";

  const GROUPS: { title: string; keys: [string[], string][] }[] = [
    {
      title: "Videos",
      keys: [
        [["j", "↓"], "Next video"],
        [["k", "↑"], "Previous video"],
        [["gg", "G"], "First or last video (5G: the fifth)"],
        [["r", "cw", "F2"], "Rename the result"],
        [["s"], "Leave out of Compress, or include again"],
        [["x", "dd", "Delete"], "Remove from the list"],
      ],
    },
    {
      title: "Playhead and trim",
      keys: [
        [["Space"], "Play or pause the kept part"],
        [["h", "l", "←", "→"], "Back or forward 0.1 s"],
        [["b", "w", "H", "L"], "Back or forward 1 s"],
        [["0", "$"], "Start or end of the kept part"],
        [["i", "o"], "Set the start or end at the playhead"],
        [["PgUp", "PgDn"], "With a trim handle focused, move it a twentieth of the video"],
      ],
    },
    {
      title: "Results",
      keys: [
        [[`${mod}+Enter`], "Compress"],
        [["y", `${mod}+C`], "Copy the finished video, ready to paste"],
      ],
    },
  ];
</script>

<Sheet title="Keyboard" onclose={() => (app.sheet = null)}>
  <p class="muted lead">Vim motions work throughout. A number first repeats a motion: 5l moves half a second, 3j three videos.</p>
  {#each GROUPS as group (group.title)}
    <section>
      <h3>{group.title}</h3>
      <dl>
        {#each group.keys as [keys, does] (does)}
          <dt>
            {#each keys as key (key)}<kbd>{key}</kbd>{/each}
          </dt>
          <dd>{does}</dd>
        {/each}
      </dl>
    </section>
  {/each}
</Sheet>

<style>
  .lead {
    margin-bottom: 18px;
  }

  section + section {
    margin-top: 20px;
  }

  h3 {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: var(--step--1);
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  dl {
    display: grid;
    grid-template-columns: minmax(0, 11rem) minmax(0, 1fr);
    gap: 8px 16px;
    margin: 0;
    align-items: baseline;
  }

  dt {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  dd {
    margin: 0;
  }

  kbd {
    min-width: 24px;
    padding: 1px 7px;
    border: 1px solid var(--line-strong);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--bench);
    font: inherit;
    font-size: var(--step--1);
    font-weight: 600;
    text-align: center;
  }
</style>
