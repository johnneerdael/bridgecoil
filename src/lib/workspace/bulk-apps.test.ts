import { afterEach, beforeAll, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import type { Device } from "../model";
import type { BulkReview, Operation, Platform, Query, Review } from "./client";
const native = vi.hoisted(() => ({ api: {} as Platform }));
vi.mock("./client", async (original) => ({
  ...(await original<typeof import("./client")>()),
  nativePlatform: () => native.api,
}));
import AppsPage from "./AppsPage.svelte";
import WorkspaceScreen from "./WorkspaceScreen.svelte";
import BulkReviewView from "./BulkReview.svelte";

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
});
afterEach(cleanup);
const device: Device = {
  id: "session",
  deviceId: "tv",
  profileId: 0,
  serial: "serial",
  build: "build",
  manufacturer: "Vendor",
  model: "TV",
  endpoint: "tv:1",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: [],
};
const target = { deviceId: "tv", sessionId: "session", profileId: 0 };
const review = (id: string): Review => ({
  id,
  target,
  title: "Uninstall app",
  effect: "Delete this app from the TV completely.",
  currentState: "Installed",
  losses: ["App data is deleted from every profile."],
  reversible: false,
  commands: [`pm uninstall '${id}'`],
  expiresMs: Date.now() + 600000,
});
const bulk = (ids: string[]): BulkReview => ({
  id: "batch",
  target,
  items: ids.map((id) => ({ package: id, review: review(id) })),
  excluded: [{ package: "org.system", reason: "This app came with the TV." }],
  expiresMs: Date.now() + 600000,
});
const operation = (kind = "package"): Operation => ({
  id: "operation",
  target,
  kind,
  title: "App changes",
  status: "verified",
  output: "Done",
  error: null,
  recovery: null,
  createdMs: 1,
});
function setup(count = 80) {
  const apps = Array.from({ length: count }, (_, i) => ({
    package: `org.app${i}`,
    label: `Player ${i}`,
    icon: null,
  }));
  const remove = (id: string) =>
    apps.splice(
      apps.findIndex((app) => app.package === id),
      1,
    );
  const api = {
    query: vi.fn(async (_target: unknown, q: Query) => {
      if (q.kind === "packages")
        return {
          items: apps.slice(q.page * q.pageSize, (q.page + 1) * q.pageSize),
          total: apps.length,
          page: q.page,
          pageSize: q.pageSize,
        };
      return {
        package: "org.app50",
        system: false,
        state: { installed: true, enabled: 0, stopped: false, version: 1 },
        activities: [],
        permissions: [],
        raw: "",
      };
    }),
    prepare: vi.fn(async () => review("org.app50")),
    apply: vi.fn(async () => {
      remove("org.app50");
      return operation();
    }),
    prepareBulk: vi.fn(async (_t: unknown, ids: string[]) => bulk(ids)),
    applyBulk: vi.fn(async () => {
      remove("org.app50");
      return {
        ...operation("bulkPackages"),
        status: "partial",
        output: JSON.stringify({
          items: [
            { package: "org.app50", status: "verified" },
            { package: "org.app51", status: "failed" },
          ],
        }),
      };
    }),
    discard: vi.fn(async () => {}),
    discardBulk: vi.fn(async () => {}),
    ratingsStatus: vi.fn(async () => ({ stored: true })),
    ratingsUpdateIfDue: vi.fn(async () => false),
  };
  native.api = api as unknown as Platform;
  return api;
}
const props = () => ({
  device,
  api: native.api,
  onreview: vi.fn(),
  onbulk: vi.fn(),
  oninstall: vi.fn(),
});
async function loadSecondPage() {
  await screen.findByText("Player 31");
  const list = screen.getByRole("region", { name: "Installed apps" });
  Object.defineProperties(list, {
    scrollHeight: { value: 3000, configurable: true },
    clientHeight: { value: 400, configurable: true },
    scrollTop: { value: 2500, writable: true, configurable: true },
  });
  await fireEvent.scroll(list);
  await screen.findByText("Player 63");
  list.scrollTop = 1500;
  return list;
}
async function closeOutcome() {
  const dialog = await screen.findByRole("dialog", {
    name: "Operation result",
  });
  await fireEvent.click(
    within(dialog).getByRole("button", { name: "Close dialog" }),
  );
}

it("prepares twenty selected apps for uninstall once, without applying from the list", async () => {
  const api = setup(20),
    p = props();
  render(AppsPage, p);
  await screen.findByText("Player 19");
  await fireEvent.click(screen.getByRole("button", { name: "Select" }));
  await fireEvent.change(screen.getByLabelText("Bulk app action"), {
    target: { value: "uninstall" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Select shown" }));
  await fireEvent.click(screen.getByRole("button", { name: "Review 20 apps" }));
  await waitFor(() => expect(p.onbulk).toHaveBeenCalledOnce());
  expect(api.prepareBulk).toHaveBeenCalledWith(
    target,
    Array.from({ length: 20 }, (_, i) => `org.app${i}`),
    "uninstall",
  );
  expect(api.applyBulk).not.toHaveBeenCalled();
});

it("caps explicit selection at 32 loaded apps and can clear it without changing the TV", async () => {
  const api = setup();
  render(AppsPage, props());
  await loadSecondPage();
  await fireEvent.click(screen.getByRole("button", { name: "Select" }));
  await fireEvent.click(screen.getByRole("button", { name: "Select shown" }));
  expect(
    screen
      .getAllByRole("checkbox")
      .filter((box) => (box as HTMLInputElement).checked),
  ).toHaveLength(32);
  expect(
    (screen.getByRole("button", { name: "Select shown" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  await fireEvent.click(
    screen.getByRole("button", { name: "Clear selection" }),
  );
  expect(
    screen
      .getAllByRole("checkbox")
      .some((box) => (box as HTMLInputElement).checked),
  ).toBe(false);
  expect(api.prepareBulk).not.toHaveBeenCalled();
});

it("shows all batch candidates and exclusions together, with one confirmation", async () => {
  const api = setup();
  render(BulkReviewView, {
    review: bulk(Array.from({ length: 20 }, (_, i) => `org.app${i}`)),
    api: native.api,
    deviceName: "TV",
    onclose: vi.fn(),
    onresult: vi.fn(),
  });
  for (let i = 0; i < 20; i++)
    expect(
      await screen.findByText(`org.app${i}`, { exact: true }),
    ).toBeTruthy();
  expect(screen.getByText("org.system", { exact: true })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Next" })).toBeNull();
  const apply = screen.getByRole("button", {
    name: "Apply 20 changes",
  }) as HTMLButtonElement;
  expect(apply.disabled).toBe(true);
  await fireEvent.click(
    screen.getByRole("checkbox", { name: /Apply the included changes/ }),
  );
  await fireEvent.click(apply);
  await waitFor(() =>
    expect(api.applyBulk).toHaveBeenCalledExactlyOnceWith("batch", true),
  );
});

it("keeps the same list, loaded depth and position after a single uninstall", async () => {
  const api = setup();
  render(WorkspaceScreen, {
    page: "apps",
    device,
    ondevice: vi.fn(),
    onnavigate: vi.fn(),
  });
  const list = await loadSecondPage();
  await fireEvent.click(screen.getByRole("button", { name: /^Player 50/ }));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Uninstall" }),
  );
  await fireEvent.click(
    await screen.findByRole("checkbox", { name: /Apply this change/ }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Apply change" }));
  await closeOutcome();
  await waitFor(() => expect(screen.queryByText("Player 50")).toBeNull());
  expect(screen.getByRole("region", { name: "Installed apps" })).toBe(list);
  expect(list.scrollTop).toBe(1500);
  expect(screen.getByText("Player 63")).toBeTruthy();
  list.scrollTop = 2500;
  await fireEvent.scroll(list);
  await screen.findByText("Player 79");
  expect(api.apply).toHaveBeenCalledOnce();
});

it("keeps selection mode and failed selections after a partial batch", async () => {
  setup();
  render(WorkspaceScreen, {
    page: "apps",
    device,
    ondevice: vi.fn(),
    onnavigate: vi.fn(),
  });
  const list = await loadSecondPage();
  await fireEvent.click(screen.getByRole("button", { name: "Select" }));
  for (const i of [50, 51])
    await fireEvent.click(screen.getByLabelText(`Select Player ${i}`));
  await fireEvent.change(screen.getByLabelText("Bulk app action"), {
    target: { value: "uninstall" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Review 2 apps" }));
  await fireEvent.click(
    await screen.findByRole("checkbox", { name: /Apply the included changes/ }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Apply 2 changes" }),
  );
  await closeOutcome();
  await waitFor(() =>
    expect(screen.queryByLabelText("Select Player 50")).toBeNull(),
  );
  expect(
    (screen.getByLabelText("Select Player 51") as HTMLInputElement).checked,
  ).toBe(true);
  expect(screen.getByRole("button", { name: "Review 1 app" })).toBeTruthy();
  expect(screen.getByRole("region", { name: "Installed apps" })).toBe(list);
  expect(list.scrollTop).toBe(1500);
});

it("retains the loaded list on a refresh failure, then resets for a different view", async () => {
  const api = setup(),
    p = props();
  const view = render(AppsPage, { ...p, reload: 0 });
  const list = await loadSecondPage();
  await api.apply(); // The TV changed; old page offsets are no longer safe.
  api.query.mockRejectedValueOnce(new Error("TV unavailable"));
  await view.rerender({ ...p, reload: 1 });
  await screen.findByText("TV unavailable");
  expect(screen.getByText("Player 63")).toBeTruthy();
  expect(list.scrollTop).toBe(1500);
  const queries = api.query.mock.calls.length;
  list.scrollTop = 2500;
  await fireEvent.scroll(list);
  expect(api.query).toHaveBeenCalledTimes(queries);
  await fireEvent.click(
    screen.getByRole("button", { name: "Retry refreshing apps" }),
  );
  await waitFor(() => expect(screen.queryByText("Player 50")).toBeNull());
  expect(screen.getByText("Player 64")).toBeTruthy();
  await fireEvent.scroll(list);
  await screen.findByText("Player 79");
  await view.rerender({ ...p, reload: 1, filter: "user" });
  await waitFor(() => expect(screen.queryByText("Player 63")).toBeNull());
  expect(list.scrollTop).toBe(0);
});
