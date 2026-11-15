// Where Sub20 Lite keeps large results: the origin private file system, a
// folder of the browser's own on this device, invisible to the user. A
// 500 MB or 1 GB result held in memory can take a phone's tab down; written
// here it costs disk, and its `File` is read from disk when it is played,
// downloaded or shared. Small results stay in memory, where the MP4's index
// can go at the front for players that start before the file has arrived.

/** Results expected to be at least this large are written to disk. */
export const DISK_FROM_BYTES = 64 * 1024 * 1024;

const FOLDER = "results";

/** The results folder, or `null` where the browser can't write files there (older Safari). */
export async function resultsFolder(): Promise<FileSystemDirectoryHandle | null> {
  try {
    const root = await navigator.storage.getDirectory();
    const folder = await root.getDirectoryHandle(FOLDER, { create: true });
    // Safari had the folder before it had `createWritable`, which the encoder needs.
    if (!("createWritable" in FileSystemFileHandle.prototype)) return null;
    return folder;
  } catch {
    return null;
  }
}

/** A new file in the results folder, open for writing; `drop` deletes it. */
export async function newResult(folder: FileSystemDirectoryHandle) {
  const name = `${crypto.randomUUID()}.mp4`;
  const handle = await folder.getFileHandle(name, { create: true });
  const writable = await handle.createWritable();
  return {
    writable,
    read: () => handle.getFile(),
    drop: () => folder.removeEntry(name).catch(() => {}),
  };
}

/**
 * Empties the results folder. Results live for one visit (the page keeps
 * them in a map, by made-up path), so what an earlier visit left is out of
 * reach and only takes space.
 */
export async function clearResults(): Promise<void> {
  try {
    const root = await navigator.storage.getDirectory();
    await root.removeEntry(FOLDER, { recursive: true });
  } catch {
    // Nothing there, or no private file system: nothing to clear.
  }
}
