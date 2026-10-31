import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import SetupTool from "./SetupTool.svelte";
import type { Platform, ToolkitSnapshot, Query } from "./client";
import type { Device } from "../model";
afterEach(cleanup);
const device: Device = {
  id: "session",
  deviceId: "device",
  profileId: 0,
  serial: "serial",
  build: "build",
  manufacturer: "Vendor",
  model: "TV",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: ["armeabi-v7a"],
};
const missing = { installed: false, official: false, version: null };
const official = { installed: true, official: true, version: "1" };
const snapshot = (over: Partial<ToolkitSnapshot>): ToolkitSnapshot => ({
  shizuku: "empty",
  shizukuVersion: null,
  porter: missing,
  termux: missing,
  termuxBoot: missing,
  manager: null,
  serverRunning: false,
  canConfigureTerminal: false,
  terminalReady: false,
  shellReady: false,
  authorized: false,
  monitor: false,
  listener: false,
  stage: "manager",
  ...over,
});
function platform(state: ToolkitSnapshot, route = "shell") {
  return {
    query: vi.fn(async (_: unknown, q: Query) =>
      q.kind === "verifyShell" ? { route } : state,
    ),
    prepare: vi.fn(async () => ({ id: "review" })),
    installTool: vi.fn(async () => "v1"),
    remote: vi.fn(async () => ({ status: "sent" })),
  } as unknown as Platform;
}
const open = (api: Platform, onreview = vi.fn()) =>
  render(SetupTool, { device, api, reload: 0, onreview, onbuttons: vi.fn() });

it("offers only the two supported managers with no file to find", async () => {
  const api = platform(snapshot({}));
  open(api);
  await screen.findByText("Shizuku-Next");
  expect(screen.getByText("Porter")).toBeTruthy();
  expect(screen.queryByText(/APK/)).toBeNull();
  await fireEvent.click(
    screen.getByRole("button", { name: "Install Shizuku-Next" }),
  );
  await waitFor(() =>
    expect(api.installTool).toHaveBeenCalledWith(
      "session",
      "shizukuNext",
      expect.any(Function),
    ),
  );
});

it("says when something else holds the Shizuku name and steers to Porter", async () => {
  open(platform(snapshot({ shizuku: "other" })));
  expect(await screen.findByText(/different app is using/)).toBeTruthy();
  expect(
    (screen.getByRole("button", {
      name: "Install Shizuku-Next",
    }) as HTMLButtonElement).disabled,
  ).toBe(true);
  expect(
    (screen.getByRole("button", { name: "Install Porter" }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
});

it("shows a failed download without leaving the step", async () => {
  const api = platform(snapshot({}));
  (api.installTool as ReturnType<typeof vi.fn>).mockRejectedValue({
    kind: "toolIdentity",
    message: "This download is not signed by its publisher.",
  });
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Install Porter" }),
  );
  expect((await screen.findByRole("alert")).textContent).toContain("signed");
  expect(screen.getByRole("button", { name: "Install Porter" })).toBeTruthy();
});

it("reviews starting the service", async () => {
  const api = platform(
    snapshot({ shizuku: "shizukuNext", manager: "shizukuNext", stage: "server" }),
  );
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Start Shizuku-Next" }),
  );
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "setup",
    step: { kind: "startServer", manager: "shizukuNext" },
  });
});

it("installs what Termux needs before preparing it", async () => {
  const running = {
    shizuku: "shizukuNext" as const,
    manager: "shizukuNext" as const,
    serverRunning: true,
    stage: "terminal" as const,
  };
  const api = platform(snapshot(running));
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Install Termux" }),
  );
  await waitFor(() =>
    expect(api.installTool).toHaveBeenCalledWith(
      "session",
      "termux",
      expect.any(Function),
    ),
  );
  cleanup();
  open(platform(snapshot({ ...running, termux: official })));
  expect(
    await screen.findByRole("button", { name: "Install Termux:Boot" }),
  ).toBeTruthy();
  cleanup();
  open(
    platform(
      snapshot({
        ...running,
        termux: official,
        termuxBoot: official,
        canConfigureTerminal: true,
      }),
    ),
  );
  expect(
    await screen.findByRole("button", { name: "Prepare Termux" }),
  ).toBeTruthy();
});

it("won't use a Termux it can't configure", async () => {
  open(
    platform(
      snapshot({
        manager: "porter",
        porter: official,
        serverRunning: true,
        termux: { installed: true, official: false, version: "1" },
        stage: "terminal",
      }),
    ),
  );
  expect(await screen.findByText(/different Termux/)).toBeTruthy();
});

it("guides the TV prompt with its real wording and proves access", async () => {
  const state = snapshot({
    manager: "porter",
    porter: official,
    serverRunning: true,
    stage: "access",
  });
  const api = platform(state);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Ask the TV" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "setup",
      step: { kind: "requestAccess" },
    }),
  );
  expect(screen.getByText(/Allow all the time/)).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "OK" }));
  expect(api.remote).toHaveBeenCalledWith(expect.anything(), {
    kind: "key",
    key: "ok",
    longPress: false,
  });
  await fireEvent.click(screen.getByRole("button", { name: "Check access" }));
  await waitFor(() =>
    expect(api.query).toHaveBeenCalledWith(expect.anything(), {
      kind: "verifyShell",
    }),
  );
});

it("says plainly when access was not granted", async () => {
  const api = platform(
    snapshot({ manager: "porter", porter: official, stage: "access" }),
    "none",
  );
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Check access" }),
  );
  expect((await screen.findByRole("alert")).textContent).toContain(
    "hasn't been allowed",
  );
});

it("never calls a step done until the TV proves it", async () => {
  open(
    platform(
      snapshot({
        shizuku: "shizukuNext",
        manager: "shizukuNext",
        stage: "server",
      }),
    ),
  );
  await screen.findByRole("button", { name: "Start Shizuku-Next" });
  expect(document.querySelectorAll(".setup-step.done")).toHaveLength(1);
});

it("is honest about restarts when finished", async () => {
  const onbuttons = vi.fn();
  render(SetupTool, {
    device,
    api: platform(
      snapshot({
        manager: "porter",
        porter: official,
        stage: "done",
        monitor: true,
      }),
    ),
    reload: 0,
    onreview: vi.fn(),
    onbuttons,
  });
  expect(await screen.findByText(/hasn't been proven/)).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Remap buttons" }));
  expect(onbuttons).toHaveBeenCalled();
});

it("says so once mappings have come back after a restart", async () => {
  const api = platform(
    snapshot({ manager: "porter", porter: official, stage: "done", monitor: true }),
  );
  const query = api.query as unknown as ReturnType<typeof vi.fn<(t: unknown, q: { kind: string }) => unknown>>;
  const answer = query.getMockImplementation()!;
  query.mockImplementation(async (t, q) =>
    q.kind === "mappings" ? { survivedRestart: true } : answer(t, q),
  );
  render(SetupTool, { device, api, reload: 0, onreview: vi.fn(), onbuttons: vi.fn() });
  expect(await screen.findByText(/Proven on this TV/)).toBeTruthy();
});

it("stops and explains when a TV blocks setting up Termux", async () => {
  open(
    platform(
      snapshot({
        manager: "porter",
        porter: official,
        serverRunning: true,
        termux: official,
        termuxBoot: official,
        canConfigureTerminal: false,
        stage: "terminal",
      }),
    ),
  );
  expect(await screen.findByText(/blocks/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Prepare Termux" })).toBeNull();
});

it("offers a retry instead of blaming another app when it couldn't check", async () => {
  open(platform(snapshot({ shizuku: "unknown" })));
  expect(await screen.findByText(/couldn't check/)).toBeTruthy();
  expect(screen.queryByText(/different app is using/)).toBeNull();
});

