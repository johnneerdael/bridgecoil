import {
  render,
  screen,
  fireEvent,
  cleanup,
  waitFor,
  within,
} from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import AppsPage from "./AppsPage.svelte";
import type { AppMetadata, Platform, Query } from "./client";
import type { Group, Level } from "./ratings";
import type { Device } from "../model";
afterEach(cleanup);
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
const app = (label: string, group: Group, level: Level | null = null) => ({
  package: `org.demo.${label.toLowerCase().replaceAll(" ", "")}`,
  label,
  icon: null,
  group,
  rating:
    group === "notRated" || level === null
      ? null
      : {
          level,
          source: "Oem",
          description: "",
          dependencies: [],
          neededBy: [],
        },
});
const order: Group[] = [
  "installed",
  "recommended",
  "advanced",
  "notRated",
  "expert",
  "unsafe",
];
function page(apps: AppMetadata[], q: { page: number; pageSize: number }) {
  return {
    items: apps.slice(q.page * q.pageSize, (q.page + 1) * q.pageSize),
    total: apps.length,
    page: q.page,
    pageSize: q.pageSize,
    groups: order
      .map((id) => ({ id, count: apps.filter((a) => a.group === id).length }))
      .filter((g) => g.count),
  };
}
function platform(apps: AppMetadata[]) {
  const query = vi.fn(async (_: unknown, q: Query) => {
    if (q.kind !== "packages") return {};
    const needle = q.search.toLowerCase();
    return page(
      apps.filter((a) => (a.label ?? "").toLowerCase().includes(needle)),
      q,
    );
  });
  return {
    query,
    ratingsStatus: vi.fn(async () => ({
      stored: true,
      commit: "a".repeat(40),
      committedAt: "2026-10-24T07:42:13Z",
      packages: 5382,
    })),
    ratingsUpdateIfDue: vi.fn(async () => false),
  } as unknown as Platform;
}
const open = (api: Platform) =>
  render(AppsPage, {
    device,
    api,
    onreview: vi.fn(),
    onbulk: vi.fn(),
    oninstall: vi.fn(),
  });
const mixed = [
  app("Mine", "installed"),
  app("Mine Rated", "installed", "expert"),
  app("Good One", "recommended", "recommended"),
  app("Good Two", "recommended", "recommended"),
  app("Unknown", "notRated"),
  app("Core", "unsafe", "unsafe"),
];
const headers = () =>
  screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent);
function scrollToEnd() {
  const list = screen.getByRole("region", { name: "Installed apps" });
  Object.defineProperties(list, {
    scrollHeight: { value: 2000, configurable: true },
    clientHeight: { value: 400, configurable: true },
    scrollTop: { value: 1500, writable: true, configurable: true },
  });
  return fireEvent.scroll(list);
}

it("shows headers with counts in order", async () => {
  open(platform(mixed));
  await screen.findByRole("button", { name: /Mine Rated/ });
  expect(headers().map((h) => h?.replace(/\s+/g, " ").trim())).toEqual([
    "Installed by you 2",
    "Recommended 2",
    "Not rated 1",
    "Unsafe 1",
  ]);
});

it("shows each row's rating badge", async () => {
  open(platform(mixed));
  const good = await screen.findByRole("button", { name: /Good One/ });
  expect(within(good).getByText("Recommended")).toBeTruthy();
  const unknown = screen.getByRole("button", { name: /^Unknown/ });
  expect(within(unknown).getByText("Not rated")).toBeTruthy();
});

it("keeps a rating badge on a rated app the user installed", async () => {
  open(platform(mixed));
  const rated = await screen.findByRole("button", { name: /Mine Rated/ });
  expect(within(rated).getByText("Expert")).toBeTruthy();
});

it("leaves an unrated app the user installed without a badge", async () => {
  open(platform(mixed));
  const mine = await screen.findByRole("button", { name: /^Mineorg/ });
  expect(within(mine).queryByText("Not rated")).toBeNull();
});

it("starts unsafe collapsed and expands it", async () => {
  open(platform(mixed));
  await screen.findByRole("button", { name: /Good One/ });
  expect(screen.queryByRole("button", { name: /^Core/ })).toBeNull();
  const header = screen.getByRole("button", { name: /Unsafe/ });
  expect(header.getAttribute("aria-expanded")).toBe("false");
  await fireEvent.click(header);
  expect(screen.getByRole("button", { name: /^Core/ })).toBeTruthy();
  expect(header.getAttribute("aria-expanded")).toBe("true");
});

it("stops paging at collapsed unsafe and resumes when expanded", async () => {
  const apps = [
    ...Array.from({ length: 32 }, (_, i) =>
      app(`Good ${i}`, "recommended", "recommended"),
    ),
    ...Array.from({ length: 40 }, (_, i) =>
      app(`Core ${i}`, "unsafe", "unsafe"),
    ),
  ];
  const api = platform(apps);
  open(api);
  await screen.findByRole("button", { name: /Good 31/ });
  await scrollToEnd();
  expect(api.query).toHaveBeenCalledTimes(1);
  await fireEvent.click(screen.getByRole("button", { name: /Unsafe/ }));
  await screen.findByRole("button", { name: /^Core 31/ });
  expect(api.query).toHaveBeenCalledTimes(2);
});

it("stops loading when the list ends at collapsed unsafe", async () => {
  const apps = [
    ...Array.from({ length: 32 }, (_, i) =>
      app(`Good ${i}`, "recommended", "recommended"),
    ),
    app("Core", "unsafe", "unsafe"),
  ];
  open(platform(apps));
  await screen.findByRole("button", { name: /Good 31/ });
  await scrollToEnd();
  expect(screen.queryByText("Reading more apps…")).toBeNull();
  expect(screen.getByRole("button", { name: /Unsafe/ })).toBeTruthy();
});

it("expands every group during a search", async () => {
  open(platform(mixed));
  await screen.findByRole("button", { name: /Good One/ });
  await fireEvent.input(screen.getByLabelText("Search apps"), {
    target: { value: "core" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Search" }));
  expect(await screen.findByRole("button", { name: /^Core/ })).toBeTruthy();
});

it("shows no stale rows when the filter changes mid-load", async () => {
  let release: (value: unknown) => void = () => {};
  const api = platform(mixed);
  (api.query as ReturnType<typeof vi.fn>).mockImplementationOnce(
    () => new Promise((resolve) => (release = resolve)),
  );
  open(api);
  await fireEvent.change(screen.getByLabelText("App filter"), {
    target: { value: "user" },
  });
  release(page([app("Stale", "recommended", "recommended")], {
    page: 0,
    pageSize: 32,
  }));
  await screen.findByRole("button", { name: /Mine Rated/ });
  expect(screen.queryByRole("button", { name: /Stale/ })).toBeNull();
});

it("shows the flat list with no headers without a stored list", async () => {
  const api = platform([]);
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    items: [{ package: "org.demo.flat", label: "Flat", icon: null }],
    total: 1,
    page: 0,
    pageSize: 32,
  });
  open(api);
  const row = await screen.findByRole("button", { name: /Flat/ });
  expect(screen.queryByRole("heading", { level: 3 })).toBeNull();
  expect(within(row).queryByText("Not rated")).toBeNull();
});

it("returns to the flat list once ratings are gone", async () => {
  const api = platform(mixed);
  open(api);
  await screen.findByRole("button", { name: /Good One/ });
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    items: [{ package: "org.demo.flat", label: "Flat", icon: null }],
    total: 1,
    page: 0,
    pageSize: 32,
  });
  const search = screen.getByRole("button", {
    name: "Search",
  }) as HTMLButtonElement;
  await waitFor(() => expect(search.disabled).toBe(false));
  await fireEvent.click(search);
  await screen.findByRole("button", { name: /Flat/ });
  await waitFor(() =>
    expect(screen.queryByRole("heading", { level: 3 })).toBeNull(),
  );
});

it("never preselects a group or offers select all", async () => {
  open(platform(mixed));
  await screen.findByRole("button", { name: /Good One/ });
  await fireEvent.click(screen.getByRole("button", { name: "Select" }));
  const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
  expect(boxes.length).toBeGreaterThan(0);
  expect(boxes.some((box) => box.checked)).toBe(false);
  expect(screen.queryByRole("button", { name: /select all/i })).toBeNull();
});

it("shows collapsed unsafe rows when selecting only after expanding", async () => {
  open(platform(mixed));
  await screen.findByRole("button", { name: /Good One/ });
  await fireEvent.click(screen.getByRole("button", { name: "Select" }));
  expect(screen.queryByLabelText("Select Core")).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: /Unsafe/ }));
  expect(screen.getByLabelText("Select Core")).toBeTruthy();
});
