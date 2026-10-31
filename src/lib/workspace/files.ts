import type { FileEntry } from "./client";

export const internalStorage = "/sdcard";
export const storageRoots = [
  { path: internalStorage, label: "Internal storage" },
  { path: `${internalStorage}/Download`, label: "Downloads" },
  { path: `${internalStorage}/Movies`, label: "Movies" },
  { path: `${internalStorage}/Music`, label: "Music" },
  { path: `${internalStorage}/Pictures`, label: "Pictures" },
  { path: "/", label: "Device" },
];
const storagePrefixes = [internalStorage, "/storage/emulated/0"];

export const parentPath = (path: string) =>
  path.replace(/\/[^/]*$/, "") || "/";

export const joinPath = (folder: string, name: string) =>
  `${folder === "/" ? "" : folder}/${name}`;

/** Mirrors `valid_name` in the backend, so a bad name is caught before review. */
export const validName = (name: string) =>
  name.trim().length > 0 &&
  new TextEncoder().encode(name).length <= 255 &&
  name !== "." &&
  name !== ".." &&
  !name.includes("/");

export function breadcrumbs(path: string) {
  const prefix = storagePrefixes.find(
    (p) => path === p || path.startsWith(`${p}/`),
  );
  const crumbs = [
    { label: prefix ? "Internal storage" : "Device", path: prefix ?? "/" },
  ];
  let current = prefix ?? "";
  for (const part of path.slice(current.length).split("/").filter(Boolean)) {
    current += `/${part}`;
    crumbs.push({ label: part, path: current });
  }
  return crumbs;
}

export const visibleEntries = (entries: FileEntry[], showHidden: boolean) =>
  showHidden ? entries : entries.filter((e) => !e.name.startsWith("."));

const count = (n: number, noun: string) => `${n} ${noun}${n === 1 ? "" : "s"}`;

export function folderSummary(entries: FileEntry[]) {
  const folders = entries.filter((e) => e.kind === "folder").length;
  return `${count(folders, "folder")} · ${count(entries.length - folders, "file")}`;
}

export function formatSize(bytes: number) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${unit === 0 || value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`;
}

const iconsByExtension: Record<string, string> = {
  apk: "apps",
  apks: "apps",
  xapk: "apps",
  jpg: "image",
  jpeg: "image",
  png: "image",
  gif: "image",
  webp: "image",
  mp4: "video",
  mkv: "video",
  webm: "video",
  avi: "video",
  mov: "video",
  mp3: "audio",
  flac: "audio",
  ogg: "audio",
  m4a: "audio",
  wav: "audio",
  zip: "archive",
  gz: "archive",
  tar: "archive",
  "7z": "archive",
  rar: "archive",
};

export function entryIcon(entry: FileEntry) {
  if (entry.kind === "denied" || (entry.kind === "folder" && !entry.canOpen))
    return "lock";
  if (entry.kind === "folder") return "folder";
  if (entry.kind === "brokenLink") return "link";
  return iconsByExtension[entry.name.split(".").pop()!.toLowerCase()] ?? "file";
}

/** Says why an entry can't be opened rather than letting a tap fail. */
export function entryNote(entry: FileEntry) {
  if (entry.kind === "denied") return "No access";
  if (entry.kind === "brokenLink") return "Link target unavailable";
  if (entry.kind === "folder" && !entry.canOpen) return "No access";
  const date =
    entry.modified === null
      ? null
      : new Date(entry.modified * 1000).toLocaleDateString();
  const details = [
    entry.kind === "file" && entry.size !== null ? formatSize(entry.size) : null,
    entry.kind === "other" ? "System file" : null,
    date,
    entry.link ? "Link" : null,
    entry.kind === "file" && !entry.readable ? "No read access" : null,
  ];
  return details.filter(Boolean).join(" · ");
}
