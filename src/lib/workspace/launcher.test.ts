import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import LauncherTool from "./LauncherTool.svelte";
import ScreenshotTool from "./ScreenshotTool.svelte";
import type { Platform } from "./client";
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
const stock = "com.vendor.home/.Main";
const fresh = "org.fresh.launcher/.Home";
function platform(current: string, launch = { status: "sent" }) {
  return {
    query: vi.fn(async () => ({ homes: [stock, fresh], current, stock: [stock] })),
    prepare: vi.fn(async () => ({ id: "review" })),
    remote: vi.fn(async () => launch),
  } as unknown as Platform;
}
const open = (api: Platform, onreview = vi.fn()) =>
  render(LauncherTool, { device, api, reload: 0, onreview });

it("shows the current launcher and the alternatives", async () => {
  open(platform(stock));
  expect(await screen.findByText(/org\.fresh\.launcher/)).toBeTruthy();
  expect(screen.getByText(/In use/)).toBeTruthy();
});

it("says when a TV only has its own launcher", async () => {
  const api = platform(stock);
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [stock],
    current: stock,
    stock: [stock],
  });
  open(api);
  expect(await screen.findByText(/only launcher/)).toBeTruthy();
});

it("tries a launcher before anything is changed", async () => {
  const api = platform(stock);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
  );
  await waitFor(() =>
    expect(api.remote).toHaveBeenCalledWith(expect.anything(), {
      kind: "launch",
      component: fresh,
    }),
  );
  expect(api.prepare).not.toHaveBeenCalled();
  expect(await screen.findByText(/Did it open/)).toBeTruthy();
});

it("only offers making it the default after the try worked", async () => {
  const api = platform(stock);
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Yes, it works" }));
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "defaultHome",
    component: fresh,
  });
});

it("changes nothing when the try failed", async () => {
  const api = platform(stock);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "No" }));
  expect(api.prepare).not.toHaveBeenCalled();
  expect(await screen.findByText(/org\.fresh\.launcher/)).toBeTruthy();
});

it("keeps stock switched on until the Home button is proven", async () => {
  const api = platform(fresh);
  open(api);
  await screen.findByText(/Press Home/);
  expect(screen.queryByRole("button", { name: /Turn off/ })).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Home opened it" }));
  expect(
    await screen.findByRole("button", { name: "Turn off com.vendor.home" }),
  ).toBeTruthy();
});

it("turns stock off for this profile through review, with a way back", async () => {
  const api = platform(fresh);
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Home opened it" }),
  );
  expect(screen.getByText(/History/)).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Turn off com.vendor.home" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "package",
      package: "com.vendor.home",
      action: "disable",
    }),
  );
});

it("offers going back when Home didn't open the new launcher", async () => {
  const api = platform(fresh);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Home opened something else" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Go back to com.vendor.home" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "defaultHome",
      component: stock,
    }),
  );
});

it("takes a screenshot on request and shows it", async () => {
  const api = {
    query: vi.fn(async () => ({ png: "iVBORw0KGgo=" })),
  } as unknown as Platform;
  render(ScreenshotTool, { device, api });
  await fireEvent.click(
    screen.getByRole("button", { name: "Take a screenshot" }),
  );
  const image = (await screen.findByRole("img")) as HTMLImageElement;
  expect(image.src).toContain("iVBORw0KGgo=");
});
