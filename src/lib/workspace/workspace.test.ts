import {
  render,
  screen,
  fireEvent,
  cleanup,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, beforeAll, describe, it, expect, vi } from "vitest";
import AppsPage from "./AppsPage.svelte";
import RemotePage from "./RemotePage.svelte";
import ActionReview from "./ActionReview.svelte";
import BulkActionReview from "./BulkReview.svelte";
import HistoryPage from "./HistoryPage.svelte";
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
it("scrolls continuously through app batches without page controls", async () => {
  const apps = Array.from({ length: 64 }, (_, i) => ({
    package: `org.demo.app${i}`,
    label: `Player ${i}`,
    icon: null,
  }));
  const query = vi.fn(async (_target, q) => ({
    items: apps.slice(q.page * q.pageSize, (q.page + 1) * q.pageSize),
    total: apps.length,
    page: q.page,
    pageSize: q.pageSize,
  }));
  render(AppsPage, {
    device,
    api: { query } as unknown as Platform,
    onreview: vi.fn(),
    onbulk: vi.fn(),
    oninstall: vi.fn(),
  });
  await screen.findByRole("button", { name: /Player 31/ });
  expect(screen.queryByRole("button", { name: "Next" })).toBeNull();
  expect(screen.queryByRole("button", { name: /Player 63/ })).toBeNull();
  const list = screen.getByRole("region", { name: "Installed apps" });
  Object.defineProperties(list, {
    scrollHeight: { value: 2000 },
    clientHeight: { value: 400 },
    scrollTop: { value: 1500, writable: true },
  });
  await fireEvent.scroll(list);
  await screen.findByRole("button", { name: /Player 63/ });
  expect(screen.getByText("Player 0")).toBeTruthy();
  await fireEvent.scroll(list);
  expect(query).toHaveBeenCalledTimes(2);
});

it("discards an old app batch when the filter changes", async () => {
  let complete!: (value: unknown) => void;
  const query = vi
    .fn()
    .mockResolvedValueOnce({
      items: [{ package: "org.demo.old", label: "Old app" }],
      total: 64,
      page: 0,
      pageSize: 32,
    })
    .mockReturnValueOnce(new Promise((resolve) => (complete = resolve)))
    .mockResolvedValueOnce({
      items: [{ package: "org.demo.new", label: "New app" }],
      total: 1,
      page: 0,
      pageSize: 32,
    });
  render(AppsPage, {
    device,
    api: { query } as unknown as Platform,
    onreview: vi.fn(),
    onbulk: vi.fn(),
    oninstall: vi.fn(),
  });
  await screen.findByRole("button", { name: /Old app/ });
  await fireEvent.scroll(
    screen.getByRole("region", { name: "Installed apps" }),
  );
  await fireEvent.change(screen.getByLabelText("App filter"), {
    target: { value: "user" },
  });
  await screen.findByRole("button", { name: /New app/ });
  complete({
    items: [{ package: "org.demo.stale", label: "Stale app" }],
    total: 64,
    page: 1,
    pageSize: 32,
  });
  await waitFor(() =>
    expect(screen.queryByRole("button", { name: /Stale app/ })).toBeNull(),
  );
  expect(screen.queryByRole("button", { name: /Old app/ })).toBeNull();
});
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
    onbulk: vi.fn(),
    oninstall: vi.fn(),
  });
  await fireEvent.click(await screen.findByRole("button", { name: /Player/ }));
  await fireEvent.click(await screen.findByRole("button", { name: "Actions" }));
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

it("bulk selection prepares only selected packages and never applies during review", async () => {
  const bulk = {
    id: "bulk",
    target: review.target,
    items: [{ package: "org.demo", review }],
    excluded: [],
    expiresMs: Date.now() + 10000,
  };
  const prepareBulk = vi.fn().mockResolvedValue(bulk),
    applyBulk = vi.fn(),
    onbulk = vi.fn();
  const query = vi.fn().mockResolvedValue({
    items: [{ package: "org.demo", label: "Player", icon: null }],
    total: 1,
    page: 0,
    pageSize: 3,
  });
  render(AppsPage, {
    device,
    api: { query, prepareBulk, applyBulk } as unknown as Platform,
    onreview: vi.fn(),
    onbulk,
    oninstall: vi.fn(),
  });
  await screen.findByRole("button", { name: /Player/ });
  await fireEvent.click(screen.getByRole("button", { name: "Select" }));
  await fireEvent.click(
    screen.getByRole("checkbox", { name: "Select Player" }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Review 1 app" }));
  await waitFor(() => expect(onbulk).toHaveBeenCalledWith(bulk));
  expect(prepareBulk).toHaveBeenCalledWith(
    review.target,
    ["org.demo"],
    "disable",
  );
  expect(applyBulk).not.toHaveBeenCalled();
});
it("a running bulk review keeps cancellation available until the active item finishes", async () => {
  let finish!: (operation: Operation) => void;
  const applyBulk = vi
      .fn()
      .mockReturnValue(new Promise<Operation>((resolve) => (finish = resolve))),
    cancelBulk = vi.fn().mockResolvedValue(undefined);
  const bulk = {
    id: "bulk",
    target: review.target,
    items: [{ package: "org.demo", review }],
    excluded: [],
    expiresMs: Date.now() + 10000,
  };
  render(BulkActionReview, {
    review: bulk,
    api: { applyBulk, cancelBulk } as unknown as Platform,
    deviceName: "TV",
    onclose: vi.fn(),
    onresult: vi.fn(),
  });
  await fireEvent.click(
    screen.getByRole("checkbox", { name: /Apply the included/ }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Apply 1 change" }));
  expect(
    (screen.getByRole("button", { name: "Close dialog" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  await fireEvent.click(
    screen.getByRole("button", { name: "Cancel remaining items" }),
  );
  expect(cancelBulk).toHaveBeenCalledWith("bulk");
  finish({ ...operation, status: "cancelled" });
  await waitFor(() => expect(applyBulk).toHaveBeenCalledTimes(1));
});

it("opens a browsed app without exposing or requiring a typed activity name", async () => {
  const query = vi
    .fn()
    .mockResolvedValueOnce({
      items: [{ package: "org.demo", label: "Player", icon: null }],
      total: 1,
      page: 0,
      pageSize: 3,
    })
    .mockResolvedValueOnce({
      package: "org.demo",
      state: { installed: true, enabled: 0, stopped: false, version: 1 },
      versionName: "1",
      activities: ["org.demo/.Main"],
      launchActivity: "org.demo/.Main",
      permissions: [],
      raw: "",
    });
  const remote = vi.fn().mockResolvedValue(operation),
    prepare = vi.fn();
  render(AppsPage, {
    device,
    api: { query, remote, prepare } as unknown as Platform,
    onreview: vi.fn(),
    onbulk: vi.fn(),
    oninstall: vi.fn(),
  });
  await fireEvent.click(await screen.findByRole("button", { name: /Player/ }));
  await fireEvent.click(screen.getByRole("button", { name: "Open app" }));
  await waitFor(() =>
    expect(remote).toHaveBeenCalledWith(review.target, {
      kind: "launch",
      component: "org.demo/.Main",
    }),
  );
  expect(prepare).not.toHaveBeenCalled();
});

it("identifies original devices in history when package and profile match", async () => {
  const rows = [
    {
      ...operation,
      id: "living",
      title: "Disable app",
      deviceLabel: "Living room TV",
    },
    {
      ...operation,
      id: "bedroom",
      title: "Disable app",
      deviceLabel: "Bedroom TV",
    },
  ];
  const history = vi
      .fn()
      .mockResolvedValue({ items: rows, total: 2, page: 0, pageSize: 4 }),
    recovery = vi.fn();
  render(HistoryPage, {
    api: { history, recovery } as unknown as Platform,
    onreview: vi.fn(),
  });
  expect(
    await screen.findByRole("button", { name: /Living room TV/ }),
  ).toBeTruthy();
  expect(screen.getByRole("button", { name: /Bedroom TV/ })).toBeTruthy();
  expect(recovery).not.toHaveBeenCalled();
});

describe("uninstalling an app", () => {
  const detail = (system: boolean, installed = true) => ({
    package: "org.demo",
    system,
    state: { installed, enabled: 0, stopped: false, version: 1 },
    versionName: "1",
    activities: ["org.demo/.Main"],
    permissions: [],
    raw: "raw",
  });
  async function open(info: ReturnType<typeof detail>) {
    const query = vi
      .fn()
      .mockResolvedValueOnce({
        items: [{ package: "org.demo", label: "Player", icon: null }],
        page: 0,
        pageSize: 4,
        total: 1,
      })
      .mockResolvedValueOnce(info);
    const prepare = vi.fn().mockResolvedValue(review);
    render(AppsPage, {
      device,
      api: { query, prepare } as unknown as Platform,
      onreview: vi.fn(),
      onbulk: vi.fn(),
      oninstall: vi.fn(),
    });
    await fireEvent.click(await screen.findByRole("button", { name: /Player/ }));
    await screen.findByRole("button", { name: "Open app" });
    return prepare;
  }
  it("offers it straight away on an app the owner added, and sends it for review", async () => {
    const prepare = await open(detail(false));
    await fireEvent.click(screen.getByRole("button", { name: "Uninstall" }));
    expect(prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "package",
      package: "org.demo",
      action: "uninstall",
    });
  });
  it("replaces the per-profile actions, which mean little for such an app", async () => {
    await open(detail(false));
    await fireEvent.click(screen.getByRole("button", { name: "Actions" }));
    expect(screen.getByRole("button", { name: "Uninstall" })).toBeTruthy();
    for (const page of [0, 1]) {
      expect(screen.queryByRole("button", { name: "Remove from profile" })).toBeNull();
      expect(
        screen.queryByRole("button", { name: "Restore existing package" }),
      ).toBeNull();
      const next = screen.queryByRole("button", { name: /next/i });
      if (page === 0 && next) await fireEvent.click(next);
    }
  });
  it("isn't offered for an app that came with the TV", async () => {
    await open(detail(true));
    expect(screen.queryByRole("button", { name: "Uninstall" })).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Actions" }));
    expect(screen.queryByRole("button", { name: "Uninstall" })).toBeNull();
  });
});
