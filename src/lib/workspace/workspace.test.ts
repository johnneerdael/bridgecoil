import {
  render,
  screen,
  fireEvent,
  cleanup,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, beforeAll, it, expect, vi } from "vitest";
import AppsPage from "./AppsPage.svelte";
import RemotePage from "./RemotePage.svelte";
import ActionReview from "./ActionReview.svelte";
import type { Platform, Review, Operation } from "./client";
import type { Device } from "../model";
afterEach(cleanup);
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
});
const device: Device = {
  id: "session-a",
  deviceId: "device-a",
  profileId: 10,
  serial: "serial",
  build: "build",
  manufacturer: "Vendor",
  model: "TV",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: ["arm64-v8a"],
};
const review: Review = {
  id: "review-a",
  target: { deviceId: "device-a", sessionId: "session-a", profileId: 10 },
  title: "Disable org.demo",
  effect: "Disable this app for profile 10.",
  currentState: "Installed, enabled",
  losses: [],
  reversible: true,
  commands: ["pm disable-user --user 10 'org.demo'"],
  expiresMs: Date.now() + 10000,
};
const operation: Operation = {
  id: "op",
  target: review.target,
  title: "Remote",
  kind: "remote",
  status: "sent",
  output: "",
  error: null,
  recovery: null,
  createdMs: 1,
};
it("browses an app and prepares its selected-profile action without executing it", async () => {
  const query = vi
    .fn()
    .mockResolvedValueOnce({
      items: [{ package: "org.demo", label: "Player", icon: null }],
      page: 0,
      pageSize: 4,
      total: 1,
    })
    .mockResolvedValueOnce({
      package: "org.demo",
      state: { installed: true, enabled: 0, stopped: false, version: 1 },
      versionName: "1",
      activities: ["org.demo/.Main"],
      permissions: [],
      raw: "raw",
    });
  const prepare = vi.fn().mockResolvedValue(review),
    apply = vi.fn(),
    onreview = vi.fn();
  render(AppsPage, {
    device,
    api: { query, prepare, apply } as unknown as Platform,
    onreview,
    oninstall: vi.fn(),
  });
  await fireEvent.click(await screen.findByRole("button", { name: /Player/ }));
  await fireEvent.click(await screen.findByRole("button", { name: "actions" }));
  await fireEvent.click(screen.getByRole("button", { name: "Disable" }));
  await waitFor(() => expect(onreview).toHaveBeenCalledWith(review));
  expect(prepare).toHaveBeenCalledWith(review.target, {
    kind: "package",
    package: "org.demo",
    action: "disable",
  });
  expect(apply).not.toHaveBeenCalled();
});
it("does not send duplicate remote presses while a dispatch is in flight", async () => {
  let complete!: (r: Operation) => void;
  const remote = vi
    .fn()
    .mockReturnValue(new Promise<Operation>((resolve) => (complete = resolve)));
  render(RemotePage, {
    device,
    api: { remote } as unknown as Platform,
    onresult: vi.fn(),
    onapps: vi.fn(),
  });
  const up = screen.getByRole("button", { name: "Up" });
  await fireEvent.click(up);
  await fireEvent.click(up);
  expect(remote).toHaveBeenCalledTimes(1);
  expect(remote).toHaveBeenCalledWith(review.target, {
    kind: "key",
    key: "up",
    longPress: false,
  });
  complete(operation);
  await screen.findByText("Sent · check the TV");
});
it("a new review loses the old confirmation before any apply can happen", async () => {
  const apply = vi.fn().mockResolvedValue(operation);
  const props = {
    review,
    api: { apply } as unknown as Platform,
    onclose: vi.fn(),
    onresult: vi.fn(),
  };
  const view = render(ActionReview, props);
  const check = await screen.findByRole("checkbox");
  await fireEvent.click(check);
  expect(
    (screen.getByRole("button", { name: "Apply change" }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
  await view.rerender({
    ...props,
    review: {
      ...review,
      id: "review-b",
      target: { ...review.target, profileId: 0 },
    },
  });
  expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(
    false,
  );
  expect(
    (screen.getByRole("button", { name: "Apply change" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(apply).not.toHaveBeenCalled();
});
