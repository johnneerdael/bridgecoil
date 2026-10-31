import { it, expect, vi, beforeAll, afterEach } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  cleanup,
} from "@testing-library/svelte";
const api = vi.hoisted(() => ({
  query: vi.fn(),
  prepare: vi.fn(),
  discard: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("./client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./client")>();
  return { ...actual, nativePlatform: () => api };
});
import WorkspaceScreen from "./WorkspaceScreen.svelte";
import type { Device } from "../model";
import type { Review } from "./client";
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
  vi.clearAllMocks();
});
it("discards a late prepared review after the active profile/session changes", async () => {
  const first: Device = {
    id: "one",
    deviceId: "tv",
    profileId: 0,
    serial: "s",
    manufacturer: "Vendor",
    build: "build",
    model: "TV",
    endpoint: "tv:1",
    encrypted: true,
    androidVersion: "14",
    sdk: 34,
    abis: [],
  };
  api.query.mockImplementation(async (_target, q) =>
    q.kind === "packages"
      ? {
          items: [{ package: "org.demo", label: "Player", icon: null }],
          total: 1,
          page: 0,
          pageSize: 3,
        }
      : {
          package: "org.demo",
          state: { installed: true, enabled: 0, stopped: false, version: 1 },
          activities: [],
          permissions: [],
          raw: "",
        },
  );
  let finish!: (review: Review) => void;
  api.prepare.mockReturnValue(
    new Promise<Review>((resolve) => (finish = resolve)),
  );
  const props = {
    page: "apps" as const,
    device: first,
    ondevice: vi.fn(),
    onnavigate: vi.fn(),
  };
  const view = render(WorkspaceScreen, props);
  await fireEvent.click(await screen.findByRole("button", { name: /Player/ }));
  await fireEvent.click(screen.getByRole("button", { name: "Actions" }));
  await fireEvent.click(screen.getByRole("button", { name: "Disable" }));
  await waitFor(() => expect(api.prepare).toHaveBeenCalledTimes(1));
  await view.rerender({
    ...props,
    device: { ...first, id: "two", profileId: 20 },
  });
  finish({
    id: "old-review",
    target: { deviceId: "tv", sessionId: "one", profileId: 0 },
    title: "Disable",
    effect: "Disable app",
    currentState: "Enabled",
    losses: [],
    reversible: true,
    commands: [],
    expiresMs: Date.now() + 1000,
  });
  await waitFor(() => expect(api.discard).toHaveBeenCalledWith("old-review"));
  expect(screen.queryByRole("dialog", { name: "Review change" })).toBeNull();
  expect(screen.getByText(/Profile 20/)).toBeTruthy();
});
