import {
  render,
  screen,
  fireEvent,
  cleanup,
  waitFor,
} from "@testing-library/svelte";
import { afterAll, afterEach, beforeAll, describe, it, expect, vi } from "vitest";
import App from "./App.svelte";
import { savedDevicesKey } from "./lib/saved-devices";
import { pairStoreKey } from "./lib/connection-model";
import { wakeTiming } from "./lib/wake";
import { keep, scrollback } from "./lib/workspace/shell-line";
import type { Device } from "./lib/model";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke,
  Channel: class {
    onmessage = () => {};
  },
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
const tv: Device = {
  id: "fresh-session",
  deviceId: "android:tv",
  profileId: 0,
  serial: "tv",
  build: "build",
  manufacturer: "TCL",
  model: "Smart TV Pro",
  endpoint: "192.0.2.12:46833",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: [],
};
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
  localStorage.clear();
  invoke.mockReset();
});
function backend(services: unknown[] = []) {
  let active: Device | null = null;
  invoke.mockImplementation(async (command, args) => {
    if (command === "connection_status") return active;
    if (command === "discover_adb") return { services, warnings: [] };
    if (command === "disconnect_device") {
      active = null;
      return;
    }
    if (command === "connect_device") {
      active = { ...tv, endpoint: args.endpoint };
      return active;
    }
    if (command === "workspace_query")
      return args.query.kind === "profiles"
        ? [{ id: 0, name: "Owner", running: true }]
        : {};
    return undefined;
  });
}
it("recovers the old connection and remembers the TV for one-tap reconnect after restart", async () => {
  backend();
  localStorage.setItem("adb.connectionEndpoint", tv.endpoint);
  const first = render(App);
  const quick = await screen.findByRole("button", {
    name: "Quick connect to 192.0.2.12",
  });
  await waitFor(() =>
    expect((quick as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(quick);
  await screen.findByRole("button", { name: "Connection status: Connected" });
  expect(invoke).toHaveBeenCalledWith(
    "connect_device",
    expect.objectContaining({ endpoint: tv.endpoint, allowLegacy: false }),
  );
  expect(
    invoke.mock.calls.filter(([command]) => command === "connect_device"),
  ).toHaveLength(1);
  first.unmount();
  backend();
  render(App);
  expect(
    await screen.findByRole("button", {
      name: "Quick connect to Smart TV Pro",
    }),
  ).toBeTruthy();
  expect(
    screen.getByRole("button", { name: "Connection status: Not connected" }),
  ).toBeTruthy();
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: "Quick connect to Smart TV Pro",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Quick connect to Smart TV Pro" }),
  );
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "connect_device",
      expect.objectContaining({ expectedDeviceId: "android:tv" }),
    ),
  );
});
it("can browse discovery while connected without disconnecting or changing the TV", async () => {
  backend([
    {
      name: "adb-other",
      kind: "connect",
      host: "192.0.2.15",
      endpoint: "192.0.2.15:49000",
    },
  ]);
  localStorage.setItem("adb.connectionEndpoint", tv.endpoint);
  render(App);
  const quick = await screen.findByRole("button", {
    name: "Quick connect to 192.0.2.12",
  });
  await waitFor(() =>
    expect((quick as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(quick);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Connection status: Connected" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Find devices" }),
  );
  await screen.findByRole("button", { name: /adb-other.*Wireless connection/ });
  expect(
    invoke.mock.calls.some(([command]) => command === "disconnect_device"),
  ).toBe(false);
  expect(
    invoke.mock.calls.filter(([command]) => command === "connect_device"),
  ).toHaveLength(1);
});
it("Quick connect discovers a changed TLS port while ignoring the pairing port", async () => {
  const service = {
    name: "adb-tv",
    kind: "connect",
    host: "192.0.2.12",
    endpoint: "192.0.2.12:49000",
  };
  backend([
    service,
    { ...service, kind: "pairing", endpoint: "192.0.2.12:35000" },
  ]);
  localStorage.setItem("adb.connectionEndpoint", tv.endpoint);
  render(App);
  const quick = await screen.findByRole("button", {
    name: "Quick connect to 192.0.2.12",
  });
  await waitFor(() =>
    expect((quick as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(quick);
  await screen.findByRole("button", { name: "Connection status: Connected" });
  expect(invoke).toHaveBeenCalledWith(
    "connect_device",
    expect.objectContaining({ endpoint: service.endpoint, allowLegacy: false }),
  );
  expect(
    JSON.parse(localStorage.getItem(savedDevicesKey)!)[0].discoveryName,
  ).toBe("adb-tv");
});
it("cancelling during discovery prevents a connection when its late result arrives", async () => {
  let complete!: (value: unknown) => void;
  invoke.mockImplementation(async (command) => {
    if (command === "connection_status") return null;
    if (command === "discover_adb")
      return new Promise((resolve) => (complete = resolve));
  });
  localStorage.setItem("adb.connectionEndpoint", tv.endpoint);
  render(App);
  const quick = await screen.findByRole("button", {
    name: "Quick connect to 192.0.2.12",
  });
  await waitFor(() =>
    expect((quick as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(quick);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Cancel attempt" }),
  );
  complete({ services: [], warnings: [] });
  await screen.findByText("Connection cancelled.", { selector: ".feedback p" });
  expect(
    invoke.mock.calls.some(([command]) => command === "connect_device"),
  ).toBe(false);
});
it("reports denied discovery access instead of falling back to a connection attempt", async () => {
  invoke.mockImplementation(async (command) => {
    if (command === "connection_status") return null;
    if (command === "discover_adb") throw { kind: "permission", message: "Allow local network access." };
  });
  localStorage.setItem("adb.connectionEndpoint", tv.endpoint);
  render(App);
  const quick = await screen.findByRole("button", { name: "Quick connect to 192.0.2.12" });
  await waitFor(() => expect((quick as HTMLButtonElement).disabled).toBe(false));
  await fireEvent.click(quick);
  expect((await screen.findByRole("alert")).textContent).toContain("Allow local network access.");
  expect(invoke.mock.calls.some(([command]) => command === "connect_device")).toBe(false);
});
it("choosing a discovered pairing port fills the form without pairing or connecting", async () => {
  backend([
    {
      name: "adb-tv",
      kind: "pairing",
      host: "192.0.2.12",
      endpoint: "192.0.2.12:35000",
    },
  ]);
  render(App);
  const find = screen.getByRole("button", { name: "Find nearby ADB devices" });
  await waitFor(() => expect((find as HTMLButtonElement).disabled).toBe(false));
  await fireEvent.click(find);
  await fireEvent.click(
    await screen.findByRole("button", { name: /adb-tv.*Pairing port/ }),
  );
  expect(
    (screen.getByLabelText(/Pairing address/) as HTMLInputElement).value,
  ).toBe("192.0.2.12:35000");
  expect(
    invoke.mock.calls.some(
      ([command]) => command === "connect_device" || command === "pair_device",
    ),
  ).toBe(false);
});
it("keeps saved TVs after a failed reconnect and offers address editing", async () => {
  backend();
  localStorage.setItem(
    savedDevicesKey,
    JSON.stringify([
      {
        deviceId: tv.deviceId,
        name: tv.model,
        host: "192.0.2.12",
        endpoint: tv.endpoint,
        legacy: false,
        pairedAt: 10,
        lastConnectedAt: 20,
      },
    ]),
  );
  invoke.mockImplementation(async (command) => {
    if (command === "connect_device")
      throw { kind: "connection", message: "TV is offline." };
    if (command === "connection_status") return null;
  });
  render(App);
  const quick = await screen.findByRole("button", {
    name: "Quick connect to Smart TV Pro",
  });
  await waitFor(() =>
    expect((quick as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(quick);
  expect((await screen.findByRole("alert")).textContent).toContain(
    "TV is offline.",
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "← Saved devices" }),
  );
  expect(
    screen.getByRole("button", { name: "Quick connect to Smart TV Pro" }),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Update connection address" }),
  );
  expect(
    (screen.getByLabelText(/Connection address/) as HTMLInputElement).value,
  ).toBe(tv.endpoint);
});
it("returns to saved TVs from the Device menu item while disconnected", async () => {
  localStorage.setItem(
    savedDevicesKey,
    JSON.stringify([
      {
        deviceId: tv.deviceId,
        name: tv.model,
        host: "192.0.2.12",
        endpoint: tv.endpoint,
        legacy: false,
        pairedAt: 10,
        lastConnectedAt: 20,
      },
    ]),
  );
  invoke.mockImplementation(async (command) => {
    if (command === "connect_device")
      throw { kind: "connection", message: "TV is offline." };
    if (command === "connection_status") return null;
  });
  render(App);
  const quick = await screen.findByRole("button", {
    name: "Quick connect to Smart TV Pro",
  });
  await waitFor(() =>
    expect((quick as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(quick);
  await screen.findByRole("alert");
  await fireEvent.click(
    screen.getByRole("button", { name: "Open navigation" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: /^Device/ }));
  expect(
    screen.getByRole("button", { name: "Quick connect to Smart TV Pro" }),
  ).toBeTruthy();
  expect(screen.getByRole("button", { name: "Add device" })).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
});
const stored = {
  deviceId: tv.deviceId,
  name: tv.model,
  host: "192.0.2.12",
  endpoint: tv.endpoint,
  legacy: false,
  pairedAt: 10,
  lastConnectedAt: 20,
};
const other = {
  ...stored,
  deviceId: "android:other",
  name: "Bedroom TV",
  host: "192.0.2.77",
  endpoint: "192.0.2.77:5555",
};
const warning = () =>
  document.querySelector("dialog[open]")!.textContent!.replace(/\s+/g, " ");
async function openWarning(name = tv.model) {
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: `Quick connect to ${name}`,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  await fireEvent.click(screen.getByRole("button", { name: `Delete ${name}` }));
  await screen.findByRole("heading", { name: "Delete this TV?" });
}
it("deletes a saved TV only after the warning, and a restart doesn't bring it back", async () => {
  backend();
  localStorage.setItem(savedDevicesKey, JSON.stringify([stored, other]));
  localStorage.setItem(
    pairStoreKey,
    JSON.stringify([{ host: stored.host, guid: "guid", pairedAt: 10 }]),
  );
  localStorage.setItem("adb.connectionEndpoint", tv.endpoint);
  localStorage.setItem(`bridgecoil.home.${tv.deviceId}`, "org.home");
  localStorage.setItem(`bridgecoil.home.proven.${tv.deviceId}`, "org.home");
  localStorage.setItem("bridgecoil.home.android:other", "org.kept");
  keep(tv.deviceId, [{ id: 1, command: "ls", output: "x", failed: false }]);
  const first = render(App);
  await openWarning();
  expect(warning()).toContain("records in History");
  expect(
    invoke.mock.calls.some(([command]) => command === "delete_tv_history"),
  ).toBe(false);
  await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(
    screen.getByRole("button", { name: `Quick connect to ${tv.model}` }),
  ).toBeTruthy();
  expect(
    invoke.mock.calls.some(([command]) => command === "delete_tv_history"),
  ).toBe(false);
  await openWarning();
  await fireEvent.click(screen.getByRole("button", { name: "Delete TV" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("button", { name: `Quick connect to ${tv.model}` }),
    ).toBeNull(),
  );
  expect(invoke).toHaveBeenCalledWith("delete_tv_history", {
    deviceId: tv.deviceId,
  });
  expect(localStorage.getItem("adb.connectionEndpoint")).toBeNull();
  expect(JSON.parse(localStorage.getItem(pairStoreKey)!)).toEqual([]);
  expect(localStorage.getItem(`bridgecoil.home.${tv.deviceId}`)).toBeNull();
  expect(
    localStorage.getItem(`bridgecoil.home.proven.${tv.deviceId}`),
  ).toBeNull();
  expect(localStorage.getItem("bridgecoil.home.android:other")).toBe(
    "org.kept",
  );
  expect(scrollback(tv.deviceId)).toEqual([]);
  first.unmount();
  backend();
  render(App);
  await screen.findByRole("button", { name: "Quick connect to Bedroom TV" });
  expect(
    screen.queryByRole("button", { name: `Quick connect to ${tv.model}` }),
  ).toBeNull();
});
it("keeps the TV saved when its History records can't be deleted", async () => {
  invoke.mockImplementation(async (command) => {
    if (command === "connection_status") return null;
    if (command === "delete_tv_history")
      throw { kind: "storage", message: "History is locked." };
  });
  localStorage.setItem(savedDevicesKey, JSON.stringify([stored]));
  render(App);
  await openWarning();
  await fireEvent.click(screen.getByRole("button", { name: "Delete TV" }));
  expect((await screen.findByRole("alert")).textContent).toContain(
    "History is locked.",
  );
  expect(
    screen.getByRole("button", { name: `Quick connect to ${tv.model}` }),
  ).toBeTruthy();
  expect(JSON.parse(localStorage.getItem(savedDevicesKey)!)).toHaveLength(1);
});
it("says History stays when a TV has no ID to match its records", async () => {
  backend();
  localStorage.setItem(
    savedDevicesKey,
    JSON.stringify([{ ...stored, deviceId: null }]),
  );
  render(App);
  await openWarning();
  expect(warning()).toContain("Use Clear records in History");
  await fireEvent.click(screen.getByRole("button", { name: "Delete TV" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("button", { name: `Quick connect to ${tv.model}` }),
    ).toBeNull(),
  );
  expect(
    invoke.mock.calls.some(([command]) => command === "delete_tv_history"),
  ).toBe(false);
});
it("doesn't carry a deleted TV's legacy choice over to the next connection", async () => {
  backend();
  localStorage.setItem(
    savedDevicesKey,
    JSON.stringify([{ ...stored, legacy: true }]),
  );
  render(App);
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: `Quick connect to ${tv.model}`,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Update connection address" }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "← Saved devices" }),
  );
  await openWarning();
  await fireEvent.click(screen.getByRole("button", { name: "Delete TV" }));
  const box = (await screen.findByLabelText(
    /Allow legacy/,
  )) as HTMLInputElement;
  expect(box.checked).toBe(false);
  expect(
    (screen.getByLabelText(/Connection address/) as HTMLInputElement).value,
  ).toBe("");
});
it("renames a saved TV, and the new name survives connecting and restarting", async () => {
  backend();
  localStorage.setItem(savedDevicesKey, JSON.stringify([stored]));
  const first = render(App);
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: `Quick connect to ${tv.model}`,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: `Rename ${tv.model}` }),
  );
  const field = (await screen.findByLabelText("TV name")) as HTMLInputElement;
  expect(field.value).toBe(tv.model);
  await fireEvent.input(field, { target: { value: "Living room" } });
  await fireEvent.click(screen.getByRole("button", { name: "Save name" }));
  const quick = await screen.findByRole("button", {
    name: "Quick connect to Living room",
  });
  await fireEvent.click(quick);
  await screen.findByRole("button", { name: "Connection status: Connected" });
  first.unmount();
  backend();
  render(App);
  expect(
    await screen.findByRole("button", { name: "Quick connect to Living room" }),
  ).toBeTruthy();
  expect(JSON.parse(localStorage.getItem(savedDevicesKey)!)[0].name).toBe(
    tv.model,
  );
});
describe("Wake-on-LAN", () => {
  const addresses = ["90:0e:b3:fd:fe:d0"];
  const asleep = (failures: number, kind = "connection") => {
    let left = failures;
    invoke.mockImplementation(async (command, args) => {
      if (command === "connection_status") return null;
      if (command === "discover_adb") return { services: [], warnings: [] };
      if (command === "wake_tv") return true;
      if (command === "workspace_query")
        return args.query.kind === "profiles"
          ? [{ id: 0, name: "Owner", running: true }]
          : {};
      if (command === "connect_device") {
        if (left-- > 0) throw { kind, message: "TV is asleep." };
        return { ...tv, endpoint: args.endpoint };
      }
    });
  };
  const connects = () =>
    invoke.mock.calls.filter(([command]) => command === "connect_device");
  const quick = async () => {
    const button = (await screen.findByRole("button", {
      name: `Quick connect to ${tv.model}`,
    })) as HTMLButtonElement;
    await waitFor(() => expect(button.disabled).toBe(false));
    await fireEvent.click(button);
  };
  const restore = wakeTiming.pause;
  beforeAll(() => {
    wakeTiming.pause = 1;
  });
  afterAll(() => {
    wakeTiming.pause = restore;
  });
  it("wakes the TV and keeps trying while it comes up", async () => {
    asleep(2);
    localStorage.setItem(
      savedDevicesKey,
      JSON.stringify([{ ...stored, hardwareAddresses: addresses }]),
    );
    render(App);
    await quick();
    await screen.findByRole("button", { name: "Connection status: Connected" });
    expect(invoke).toHaveBeenCalledWith("wake_tv", {
      addresses,
      host: stored.host,
    });
    expect(connects()).toHaveLength(3);
  });
  it("keeps trying while the TV's port still refuses", async () => {
    asleep(1, "connectionRefused");
    localStorage.setItem(
      savedDevicesKey,
      JSON.stringify([{ ...stored, hardwareAddresses: addresses }]),
    );
    render(App);
    await quick();
    await screen.findByRole("button", { name: "Connection status: Connected" });
    expect(connects()).toHaveLength(2);
  });
  it("doesn't delay a TV that's already awake", async () => {
    asleep(0);
    localStorage.setItem(
      savedDevicesKey,
      JSON.stringify([{ ...stored, hardwareAddresses: addresses }]),
    );
    render(App);
    await quick();
    await screen.findByRole("button", { name: "Connection status: Connected" });
    expect(connects()).toHaveLength(1);
  });
  it("stops after its last try and shows why", async () => {
    asleep(99);
    localStorage.setItem(
      savedDevicesKey,
      JSON.stringify([{ ...stored, hardwareAddresses: addresses }]),
    );
    render(App);
    await quick();
    expect((await screen.findByRole("alert")).textContent).toContain(
      "TV is asleep.",
    );
    expect(connects()).toHaveLength(wakeTiming.retries + 1);
  });
  it("doesn't retry something waking can't fix", async () => {
    asleep(99, "differentDevice");
    localStorage.setItem(
      savedDevicesKey,
      JSON.stringify([{ ...stored, hardwareAddresses: addresses }]),
    );
    render(App);
    await quick();
    await screen.findByRole("alert");
    expect(connects()).toHaveLength(1);
  });
  it("connects as before when the TV never gave an address, or the packet can't be sent", async () => {
    asleep(99);
    localStorage.setItem(savedDevicesKey, JSON.stringify([stored]));
    const first = render(App);
    await quick();
    await screen.findByRole("alert");
    expect(invoke).not.toHaveBeenCalledWith("wake_tv", expect.anything());
    expect(connects()).toHaveLength(1);
    first.unmount();
    invoke.mockClear();
    asleep(99);
    const refused = invoke.getMockImplementation()!;
    invoke.mockImplementation(async (command, args) => {
      if (command === "wake_tv") throw { kind: "permission", message: "No." };
      return refused(command, args);
    });
    localStorage.setItem(
      savedDevicesKey,
      JSON.stringify([{ ...stored, hardwareAddresses: addresses }]),
    );
    render(App);
    await quick();
    expect((await screen.findByRole("alert")).textContent).toContain(
      "TV is asleep.",
    );
    expect(connects()).toHaveLength(1);
  });
});
it("shows how much of a multi-part file this TV gets before installing it", async () => {
  backend();
  const fallback = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) =>
    String(command).includes("rating")
      ? { stored: false }
      : command === "choose_apk"
      ? {
          id: "selection",
          sessionId: "fresh-session",
          name: "game.apk",
          package: "com.example.game",
          versionCode: 12,
          bytes: 4096,
          sha256: "ab",
          abis: [],
          parts: 4,
          available: 58,
          gameData: 3 * 1048576,
          warnings: ["Some signatures cannot be verified locally."],
        }
      : fallback(command, args),
  );
  localStorage.setItem(savedDevicesKey, JSON.stringify([stored]));
  render(App);
  const quick = (await screen.findByRole("button", {
    name: `Quick connect to ${tv.model}`,
  })) as HTMLButtonElement;
  await waitFor(() => expect(quick.disabled).toBe(false));
  await fireEvent.click(quick);
  await screen.findByRole("button", { name: "Connection status: Connected" });
  await fireEvent.click(await screen.findByRole("button", { name: /Manage apps/ }));
  await fireEvent.click(await screen.findByRole("button", { name: /Install APK/ }));
  await fireEvent.click(await screen.findByRole("button", { name: /Choose file/ }));
  const review = (await screen.findByText(/parts in this file/)).closest("section")!;
  const text = review.textContent!.replace(/\s+/g, " ");
  expect(text).toContain("4 of 58 parts in this file are needed for this TV");
  expect(text).toContain("3.0 MB of game data");
  expect(text).toContain("Some signatures cannot be verified locally.");
  const install = screen.getByRole("button", { name: /Install app/ });
  expect((install as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.click(screen.getByRole("checkbox"));
  await fireEvent.click(install);
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("install_apk", {
      sessionId: "fresh-session",
      selectionId: "selection",
    }),
  );
});
