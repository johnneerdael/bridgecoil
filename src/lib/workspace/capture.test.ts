import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import ScreenshotTool from "./ScreenshotTool.svelte";
import LogsTool from "./LogsTool.svelte";
import type { SavedCapture, Platform, Query } from "./client";
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
const download = { location: "Download", uri: "content://media/1" };
function platform(saved: SavedCapture | null = download) {
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
    saveFile: vi.fn(async () => true),
    saveTvFile: vi.fn(async () => true),
    saveCapture: vi.fn(async () => saved),
    saveTvCapture: vi.fn(async () => saved),
    openCapture: vi.fn(async () => {}),
  } as unknown as Platform;
}
const stepsOf = (api: Platform) =>
  (api.query as ReturnType<typeof vi.fn>).mock.calls
    .map(([, q]) => q)
    .filter((q) => q.kind === "recording")
    .map((q) => q.step.kind)
    .filter((kind) => kind !== "status");

it("puts a screenshot in Download without asking", async () => {
  const api = platform();
  render(ScreenshotTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Take a screenshot" }));
  await waitFor(() =>
    expect(api.saveCapture).toHaveBeenCalledWith(
      expect.stringMatching(/^tv-.*\.png$/),
      "iVBORw0KGgo",
    ),
  );
  expect(
    await screen.findByText("Screenshot saved to your phone's Download folder."),
  ).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Save screenshot" })).toBeNull();
});

it("keeps the picture on screen when a picker was closed, so it can be saved again", async () => {
  render(ScreenshotTool, { device, api: platform(null) });
  await fireEvent.click(screen.getByRole("button", { name: "Take a screenshot" }));
  expect(await screen.findByRole("button", { name: "Save screenshot" })).toBeTruthy();
  expect(screen.queryByText(/Screenshot saved/)).toBeNull();
});

it("says only that it was saved after a picker", async () => {
  render(ScreenshotTool, { device, api: platform({ location: "", uri: null }) });
  await fireEvent.click(screen.getByRole("button", { name: "Take a screenshot" }));
  expect(await screen.findByText("Screenshot saved.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Open screenshot" })).toBeNull();
});

it("offers to open a screenshot in the phone's default app", async () => {
  const api = platform();
  render(ScreenshotTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Take a screenshot" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Open screenshot" }));
  await waitFor(() =>
    expect(api.openCapture).toHaveBeenCalledWith("content://media/1", "image/png"),
  );
  expect(screen.getByText("Screenshot saved to your phone's Download folder.")).toBeTruthy();
});

it("offers to open a recording once it's on the phone", async () => {
  const api = platform();
  render(ScreenshotTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Start recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Open recording" }));
  await waitFor(() =>
    expect(api.openCapture).toHaveBeenCalledWith("content://media/1", "video/mp4"),
  );
});

it("moves a finished recording to the phone and off the TV by itself", async () => {
  const api = platform();
  render(ScreenshotTool, { device, api });
  await fireEvent.change(screen.getByLabelText("Record for"), {
    target: { value: "60" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Start recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  await waitFor(() => expect(stepsOf(api)).toEqual(["start", "stop", "discard"]));
  expect(api.query).toHaveBeenCalledWith(expect.anything(), {
    kind: "recording",
    step: { kind: "start", seconds: 60 },
  });
  expect(api.saveTvCapture).toHaveBeenCalledWith(
    "session",
    path,
    expect.stringMatching(/^tv-.*\.mp4$/),
    expect.any(Function),
  );
  expect(
    await screen.findByText(
      "Recording saved to your phone's Download folder and removed from the TV.",
    ),
  ).toBeTruthy();
});

it("keeps the recording on the TV until it's on the phone", async () => {
  const api = platform(null);
  render(ScreenshotTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Start recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  expect(await screen.findByRole("button", { name: "Save recording" })).toBeTruthy();
  expect(stepsOf(api)).toEqual(["start", "stop"]);
});

it("keeps the recording on the TV when copying it fails", async () => {
  const api = platform();
  (api.saveTvCapture as ReturnType<typeof vi.fn>).mockRejectedValue(new Error("stalled"));
  render(ScreenshotTool, { device, api });
  await fireEvent.click(screen.getByRole("button", { name: "Start recording" }));
  await fireEvent.click(await screen.findByRole("button", { name: "Stop recording" }));
  expect(await screen.findByRole("button", { name: "Save recording" })).toBeTruthy();
  expect(stepsOf(api)).toEqual(["start", "stop"]);
});

it("picks a recording back up after leaving the screen", async () => {
  const api = platform();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({ running: true, size: 10, path });
  render(ScreenshotTool, { device, api });
  expect(await screen.findByRole("button", { name: "Stop recording" })).toBeTruthy();
});

it("moves a finished recording left on the TV to the phone", async () => {
  const api = platform();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({ running: false, size: 10, path });
  render(ScreenshotTool, { device, api });
  await waitFor(() => expect(api.saveTvCapture).toHaveBeenCalled());
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
