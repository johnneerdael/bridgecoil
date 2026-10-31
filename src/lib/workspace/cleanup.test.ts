import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import CleanupTool from "./CleanupTool.svelte";
import type { AppMetadata, Platform } from "./client";
import type { Device } from "../model";
afterEach(cleanup);
const device: Device = {
  id: "session",
  deviceId: "device",
  profileId: 10,
  serial: "serial",
  build: "build",
  manufacturer: "Vendor",
  model: "TV",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: [],
};
const app = (name: string, group: AppMetadata["group"]): AppMetadata => ({
  package: `com.vendor.${name}`,
  label: name,
  icon: null,
  group,
  rating: {
    level: group === "recommended" ? "recommended" : "expert",
    source: null,
    description: `${name} does things`,
    dependencies: [],
    neededBy: [],
  },
});
type Pages = Record<string, AppMetadata[]>;
function platform(pages: Pages, stored = true) {
  return {
    query: vi.fn(async (_t, q: { filter: string; page: number; pageSize: number }) => {
      const all = pages[q.filter] ?? [];
      return {
        items: all.slice(q.page * q.pageSize, (q.page + 1) * q.pageSize),
        total: all.length,
        page: q.page,
        pageSize: q.pageSize,
      };
    }),
    ratingsStatus: vi.fn(async () => ({
      stored,
      commit: null,
      committedAt: null,
      packages: stored ? 100 : 0,
    })),
    ratingsUpdate: vi.fn(async () => {
      stored = true;
      return true;
    }),
    prepareBulk: vi.fn(async () => ({ id: "bulk" })),
  } as unknown as Platform;
}
const open = (api: Platform, onbulk = vi.fn()) =>
  render(CleanupTool, { device, api, reload: 0, onbulk });

it("offers only recommended apps that are still on, all ticked", async () => {
  const api = platform({
    system: [app("ads", "recommended"), app("offer", "recommended"), app("core", "expert")],
    disabled: [app("offer", "recommended")],
  });
  open(api);
  const ads = await screen.findByRole("checkbox", { name: "Turn off ads" });
  expect((ads as HTMLInputElement).checked).toBe(true);
  expect(screen.queryByRole("checkbox", { name: "Turn off offer" })).toBeNull();
  expect(screen.queryByText(/core/)).toBeNull();
  expect(screen.getByText("ads does things")).toBeTruthy();
});

it("disables through a review of what stayed ticked", async () => {
  const api = platform({
    system: [app("ads", "recommended"), app("demo", "recommended")],
    disabled: [],
  });
  const onbulk = vi.fn();
  open(api, onbulk);
  await fireEvent.click(await screen.findByRole("checkbox", { name: "Turn off demo" }));
  await fireEvent.click(screen.getByRole("button", { name: "Review 1 app" }));
  await waitFor(() => expect(onbulk).toHaveBeenCalled());
  expect(api.prepareBulk).toHaveBeenCalledWith(
    expect.anything(),
    ["com.vendor.ads"],
    "disable",
  );
});

it("keeps to what one review can hold and says the rest can follow", async () => {
  const many = Array.from({ length: 40 }, (_, i) => app(`app${i}`, "recommended"));
  const api = platform({ system: many, disabled: [] });
  open(api);
  await screen.findByRole("button", { name: "Review 32 apps" });
  expect(screen.getByText(/8 more/)).toBeTruthy();
});

it("reads past the first page of recommended apps", async () => {
  const many = Array.from({ length: 33 }, (_, i) => app(`app${i}`, "recommended"));
  const api = platform({ system: many, disabled: [] });
  open(api);
  expect(await screen.findByRole("checkbox", { name: "Turn off app32" })).toBeTruthy();
});

it("lets everything turned off be turned back on", async () => {
  const api = platform({ system: [], disabled: [app("ads", "recommended")] });
  const onbulk = vi.fn();
  open(api, onbulk);
  await fireEvent.click(await screen.findByRole("button", { name: "Turn 1 back on" }));
  await waitFor(() => expect(onbulk).toHaveBeenCalled());
  expect(api.prepareBulk).toHaveBeenCalledWith(
    expect.anything(),
    ["com.vendor.ads"],
    "enable",
  );
});

it("says when nothing is left to clean up", async () => {
  open(platform({ system: [app("core", "expert")], disabled: [] }));
  expect(await screen.findByText(/nothing left/i)).toBeTruthy();
});

it("asks for the ratings before suggesting anything", async () => {
  const api = platform({ system: [app("ads", "recommended")], disabled: [] }, false);
  open(api);
  await fireEvent.click(await screen.findByRole("button", { name: "Download ratings" }));
  await waitFor(() => expect(api.ratingsUpdate).toHaveBeenCalled());
  expect(await screen.findByRole("checkbox", { name: "Turn off ads" })).toBeTruthy();
});
