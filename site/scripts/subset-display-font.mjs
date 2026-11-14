// Makes the display face's Latin file smaller: Bricolage Grotesque from
// @fontsource-variable, cut to the width and weight ranges the site uses
// (85-100% wide, weight 400-500) and to fontsource's own Latin characters.
// 78 KB becomes 55 KB, which took the home page's Lighthouse score on a phone
// from 93 to 96. The other scripts (latin-ext, Vietnamese) stay fontsource's.
//
// Run with `npm run fonts` after updating @fontsource-variable/bricolage-grotesque,
// or after the site starts using another width or weight for headings; then
// change the ranges here and in src/styles/fonts.css together.
import { readFileSync, writeFileSync, mkdirSync, copyFileSync } from "node:fs";
import subsetFont from "subset-font";

const pkg = new URL("../node_modules/@fontsource-variable/bricolage-grotesque/", import.meta.url);
const out = new URL("../src/assets/fonts/", import.meta.url);

// fontsource's Latin face: its file and its unicode-range.
const css = readFileSync(new URL("wdth.css", pkg), "utf8");
const face = css.split("@font-face").find((block) => block.includes("-latin-wdth-normal.woff2"));
const file = face.match(/url\(\.\/(files\/[^)]+)\)/)[1];
const range = face.match(/unicode-range:\s*([^;]+);/)[1];

const codes = [];
for (const part of range.split(",")) {
  const [from, to = from] = part.trim().replace(/^U\+/, "").split("-");
  for (let c = parseInt(from, 16); c <= parseInt(to, 16); c++) codes.push(c);
}

const source = readFileSync(new URL(file, pkg));
const subset = await subsetFont(source, String.fromCodePoint(...codes), {
  targetFormat: "woff2",
  variationAxes: { wdth: { min: 85, max: 100 }, wght: { min: 400, max: 500 } },
});

mkdirSync(out, { recursive: true });
writeFileSync(new URL("bricolage-grotesque-display-latin.woff2", out), subset);
copyFileSync(new URL("LICENSE", pkg), new URL("bricolage-grotesque-OFL.txt", out));
console.log(`${file}: ${source.length} bytes -> ${subset.length} (unicode-range ${range})`);
