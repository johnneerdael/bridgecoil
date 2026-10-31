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

const access = {
  manager: "porter" as const,
  porter: official,
  stage: "access" as const,
};
const watch = (api: Platform, watching: "access" | "boot") => {
  const onwatched = vi.fn();
  render(SetupTool, {
    device,
    api,
    reload: 0,
    onreview: vi.fn(),
    onbuttons: vi.fn(),
    watching,
    onwatched,
    pollMs: 1,
  });
  return onwatched;
};

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
      step: { kind: "requestAccess", manager: "porter" },
    }),
  );
  expect(screen.getByText(/Allow all the time/)).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "OK" }));
  expect(api.remote).toHaveBeenCalledWith(expect.anything(), {
    kind: "key",
    key: "ok",
    longPress: false,
  });
});

it("says plainly when access was not granted", async () => {
  const api = platform(
    snapshot({ manager: "porter", porter: official, stage: "access" }),
    "none",
  );
  watch(api, "access");
  await fireEvent.click(
    await screen.findByRole("button", { name: "Check access" }),
  );
  await waitFor(() =>
    expect(screen.getByRole("alert").textContent).toContain(
      "hasn't been allowed",
    ),
  );
});

it.each(["timeout", "unknown"])(
  "doesn't blame the viewer when the check got no answer (%s)",
  async (route) => {
    const api = platform(
      snapshot({ manager: "porter", porter: official, stage: "access" }),
      route,
    );
    watch(api, "access");
    await fireEvent.click(
      await screen.findByRole("button", { name: "Check access" }),
    );
    await waitFor(() =>
      expect(screen.getByRole("alert").textContent).toContain(
        "didn't answer in time",
      ),
    );
  },
);

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

it("only warns when a TV really can't start its manager", async () => {
  const ready = {
    manager: "porter" as const,
    terminalReady: true,
    shellReady: true,
    authorized: true,
    serverRunning: true,
    stage: "startup" as const,
  };
  const wired = open(platform(snapshot({ ...ready, wired: true })));
  expect(await screen.findByText(/on a network cable/)).toBeTruthy();
  wired.unmount();
  const old = open(platform(snapshot({ ...ready, noWirelessDebugging: true })));
  expect(await screen.findByText(/no network ADB/)).toBeTruthy();
  old.unmount();
  const network = open(
    platform(
      snapshot({
        ...ready,
        wired: true,
        noWirelessDebugging: true,
        networkAdb: true,
      }),
    ),
  );
  await screen.findByRole("button", { name: /Keep mappings running/ });
  expect(screen.queryByRole("note")).toBeNull();
  network.unmount();
  open(platform(snapshot(ready)));
  await screen.findByRole("button", { name: /Keep mappings running/ });
  expect(screen.queryByRole("note")).toBeNull();
});

it("only calls the TV ready once it can come back by itself", async () => {
  const ready = {
    manager: "shizukuNext" as const,
    shizuku: "shizukuNext" as const,
    terminalReady: true,
    shellReady: true,
    authorized: true,
    serverRunning: true,
    monitor: true,
  };
  const onbuttons = vi.fn();
  const api = platform(snapshot({ ...ready, stage: "startup" }));
  const missing = render(SetupTool, {
    device,
    api,
    reload: 0,
    onreview: vi.fn(),
    onbuttons,
  });
  expect(await screen.findByText("Not ready for restarts yet")).toBeTruthy();
  expect(screen.queryByText("Your TV is ready")).toBeNull();
  expect(screen.getByText(/On the TV: Settings, Start on boot/)).toBeTruthy();
  expect(screen.getByText(/Watchdog\s+off/)).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Remap buttons" }));
  expect(onbuttons).toHaveBeenCalled();
  missing.unmount();
  open(
    platform(
      snapshot({ ...ready, bootStart: true, watchdog: true, stage: "done" }),
    ),
  );
  expect(await screen.findByText("Your TV is ready")).toBeTruthy();
  expect(screen.queryByText("Not ready for restarts yet")).toBeNull();
});

it("moves on by itself once the TV has allowed access", async () => {
  let tries = 0;
  const api = platform(snapshot(access));
  const ready = snapshot({ ...access, stage: "startup", authorized: true, shellReady: true });
  vi.mocked(api.query).mockImplementation((async (_: unknown, q: Query) => {
    if (q.kind === "verifyShell") return { route: ++tries < 3 ? "none" : "shell" };
    return tries >= 3 ? ready : snapshot(access);
  }) as never);
  const onwatched = watch(api, "access");
  expect(await screen.findByText(/moves on by itself/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Check access" })).toBeNull();
  await waitFor(() => expect(onwatched).toHaveBeenCalled());
  expect(await screen.findByText("Survive restarts", { selector: "h3" })).toBeTruthy();
  expect(tries).toBe(3);
});

it("offers Check access only after the TV never answered", async () => {
  const api = platform(snapshot(access), "none");
  const onwatched = watch(api, "access");
  await waitFor(() => expect(onwatched).toHaveBeenCalled());
  expect(await screen.findByRole("alert")).toBeTruthy();
});

it("doesn't touch the helper until the viewer asks", async () => {
  const api = platform(snapshot(access));
  open(api);
  await screen.findByRole("button", { name: "Ask the TV" });
  await new Promise((done) => setTimeout(done, 20));
  expect(api.query).not.toHaveBeenCalledWith(expect.anything(), { kind: "verifyShell" });
});

it("ticks start on boot by itself once it's turned on", async () => {
  const ready = {
    manager: "porter" as const,
    porter: official,
    terminalReady: true,
    shellReady: true,
    authorized: true,
    serverRunning: true,
    monitor: true,
    stage: "startup" as const,
  };
  let reads = 0;
  const api = platform(snapshot(ready));
  vi.mocked(api.query).mockImplementation((async () =>
    ++reads < 3
      ? snapshot(ready)
      : snapshot({ ...ready, bootStart: true, stage: "done" })) as never);
  const onwatched = watch(api, "boot");
  expect(await screen.findByText("Your TV is ready")).toBeTruthy();
  await waitFor(() => expect(onwatched).toHaveBeenCalled());
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

