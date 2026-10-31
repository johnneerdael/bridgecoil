import {
  render,
  screen,
  fireEvent,
  cleanup,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, beforeAll, it, expect, vi } from "vitest";
import AppsPage from "./AppsPage.svelte";
import type { Platform } from "./client";
import type { RatingsStatus } from "./ratings";
import type { Device } from "../model";
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
});
afterEach(() => {
  cleanup();
  localStorage.clear();
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
const none: RatingsStatus = {
  stored: false,
  commit: null,
  committedAt: null,
  packages: 0,
};
const stored: RatingsStatus = {
  stored: true,
  commit: "5c179a04daf55afe51e35ed2aeaa60c9b07a03d8",
  committedAt: "2026-10-24T07:42:13Z",
  packages: 5382,
};
function platform(status: RatingsStatus) {
  const state = { status };
  return {
    state,
    api: {
      query: vi.fn(async () => ({
        items: [{ package: "org.demo.app", label: "App", icon: null }],
        total: 1,
        page: 0,
        pageSize: 32,
      })),
      ratingsStatus: vi.fn(async () => state.status),
      ratingsUpdate: vi.fn(async () => {
        state.status = stored;
        return true;
      }),
      ratingsUpdateIfDue: vi.fn(async () => false),
      ratingsRemove: vi.fn(async () => {
        state.status = none;
      }),
    },
  };
}
const open = (api: unknown) =>
  render(AppsPage, {
    device,
    api: api as Platform,
    onreview: vi.fn(),
    onbulk: vi.fn(),
    oninstall: vi.fn(),
  });
const card = () => screen.queryByText("Get community ratings");

it("offers ratings only when none are stored", async () => {
  const empty = open(platform(none).api);
  expect(await screen.findByText("Get community ratings")).toBeTruthy();
  empty.unmount();
  open(platform(stored).api);
  await screen.findByRole("button", { name: /^App/ });
  await screen.findByRole("button", { name: "Ratings" });
  expect(card()).toBeNull();
});

it("downloads nothing until asked", async () => {
  const { api } = platform(none);
  open(api);
  await screen.findByText("Get community ratings");
  expect(api.ratingsUpdate).not.toHaveBeenCalled();
});

it("names the source and GitHub on the card", async () => {
  open(platform(none).api);
  const text = (await screen.findByText("Get community ratings")).closest(
    "aside",
  )?.textContent;
  expect(text).toContain("Universal Android Debloater");
  expect(text).toContain("GitHub");
});

it("reloads the list after a download", async () => {
  const { api } = platform(none);
  open(api);
  await screen.findByText("Get community ratings");
  await fireEvent.click(screen.getByRole("button", { name: "Download" }));
  await waitFor(() => expect(card()).toBeNull());
  await waitFor(() => expect(api.query).toHaveBeenCalledTimes(2));
});

it("keeps the card away after dismissing it", async () => {
  const first = open(platform(none).api);
  await screen.findByText("Get community ratings");
  await fireEvent.click(screen.getByRole("button", { name: "Not now" }));
  expect(card()).toBeNull();
  first.unmount();
  open(platform(none).api);
  await screen.findByRole("button", { name: "Ratings" });
  expect(card()).toBeNull();
});

it("opens the sheet from the summary line after dismissal", async () => {
  localStorage.setItem("bridgecoil.ratingsCardDismissed", "1");
  open(platform(none).api);
  await fireEvent.click(await screen.findByRole("button", { name: "Ratings" }));
  const download = await screen.findByRole("button", {
    name: "Download ratings",
  });
  expect(download).toBeTruthy();
});

it("shows a retry after a failed download and keeps apps usable", async () => {
  const { api } = platform(none);
  api.ratingsUpdate.mockRejectedValueOnce({
    kind: "ratingsNetwork",
    message: "Couldn't reach GitHub for the ratings list.",
  });
  open(api);
  await screen.findByText("Get community ratings");
  await fireEvent.click(screen.getByRole("button", { name: "Download" }));
  expect((await screen.findByRole("alert")).textContent).toContain("GitHub");
  expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  expect(screen.getByRole("button", { name: /^App/ })).toBeTruthy();
});

it("shows the source, commit and date in the sheet", async () => {
  open(platform(stored).api);
  await fireEvent.click(await screen.findByRole("button", { name: "Ratings" }));
  const dialog = await screen.findByRole("dialog", { name: "Ratings" });
  expect(dialog.textContent).toContain("5c179a0");
  expect(dialog.textContent).toContain("2026");
  expect(dialog.textContent).toContain("mostly based on phones");
  expect(dialog.textContent).toContain(
    "universal-android-debloater-next-generation",
  );
  expect(dialog.querySelector("a")).toBeNull();
});

it("reports a failed update check", async () => {
  const { api } = platform(stored);
  api.ratingsUpdate.mockRejectedValueOnce({
    kind: "ratingsRejected",
    message: "This ratings update looks wrong, so the current list was kept.",
  });
  open(api);
  await fireEvent.click(await screen.findByRole("button", { name: "Ratings" }));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Check for update" }),
  );
  expect((await screen.findByRole("alert")).textContent).toContain(
    "current list was kept",
  );
});

it("returns to the card after removing ratings", async () => {
  const { api } = platform(stored);
  open(api);
  await fireEvent.click(await screen.findByRole("button", { name: "Ratings" }));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remove ratings" }),
  );
  await waitFor(() => expect(api.ratingsRemove).toHaveBeenCalledOnce());
  expect(await screen.findByText("Get community ratings")).toBeTruthy();
});

it("runs the daily check once and stays silent on failure", async () => {
  const { api } = platform(stored);
  api.ratingsUpdateIfDue.mockRejectedValueOnce({
    kind: "ratingsNetwork",
    message: "offline",
  });
  open(api);
  await screen.findByRole("button", { name: /^App/ });
  await waitFor(() => expect(api.ratingsUpdateIfDue).toHaveBeenCalledOnce());
  expect(screen.queryByRole("alert")).toBeNull();
});

it("keeps an in-progress selection when the daily check finds a new list", async () => {
  const { api } = platform(stored);
  let finish: (changed: boolean) => void = () => {};
  api.ratingsUpdateIfDue.mockReturnValueOnce(
    new Promise<boolean>((resolve) => (finish = resolve)),
  );
  open(api);
  await screen.findByRole("button", { name: /^App/ });
  await fireEvent.click(screen.getByRole("button", { name: "Select" }));
  const box = screen.getByLabelText("Select App") as HTMLInputElement;
  await fireEvent.click(box);
  expect(box.checked).toBe(true);
  finish(true);
  await waitFor(() => expect(api.ratingsStatus).toHaveBeenCalledTimes(2));
  expect(api.query).toHaveBeenCalledOnce();
  expect((screen.getByLabelText("Select App") as HTMLInputElement).checked).toBe(
    true,
  );
});
