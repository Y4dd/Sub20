// The real backend: Rust commands over Tauri's IPC.

import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import type { Added, Backend, Frame, JobEvent } from "./types";

/** File types offered in the "choose files" dialog. */
const VIDEO_EXTENSIONS = [
  "mp4", "mkv", "mov", "avi", "webm", "m4v", "wmv", "flv", "mpeg", "mpg", "ts", "m2ts", "3gp", "3g2", "mts",
  "mxf", "ogv", "vob",
];

/** Preview URLs handed out by the app for platforms that need them. */
const playUrls = new Map<string, string>();

/** Thumbnail files by the URL the window shows them under; dragging needs the file. */
const thumbFiles = new Map<string, string>();

const agent = navigator.userAgent;
const frame: Frame = agent.includes("Windows") ? "custom" : agent.includes("Mac") ? "overlay" : "native";

export const tauriBackend: Backend = {
  demo: false,
  frame,
  minimize: () => getCurrentWindow().minimize(),
  toggleMaximize: () => getCurrentWindow().toggleMaximize(),
  closeWindow: () => getCurrentWindow().close(),
  onMaximized: async (handler) => {
    const window = getCurrentWindow();
    handler(await window.isMaximized());
    return window.onResized(async () => handler(await window.isMaximized()));
  },
  bootstrap: () => invoke("bootstrap"),
  ffmpegStatus: () => invoke("ffmpeg_status"),
  addVideos: async (paths) => {
    const added = await invoke<Added[]>("add_videos", { paths });
    for (const item of added) if (item.playUrl) playUrls.set(item.path, item.playUrl);
    return added;
  },
  filmstrip: async (path, duration, count, onThumb) => {
    const thumbUrl = (file: string) => {
      const url = convertFileSrc(file);
      thumbFiles.set(url, file);
      return url;
    };
    const onThumbChannel = new Channel<{ index: number; path: string }>();
    onThumbChannel.onmessage = ({ index, path }) => onThumb(index, thumbUrl(path));
    const files = await invoke<string[]>("filmstrip", { path, duration, count, onThumb: onThumbChannel });
    return files.map(thumbUrl);
  },
  videoUrl: (path) => playUrls.get(path) ?? convertFileSrc(path),
  previewFrame: (path, at) => invoke("preview_frame", { path, at }),
  estimate: (info, trim, options) => invoke("estimate", { info, trim, options }),
  compress: (jobs) => invoke("compress", { jobs }),
  cancel: async (id) => {
    await invoke("cancel", { id });
  },
  cancelAll: () => invoke("cancel_all"),
  saveSettings: (settings) => invoke("save_settings", { settings }),
  encoders: (refresh) => invoke("encoders", { refresh }),
  historyList: () => invoke("history_list"),
  historyRemove: (id) => invoke("history_remove", { id }),
  historyClear: () => invoke("history_clear"),
  watchStatus: () => invoke("watch_status"),
  clipFolders: () => invoke("clip_folders"),
  reveal: (path) => invoke("reveal", { path }),
  openFile: (path) => invoke("open_file", { path }),
  resultUrl: async (path) => (await invoke<string | null>("result_url", { path })) ?? convertFileSrc(path),
  copyFile: (path) => invoke("copy_file", { path }),
  renameResult: (path, name) => invoke("rename_result", { path, name }),
  checkUpdate: () => invoke("check_update"),
  installUpdate: () => invoke("install_update"),
  copyDiagnostics: (log) => invoke("copy_diagnostics", { log }),
  dragOut: async (path, thumb) => {
    const icon = (thumb && thumbFiles.get(thumb)) ?? thumbFiles.values().next().value;
    if (!icon) return;
    await startDrag({ item: [path], icon });
  },
  pickVideos: async () => {
    const picked = await open({
      multiple: true,
      title: "Choose videos",
      filters: [{ name: "Videos", extensions: VIDEO_EXTENSIONS }],
    });
    if (!picked) return [];
    return Array.isArray(picked) ? picked : [picked];
  },
  pickFolder: async () => {
    const picked = await open({ directory: true, multiple: false, title: "Choose a folder" });
    return typeof picked === "string" ? picked : null;
  },
  onJob: (handler) => listen<JobEvent>("job", (event) => handler(event.payload)),
  onOpened: (handler) => listen<string[]>("opened", (event) => handler(event.payload)),
  onDrop: (handler) =>
    getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "drop") handler({ type: "drop", paths: payload.paths });
      else if (payload.type === "leave") handler({ type: "leave" });
      else handler({ type: "over" });
    }),
};
