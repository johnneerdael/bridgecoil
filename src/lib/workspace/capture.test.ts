import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import ScreenshotTool from "./ScreenshotTool.svelte";
import LogsTool from "./LogsTool.svelte";
import type { Platform, Query } from "./client";
import type { Device } from "../model";
// jsdom has no <dialog> methods.
HTMLDialogElement.prototype.showModal = function () {
  this.open = true;
};
HTMLDialogElement.prototype.close = function () {
  this.open = false;
};
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});
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
  abis: [],
};
const path = "/data/local/tmp/bridgecoil-recording.mp4";
function platform(saved = true) {
  return {
    query: vi.fn(async (_t: unknown, q: Query) => {
      if (q.kind === "screenshot") return { png: "iVBORw0KGgo" };
      if (q.kind === "launchableApps")
        return [
          { component: "org.demo/.Main", label: "Player" },
          { component: "org.demo/.Other", label: "Player extras" },
        ];
      if (q.kind === "logs")
        return { output: q.package === "org.gone" ? "" : "a log line", notRunning: false };
      if (q.kind === "recording" && q.step.kind === "status")
        return { running: false, size: null, path };
      return { running: false, size: 1000, path };
    }),
    saveFile: vi.fn(async () => saved),
    saveTvFile: vi.fn(async () => saved),
  } as unknown as Platform;
}
const stepsOf = (api: Platform) =>
  (api.query as ReturnType<typeof vi.fn>).mock.calls
    .map(([, q]) => q)
    .filter((q) => q.kind === "recording")
    .map((q) => q.step.kind)
    .filter((kind) => kind !== "status");

it("saves the screenshot it took", async () => {
  const api = platform();
  render(ScreenshotTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Take a screenshot" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Save screenshot" }));
  await waitFor(() =>
    expect(api.saveFile).toHaveBeenCalledWith(expect.stringMatching(/\.png$/), {
      base64: "iVBORw0KGgo",
    }),
  );
  expect(await screen.findByText("Screenshot saved.")).toBeTruthy();
});

it("says nothing was saved when the picker is closed", async () => {
  render(ScreenshotTool, { device, api: platform(false) });
  await fireEvent.click(screen.getByRole("button", { name: "Take a screenshot" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Save screenshot" }));
  await new Promise((r) => setTimeout(r, 0));
  expect(screen.queryByText("Screenshot saved.")).toBeNull();
});

it("records for the chosen length and removes the file once it's saved", async () => {
  const api = platform();
  render(ScreenshotTool, { device, api });
  await fireEvent.change(screen.getByLabelText("Record for"), {
    target: { value: "60" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Start recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Save recording" }));
  await waitFor(() => expect(stepsOf(api)).toEqual(["start", "stop", "discard"]));
  expect(api.query).toHaveBeenCalledWith(expect.anything(), {
    kind: "recording",
    step: { kind: "start", seconds: 60 },
  });
  expect(api.saveTvFile).toHaveBeenCalledWith(
    "session",
    path,
    expect.stringMatching(/\.mp4$/),
    expect.any(Function),
  );
});

it("keeps the recording on the TV until it's actually saved", async () => {
  const api = platform(false);
  render(ScreenshotTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Start recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Save recording" }));
  await waitFor(() => expect(api.saveTvFile).toHaveBeenCalled());
  await new Promise((r) => setTimeout(r, 0));
  expect(stepsOf(api)).toEqual(["start", "stop"]);
  expect(screen.getByRole("button", { name: "Save recording" })).toBeTruthy();
});

it("picks a recording back up after leaving the screen", async () => {
  const api = platform();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({ running: true, size: 10, path });
  render(ScreenshotTool, { device, api });
  expect(await screen.findByRole("button", { name: "Stop recording" })).toBeTruthy();
});

it("offers a finished recording left on the TV", async () => {
  const api = platform();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({ running: false, size: 10, path });
  render(ScreenshotTool, { device, api });
  expect(await screen.findByRole("button", { name: "Save recording" })).toBeTruthy();
});

it("reports a recording that came out empty", async () => {
  const api = platform();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    running: false,
    size: null,
    path,
  });
  render(ScreenshotTool, { device, api });
  await fireEvent.click(await screen.findByRole("button", { name: "Start recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  expect(await screen.findByRole("alert")).toBeTruthy();
});

it("filters logs by detail and app, listing each app once", async () => {
  const api = platform();
  render(LogsTool, { device, api });
  await screen.findByRole("option", { name: "Player" });
  expect(screen.queryByRole("option", { name: "Player extras" })).toBeNull();
  await fireEvent.change(screen.getByLabelText("Show"), { target: { value: "error" } });
  await fireEvent.change(screen.getByLabelText("From"), { target: { value: "org.demo" } });
  await fireEvent.click(screen.getByRole("button", { name: "Show logs" }));
  await waitFor(() =>
    expect(api.query).toHaveBeenCalledWith(expect.anything(), {
      kind: "logs",
      level: "error",
      package: "org.demo",
      lines: 200,
    }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Save log" }));
  await waitFor(() =>
    expect(api.saveFile).toHaveBeenCalledWith("tv-log.txt", { text: "a log line" }),
  );
});

it("says when the chosen app isn't running", async () => {
  const api = platform();
  (api.query as ReturnType<typeof vi.fn>).mockImplementation(async (_t, q: Query) =>
    q.kind === "logs" ? { output: "", notRunning: true } : [],
  );
  render(LogsTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Show logs" }));
  expect(await screen.findByText(/isn't running/)).toBeTruthy();
});
