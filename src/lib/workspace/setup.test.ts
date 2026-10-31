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
  wired: false,
  noWirelessDebugging: false,
  networkAdb: false,
  helper: null,
  bootStart: false,
  watchdog: false,
  agent: false,
  agentReady: false,
  stage: "terminal",
  ...over,
});
function platform(state: ToolkitSnapshot, route = "shell") {
  return {
    query: vi.fn(async (_: unknown, q: Query) =>
      q.kind === "verifyAgent" ? { route } : state,
    ),
    prepare: vi.fn(async () => ({ id: "review" })),
    installTool: vi.fn(async () => "v1"),
    installAgent: vi.fn(async () => {}),
    remote: vi.fn(async () => ({ status: "sent" })),
  } as unknown as Platform;
}
const open = (api: Platform, onreview = vi.fn()) =>
  render(SetupTool, { device, api, reload: 0, onreview, onbuttons: vi.fn() });

const termuxReady = {
  termux: official,
  termuxBoot: official,
  canConfigureTerminal: true,
  terminalReady: true,
};
const pairing = { ...termuxReady, agent: true, stage: "pair" as const };

it("puts the agent into Termux by itself", async () => {
  const api = platform(snapshot({ ...termuxReady, stage: "pair" }));
  open(api);
  await waitFor(() => expect(api.installAgent).toHaveBeenCalledWith("session"));
  expect(api.installAgent).toHaveBeenCalledTimes(1);
});

it("connects with only the code on the TV", async () => {
  const api = platform(snapshot(pairing));
  open(api);
  const connect = await screen.findByRole("button", { name: "Connect" });
  expect((connect as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.input(screen.getByLabelText("Pairing code"), {
    target: { value: "123456" },
  });
  await fireEvent.click(connect);
  await waitFor(() =>
    expect(api.query).toHaveBeenCalledWith(expect.anything(), {
      kind: "pairAgent",
      code: "123456",
    }),
  );
  await waitFor(() =>
    expect(api.query).toHaveBeenCalledWith(expect.anything(), {
      kind: "verifyAgent",
    }),
  );
});

it("says so when pairing worked but Termux still can't reach the TV", async () => {
  const api = platform(snapshot(pairing), "none");
  open(api);
  await fireEvent.input(await screen.findByLabelText("Pairing code"), {
    target: { value: "123456" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Connect" }));
  expect((await screen.findByRole("alert")).textContent).toContain(
    "can't reach the TV",
  );
});

it("waits for the TV by itself where there's no code to type", async () => {
  let tries = 0;
  const old = { ...pairing, noWirelessDebugging: true, networkAdb: true };
  const api = platform(snapshot(old));
  vi.mocked(api.query).mockImplementation((async (_: unknown, q: Query) => {
    if (q.kind === "verifyAgent") return { route: ++tries < 3 ? "none" : "shell" };
    return tries >= 3
      ? snapshot({ ...old, agentReady: true, monitor: true, stage: "done" })
      : snapshot(old);
  }) as never);
  render(SetupTool, {
    device,
    api,
    reload: 0,
    onreview: vi.fn(),
    onbuttons: vi.fn(),
    pollMs: 1,
  });
  expect(await screen.findByText(/moves on by itself/)).toBeTruthy();
  expect(screen.queryByLabelText("Pairing code")).toBeNull();
  expect(await screen.findByText("Your TV is ready")).toBeTruthy();
  expect(tries).toBe(3);
});

it("needs no Shizuku to be ready, and offers it as an extra", async () => {
  const ready = { ...termuxReady, agent: true, agentReady: true, monitor: true };
  const api = platform(snapshot({ ...ready, stage: "done" }));
  open(api);
  expect(await screen.findByText("Your TV is ready")).toBeTruthy();
  expect(screen.getByText("Shizuku, for other apps")).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Install Shizuku-Next" }),
  );
  await waitFor(() =>
    expect(api.installTool).toHaveBeenCalledWith(
      "session",
      "shizukuNext",
      expect.anything(),
    ),
  );
});

it("shows a failed download without leaving the step", async () => {
  const api = platform(snapshot({ terminalReady: false }));
  (api.installTool as ReturnType<typeof vi.fn>).mockRejectedValue({
    kind: "toolIdentity",
    message: "This download is not signed by its publisher.",
  });
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Install Termux" }),
  );
  expect((await screen.findByRole("alert")).textContent).toContain("signed");
  expect(screen.getByRole("button", { name: "Install Termux" })).toBeTruthy();
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
