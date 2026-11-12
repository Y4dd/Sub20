// The Lite size test: is the browser's encoder accurate enough to promise
// "under the limit every time"? Development only (`npm run dev`, then open
// /lite-lab.html); it is not part of either build.

import "@fontsource-variable/hanken-grotesk/wght.css";
import "../app.css";
import "./lab.css";
import { canEncodeVideo } from "mediabunny";
import { bitrate, clock, megabytes, resolution } from "../lib/format";
import { DEFAULT_OPTIONS, planFor } from "../lib/mock";
import type { MediaInfo, Plan } from "../lib/types";
import { fit, probe, type Attempt, type FitSettings } from "../lib/web/fit";

interface Row {
  name: string;
  sourceBytes: number;
  source?: MediaInfo;
  plan?: Plan;
  attempts: Attempt[];
  under?: boolean;
  audio?: string | null;
  error?: string;
  notes?: string[];
  url?: string;
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const files = $<HTMLInputElement>("files");
const target = $<HTMLInputElement>("target");
const mode = $<HTMLSelectElement>("mode");
const hardware = $<HTMLSelectElement>("hardware");
const run = $<HTMLButtonElement>("run");
const copy = $<HTMLButtonElement>("copy");
const rows = $<HTMLTableSectionElement>("rows");
const summary = $<HTMLParagraphElement>("summary");

let results: Row[] = [];
let running = false;

async function describeBrowser() {
  const [avc, hw] = await Promise.all([
    canEncodeVideo("avc", { width: 1280, height: 720, bitrate: 2e6 }),
    canEncodeVideo("avc", { width: 1280, height: 720, bitrate: 2e6, hardwareAcceleration: "prefer-hardware" }),
  ]);
  const webcodecs = "VideoEncoder" in window;
  $("browser").textContent = !webcodecs
    ? "This browser has no WebCodecs, so Sub20 Lite can't run here."
    : `H.264 encoding: ${avc ? "yes" : "no"}, on the GPU: ${hw ? "yes" : "no"}. ${navigator.userAgent}`;
  return webcodecs && avc;
}

function render() {
  rows.replaceChildren(...results.map(rowElement));
  const finished = results.filter((r) => r.under !== undefined);
  const firsts = finished.map((r) => r.attempts[0].bytes / r.plan!.targetBytes!);
  const failed = results.filter((r) => r.error).length;
  if (results.length === 0) {
    summary.textContent = "";
    return;
  }
  const pct = (x: number) => `${(x * 100).toFixed(1)}%`;
  const parts = [
    `${finished.filter((r) => r.under).length} of ${finished.length} under the limit`,
    finished.length ? `first try lands at ${pct(Math.min(...firsts))}–${pct(Math.max(...firsts))} of the limit` : "",
    finished.length ? `${finished.filter((r) => r.attempts.length > 1).length} needed a retry` : "",
    failed ? `${failed} could not be encoded` : "",
    running ? "running…" : "",
  ];
  summary.textContent = parts.filter(Boolean).join(", ");
}

function cell(text: string, className = "") {
  const td = document.createElement("td");
  td.textContent = text;
  if (className) td.className = className;
  return td;
}

function rowElement(row: Row): HTMLTableRowElement {
  const tr = document.createElement("tr");
  const s = row.source;
  const p = row.plan;
  tr.append(cell(row.name, "name"));
  tr.append(
    cell(
      s
        ? `${resolution(s.displayWidth, s.displayHeight)} ${s.fps ?? "?"} fps, ${clock(s.duration)}, ${megabytes(row.sourceBytes)}, ${s.videoCodec ?? "?"}`
        : megabytes(row.sourceBytes),
    ),
  );
  tr.append(cell(p ? `${p.width}×${p.height}, ${bitrate(p.videoKbps)} + ${p.audio ? `${row.audio ?? p.audio} ${p.audioKbps}k` : "no sound"}` : ""));
  tr.append(
    cell(
      row.attempts
        .map((a) => `${bitrate(a.videoKbps)} → ${megabytes(a.bytes)} (${(a.bytes / p!.targetBytes! * 100).toFixed(1)}%)`)
        .join("\n"),
      "attempts",
    ),
  );
  if (row.error) {
    tr.append(cell(row.error, "bad"));
  } else if (row.under === undefined) {
    tr.append(cell(row.attempts.length || p ? "encoding…" : "reading…", "muted"));
  } else {
    const last = row.attempts[row.attempts.length - 1];
    const verdict = `${row.under ? "Under" : "Over"}, ${megabytes(last.bytes)}`;
    tr.append(cell([verdict, ...(row.notes ?? [])].join("\n"), row.under ? "ok result" : "bad result"));
  }
  const spent = row.attempts.reduce((t, a) => t + a.seconds, 0);
  tr.append(cell(spent && p ? `${(p.duration * row.attempts.length / spent).toFixed(1)}× real time` : ""));
  const link = document.createElement("td");
  if (row.url) {
    const a = document.createElement("a");
    a.href = row.url;
    a.download = row.name.replace(/\.[^.]+$/, "") + "-lite.mp4";
    a.textContent = "Save";
    link.append(a);
  }
  tr.append(link);
  return tr;
}

async function runAll(list: File[]) {
  running = true;
  run.disabled = true;
  for (const r of results) if (r.url) URL.revokeObjectURL(r.url);
  results = list.map((f) => ({ name: f.name, sourceBytes: f.size, attempts: [] }));
  render();
  const settings: FitSettings = { bitrateMode: mode.value as FitSettings["bitrateMode"], hardware: hardware.value as FitSettings["hardware"] };
  const options = { ...DEFAULT_OPTIONS, targetMb: Number(target.value) || 20 };

  for (const [i, file] of list.entries()) {
    const row = results[i];
    try {
      row.source = await probe(file);
      row.plan = planFor(row.source, null, options);
      render();
      const done = await fit(file, row.source, row.plan, settings, () => {});
      row.attempts = done.attempts;
      row.under = done.under;
      row.plan = done.plan;
      row.notes = done.notes;
      row.audio = done.audio;
      row.url = URL.createObjectURL(done.blob);
    } catch (error) {
      row.error = error instanceof Error ? error.message : String(error);
    }
    render();
  }
  running = false;
  run.disabled = false;
  copy.disabled = false;
  render();
}

function report(): string {
  const head = `Sub20 Lite size test, ${target.value} MB, ${mode.value} bitrate, ${hardware.value}\n${navigator.userAgent}\n`;
  const lines = results.map((r) => {
    const s = r.source;
    const tries = r.attempts.map((a) => `${a.videoKbps}k→${a.bytes}B ${a.seconds.toFixed(1)}s`).join(" | ");
    return [
      r.name,
      s ? `${s.displayWidth}x${s.displayHeight}@${s.fps} ${s.duration.toFixed(2)}s ${s.videoCodec}` : "",
      r.plan ? `plan ${r.plan.width}x${r.plan.height} ${r.plan.videoKbps}k` : "",
      tries,
      r.error ?? (r.under ? "UNDER" : "OVER"),
      ...(r.notes ?? []),
    ].join("; ");
  });
  return [head, ...lines, "", summary.textContent].join("\n");
}

$("controls").addEventListener("submit", (event) => {
  event.preventDefault();
  if (files.files?.length && !running) void runAll([...files.files]);
});
files.addEventListener("change", () => (run.disabled = !files.files?.length || running));
copy.addEventListener("click", () => void navigator.clipboard.writeText(report()));

// For scripted runs: `await lab.run(files)`, or `lab.run()` once the input has files.
Object.assign(window, {
  lab: {
    run: (list?: File[]) => runAll(list ?? [...(files.files ?? [])]),
    report,
    get results() {
      return results;
    },
  },
});

void describeBrowser().then((ok) => {
  if (!ok) files.disabled = true;
});
