import { render, screen, fireEvent, cleanup } from "@testing-library/svelte";
import { afterEach, beforeAll, it, expect, vi } from "vitest";
import FilesTool from "./FilesTool.svelte";
import ToolsPage from "./ToolsPage.svelte";
import DevicePage from "./DevicePage.svelte";
import {
  breadcrumbs,
  entryIcon,
  entryNote,
  folderSummary,
  formatSize,
  nameProblem,
  parentPath,
} from "./files";
import { settingChoices, settingLabel } from "./settings";
import type { FileEntry, FileListing, Platform, Query } from "./client";
import type { Device } from "../model";
afterEach(cleanup);
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
});
const device: Device = {
  id: "session",
  deviceId: "device",
  profileId: 0,
  serial: "serial",
  build: "build",
  manufacturer: "TCL",
  model: "Smart TV Pro",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: [],
};
const entry = (over: Partial<FileEntry>): FileEntry => ({
  name: "name",
  path: "/name",
  kind: "folder",
  link: false,
  canOpen: true,
  readable: true,
  size: 4096,
  modified: 1700000000,
  ...over,
});
const listing = (
  path: string,
  entries: FileEntry[],
  status: FileListing["status"] = "ok",
): FileListing => ({
  path,
  status,
  writable: true,
  entries,
  total: entries.length,
  truncated: false,
});
function platform(folders: Record<string, FileListing>) {
  const query = vi.fn(async (_: unknown, q: Query) =>
    q.kind === "files"
      ? (folders[q.path] ?? listing(q.path, [], "missing"))
      : {},
  );
  return {
    query,
    prepare: vi.fn(async () => ({ id: "review" })),
    chooseUpload: vi.fn(async () => ({
      id: "upload",
      name: "Film.mkv",
      size: 2048,
    })),
    upload: vi.fn(async (_s, _i, _f, _n, onprogress) => onprogress(2048)),
    cancelUpload: vi.fn(async () => {}),
  } as unknown as Platform;
}
const tool = (api: Platform, onreview = vi.fn()) =>
  render(FilesTool, { device, api, reload: 0, onreview });

it("names breadcrumbs for people and goes up from any depth", () => {
  expect(breadcrumbs("/sdcard/Movies/Kids").map((c) => c.label)).toEqual([
    "Internal storage",
    "Movies",
    "Kids",
  ]);
  expect(breadcrumbs("/storage/emulated/0/Music")[1].path).toBe(
    "/storage/emulated/0/Music",
  );
  expect(breadcrumbs("/").map((c) => c.path)).toEqual(["/"]);
  expect(breadcrumbs("/system/bin").map((c) => c.path)).toEqual([
    "/",
    "/system",
    "/system/bin",
  ]);
  expect(breadcrumbs("/sdcardish")[0].label).toBe("Device");
  expect(parentPath("/sdcard")).toBe("/");
  expect(parentPath("/sdcard/Movies")).toBe("/sdcard");
  expect(parentPath("/")).toBe("/");
});

it("describes entries by what can be done with them", () => {
  const locked = entry({ canOpen: false });
  expect(entryIcon(locked)).toBe("lock");
  expect(entryNote(locked)).toBe("No access");
  expect(entryNote(entry({ kind: "denied", canOpen: false }))).toBe("No access");
  expect(entryNote(entry({ kind: "brokenLink", canOpen: false }))).toBe(
    "Link target unavailable",
  );
  const movie = entry({ name: "Film.MKV", kind: "file", canOpen: false });
  expect(entryIcon(movie)).toBe("video");
  expect(entryNote(movie)).toContain("4.0 KB");
  expect(
    entryNote({ ...movie, readable: false }).endsWith("No read access"),
  ).toBe(true);
  expect(formatSize(512)).toBe("512 B");
  expect(formatSize(1536)).toBe("1.5 KB");
  expect(formatSize(5 * 1024 ** 3)).toBe("5.0 GB");
  expect(folderSummary([entry({}), movie, movie])).toBe("1 folder · 2 files");
});

it("opens folders, keeps denied rows inert and returns to the parent", async () => {
  const api = platform({
    "/sdcard": listing("/sdcard", [
      entry({ name: "Movies", path: "/sdcard/Movies" }),
      entry({ name: "Secret", path: "/sdcard/Secret", canOpen: false }),
      entry({ name: "a.txt", path: "/sdcard/a.txt", kind: "file", canOpen: false }),
    ]),
    "/sdcard/Movies": listing("/sdcard/Movies", []),
    "/": listing("/", [entry({ name: "sdcard", path: "/sdcard", link: true })]),
  });
  tool(api);
  await fireEvent.click(await screen.findByRole("button", { name: /^Movies\S/ }));
  expect(await screen.findByText("This folder is empty")).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Parent folder" }));
  expect(await screen.findByText("Secret")).toBeTruthy();
  expect(screen.queryByRole("button", { name: /^Secret/ })).toBeNull();
  expect(screen.queryByRole("button", { name: /^a\.txt/ })).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Parent folder" }));
  expect(await screen.findByRole("button", { name: /^sdcard/ })).toBeTruthy();
  expect(
    (screen.getByRole("button", { name: "Parent folder" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});

it("explains a denied folder and keeps the way out", async () => {
  const api = platform({
    "/sdcard": listing("/sdcard", []),
    "/data": listing("/data", [], "denied"),
  });
  tool(api);
  await screen.findByText("This folder is empty");
  await fireEvent.click(screen.getByRole("button", { name: "Go to a path" }));
  await fireEvent.input(screen.getByLabelText("Device path"), {
    target: { value: "/data/" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Open" }));
  expect(await screen.findByText("No access to this folder")).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.getByRole("button", { name: "Parent folder" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Internal storage" })).toBeTruthy();
});

it("hides dotfiles until asked and explains a folder of only hidden items", async () => {
  const api = platform({
    "/sdcard": listing("/sdcard", [
      entry({ name: ".cache", path: "/sdcard/.cache" }),
    ]),
  });
  tool(api);
  expect(await screen.findByText("Only hidden items here")).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Show hidden (1)" }),
  );
  expect(screen.getByRole("button", { name: /^\.cache/ })).toBeTruthy();
});

const folderWithFile = () =>
  listing("/sdcard", [
    entry({ name: "Movies", path: "/sdcard/Movies" }),
    entry({
      name: "a.txt",
      path: "/sdcard/a.txt",
      kind: "file",
      canOpen: false,
    }),
  ]);

it("reviews a delete instead of deleting directly", async () => {
  const api = platform({ "/sdcard": folderWithFile() });
  const onreview = vi.fn();
  tool(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Actions for a.txt" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Delete" }));
  await vi.waitFor(() => expect(onreview).toHaveBeenCalledOnce());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "fileDelete",
    path: "/sdcard/a.txt",
  });
});

it("renames only to a valid, unused name", async () => {
  const api = platform({ "/sdcard": folderWithFile() });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Actions for a.txt" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Rename" }));
  const name = await screen.findByLabelText("Name");
  const go = screen.getByRole("button", { name: "Continue" }) as HTMLButtonElement;
  expect(go.disabled).toBe(true);
  for (const value of ["Movies", "a/b", "..", " "]) {
    await fireEvent.input(name, { target: { value } });
    expect(go.disabled).toBe(true);
  }
  await fireEvent.input(name, { target: { value: "b.txt" } });
  expect(go.disabled).toBe(false);
  await fireEvent.click(go);
  await vi.waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "fileMove",
      from: "/sdcard/a.txt",
      to: "/sdcard/b.txt",
    }),
  );
});

it("moves by browsing to the destination", async () => {
  const api = platform({
    "/sdcard": folderWithFile(),
    "/sdcard/Movies": listing("/sdcard/Movies", []),
  });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Actions for a.txt" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Move" }));
  const here = screen.getByRole("button", {
    name: "Move here",
  }) as HTMLButtonElement;
  expect(here.disabled).toBe(true);
  await fireEvent.click(screen.getByRole("button", { name: /^Movies\S/ }));
  await screen.findByText("This folder is empty");
  const target = screen.getByRole("button", {
    name: "Move here",
  }) as HTMLButtonElement;
  expect(target.disabled).toBe(false);
  await fireEvent.click(target);
  await vi.waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "fileMove",
      from: "/sdcard/a.txt",
      to: "/sdcard/Movies/a.txt",
    }),
  );
});

it("offers no changes in a folder ADB can't write", async () => {
  const api = platform({
    "/sdcard": { ...folderWithFile(), writable: false },
  });
  tool(api);
  expect(await screen.findByText("This folder is read-only for ADB.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: /Actions for/ })).toBeNull();
  expect(screen.queryByRole("button", { name: /Upload file/ })).toBeNull();
  expect(screen.queryByRole("button", { name: /New folder/ })).toBeNull();
});

it("shows a refused action without losing the folder", async () => {
  const api = platform({ "/sdcard": folderWithFile() });
  (api.prepare as ReturnType<typeof vi.fn>).mockRejectedValue({
    kind: "protected",
    message: "This folder is protected.",
  });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Actions for Movies" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Delete" }));
  expect((await screen.findByRole("alert")).textContent).toContain("protected");
  expect(screen.getByRole("button", { name: /^Movies\S/ })).toBeTruthy();
});

it("uploads to the open folder and refreshes it", async () => {
  const api = platform({ "/sdcard": folderWithFile() });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: /Upload file/ }),
  );
  const name = (await screen.findByLabelText(
    "Name on the TV",
  )) as HTMLInputElement;
  expect(name.value).toBe("Film.mkv");
  await fireEvent.input(name, { target: { value: "a.txt" } });
  const go = screen.getByRole("button", { name: "Upload" }) as HTMLButtonElement;
  expect(go.disabled).toBe(true);
  await fireEvent.input(name, { target: { value: "Film 2.mkv" } });
  await fireEvent.click(go);
  expect(await screen.findByText("Uploaded Film 2.mkv.")).toBeTruthy();
  expect(api.upload).toHaveBeenCalledWith(
    device.id,
    "upload",
    "/sdcard",
    "Film 2.mkv",
    expect.any(Function),
  );
  expect((api.query as ReturnType<typeof vi.fn>).mock.calls.length).toBe(2);
});

it("keeps a cancelled upload quiet", async () => {
  const api = platform({ "/sdcard": folderWithFile() });
  (api.upload as ReturnType<typeof vi.fn>).mockRejectedValue({
    kind: "cancelled",
    message: "Upload cancelled.",
  });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: /Upload file/ }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Upload" }));
  await vi.waitFor(() =>
    expect(screen.queryByLabelText("Name on the TV")).toBeNull(),
  );
  expect(screen.queryByRole("alert")).toBeNull();
});

it("flags names shared storage refuses, only on shared storage", () => {
  for (const name of ['a"b', "a*b", "a:b", "a<b", "a>b", "a?b", "a\\b", "a|b"]) {
    expect(nameProblem("/sdcard/Movies", name), name).toContain(
      "can't contain",
    );
    expect(nameProblem("/storage/emulated/0", name), name).toBeTruthy();
    expect(nameProblem("/data/local/tmp", name), name).toBeNull();
  }
  expect(nameProblem("/sdcard", "it's a tést $(id) 漢字 #1.txt")).toBeNull();
  expect(nameProblem("/sdcardish", "a:b")).toBeNull();
});

it("explains a reserved character before review", async () => {
  const api = platform({ "/sdcard": folderWithFile() });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: /New folder/ }),
  );
  await fireEvent.input(await screen.findByLabelText("Name"), {
    target: { value: "a:b" },
  });
  const go = screen.getByRole("button", {
    name: "Continue",
  }) as HTMLButtonElement;
  expect(go.disabled).toBe(true);
  expect(screen.getByText(/can't contain/)).toBeTruthy();
});

it("blocks an upload with a reserved character", async () => {
  const api = platform({ "/sdcard": folderWithFile() });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: /Upload file/ }),
  );
  await fireEvent.input(await screen.findByLabelText("Name on the TV"), {
    target: { value: "Film: One.mkv" },
  });
  expect(
    (screen.getByRole("button", { name: "Upload" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(screen.getByText(/can't contain/)).toBeTruthy();
});

it("uses the TV's path after opening a typed one", async () => {
  const api = platform({
    "/sdcard": folderWithFile(),
    "/sdcard//Download": listing("/sdcard/Download", []),
    "/sdcard/Download": listing("/sdcard/Download", []),
  });
  tool(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Go to a path" }),
  );
  await fireEvent.input(screen.getByLabelText("Device path"), {
    target: { value: "/sdcard//Download" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Open" }));
  await screen.findByText("This folder is empty");
  await fireEvent.click(screen.getByRole("button", { name: /New folder/ }));
  await fireEvent.input(await screen.findByLabelText("Name"), {
    target: { value: "New" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Continue" }));
  await vi.waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "fileCreateFolder",
      path: "/sdcard/Download/New",
    }),
  );
});

it("matches Android's 1.0 to the 1 option and labels unset values", () => {
  expect(settingChoices("window_animation_scale", "1.0")).toEqual({
    selected: "1",
    options: ["0", "0.5", "1", "1.5", "2", "5", "10"],
  });
  const unusual = settingChoices("window_animation_scale", "0.75");
  expect(unusual.selected).toBe("0.75");
  expect(unusual.options[0]).toBe("0.75");
  const unset = settingChoices("window_animation_scale", "null");
  expect(unset.selected).toBe("null");
  expect(settingLabel(unset.selected)).toBe("Not set");
  expect(settingChoices("screen_brightness", null).selected).toBe("");
  expect(settingLabel(null)).toBe("Unknown");
});

it("shows the TV's current setting in its selector", async () => {
  const api = {
    query: vi.fn(async () => [
      {
        namespace: "global",
        key: "window_animation_scale",
        name: "Window animation",
        value: "1.0",
        error: null,
      },
    ]),
  } as unknown as Platform;
  render(ToolsPage, {
    device,
    api,
    onreview: () => {},
    onbulk: () => {},
    oninstall: () => {},
  });
  await fireEvent.click(screen.getByRole("button", { name: "Next" }));
  await fireEvent.click(screen.getByRole("button", { name: "Next" }));
  await fireEvent.click(await screen.findByRole("button", { name: /^Settings/ }));
  const select = (await screen.findByLabelText(
    /Window animation/,
  )) as HTMLSelectElement;
  expect(select.value).toBe("1");
});

it("does not offer connection setup on a connected device's page", async () => {
  const api = { query: vi.fn(async () => []) } as unknown as Platform;
  render(DevicePage, {
    device,
    api,
    ondevice: () => {},
    onnavigate: () => {},
  });
  expect(screen.getByText("Manage apps")).toBeTruthy();
  expect(screen.queryByText("Connection setup")).toBeNull();
});
