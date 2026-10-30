import { render, screen, fireEvent, cleanup } from "@testing-library/svelte";
import { it, expect, vi, afterEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
import App from "./App.svelte";
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it("explains native LAN denial and never connects after denied pairing", async () => {
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === "pair_device")
      throw {
        kind: "permission",
        message: "Allow local network access in app settings, then retry.",
      };
    return null;
  });
  render(App);
  await fireEvent.input(screen.getByLabelText(/Pairing address/), {
    target: { value: "tv.local:37123" },
  });
  await fireEvent.input(screen.getByLabelText("Six-digit pairing code"), {
    target: { value: "012345" },
  });
  await fireEvent.submit(
    screen
      .getByRole("button", { name: "Pair TV" })
      .closest("form")!,
  );
  const alert = await screen.findByRole("alert");
  expect(alert.textContent).toContain("Local network access is off");
  expect(alert.textContent).toContain("app settings");
  expect(
    vi.mocked(invoke).mock.calls.filter((c) => c[0] === "pair_device"),
  ).toHaveLength(1);
  expect(
    vi.mocked(invoke).mock.calls.some((c) => c[0] === "connect_device"),
  ).toBe(false);
});
