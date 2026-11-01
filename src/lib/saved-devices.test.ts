import { describe, it, expect } from "vitest";
import {
  deleteSaved,
  loadSavedDevices,
  renameSaved,
  savedName,
  rememberConnected,
  rememberPaired,
} from "./saved-devices";
import type { Device } from "./model";
const tv: Device = {
  id: "session",
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
it("migrates the previous connection address without requiring pairing again", () => {
  const saved = loadSavedDevices(
    null,
    [{ host: "192.0.2.12", guid: "guid", pairedAt: 10 }],
    tv.endpoint,
  );
  expect(saved).toHaveLength(1);
  expect(saved[0]).toMatchObject({
    endpoint: tv.endpoint,
    pairedAt: 10,
    legacy: false,
  });
});
it("persists connected devices by stable identity across address and port changes", () => {
  let saved = rememberPaired([], tv.endpoint, 10);
  saved = rememberConnected(saved, tv, 20);
  saved = loadSavedDevices(JSON.stringify(saved), [], null);
  expect(saved[0]).toMatchObject({
    name: "Smart TV Pro",
    deviceId: tv.deviceId,
    pairedAt: 10,
    lastConnectedAt: 20,
  });
  saved = rememberConnected(saved, { ...tv, endpoint: "192.0.2.15:48000" }, 30);
  expect(saved).toHaveLength(1);
  expect(saved[0].endpoint).toBe("192.0.2.15:48000");
});
it("keeps paired-only entries and never treats their pairing port as a connection port", () => {
  const saved = rememberPaired([], "192.0.2.12:35077", 10);
  expect(saved[0].endpoint).toBeNull();
  expect(saved[0].host).toBe("192.0.2.12");
});
it("rejects malformed storage and does not replace another known device at a reused address", () => {
  expect(loadSavedDevices("broken", [], "bad address")).toEqual([]);
  let saved = rememberConnected([], tv, 20);
  saved = rememberConnected(
    saved,
    { ...tv, deviceId: "android:other", model: "Other TV" },
    30,
  );
  expect(saved).toHaveLength(2);
  expect(loadSavedDevices(JSON.stringify(saved), [], null)).toHaveLength(2);
});
describe("deleteSaved", () => {
  const pairs = [
    { host: "192.0.2.12", guid: "a", pairedAt: 10 },
    { host: "192.0.2.77", guid: "b", pairedAt: 11 },
  ];
  it("takes the pairing and last address too, so a restart can't rebuild the TV", () => {
    const saved = loadSavedDevices(
      JSON.stringify(rememberConnected(rememberPaired([], tv.endpoint, 10), tv, 20)),
      pairs,
      tv.endpoint,
    );
    const target = saved.find((d) => d.host === "192.0.2.12")!;
    const left = deleteSaved(saved, pairs, tv.endpoint, target);
    expect(left.devices.map((d) => d.host)).toEqual(["192.0.2.77"]);
    expect(left.pairs.map((p) => p.host)).toEqual(["192.0.2.77"]);
    expect(left.lastEndpoint).toBeNull();
    expect(
      loadSavedDevices(
        JSON.stringify(left.devices),
        left.pairs,
        left.lastEndpoint,
      ).map((d) => d.host),
    ).toEqual(["192.0.2.77"]);
  });
  it("leaves another TV's last address alone", () => {
    const saved = rememberConnected([], tv, 20);
    const left = deleteSaved(saved, [], "192.0.2.77:5555", saved[0]);
    expect(left.lastEndpoint).toBe("192.0.2.77:5555");
  });
  it("doesn't leave a stale port that the next launch would turn into a TV", () => {
    let saved = rememberConnected([], tv, 20);
    saved = rememberConnected(
      saved,
      {
        ...tv,
        deviceId: "android:other",
        model: "Other TV",
        endpoint: "192.0.2.12:5555",
      },
      30,
    );
    const left = deleteSaved(
      saved,
      [],
      tv.endpoint,
      saved.find((d) => d.name === "Smart TV Pro")!,
    );
    expect(left.lastEndpoint).toBeNull();
    expect(
      loadSavedDevices(JSON.stringify(left.devices), [], left.lastEndpoint),
    ).toHaveLength(1);
  });
  it("keeps the pairing when another saved TV uses the same address", () => {
    let saved = rememberConnected([], tv, 20);
    saved = rememberConnected(
      saved,
      { ...tv, deviceId: "android:other", model: "Other TV" },
      30,
    );
    const left = deleteSaved(
      saved,
      pairs,
      tv.endpoint,
      saved.find((d) => d.name === "Smart TV Pro")!,
    );
    expect(left.devices.map((d) => d.name)).toEqual(["Other TV"]);
    expect(left.pairs).toEqual(pairs);
    expect(left.lastEndpoint).toBe(tv.endpoint);
  });
});
describe("renameSaved", () => {
  it("keeps the chosen name through a reconnect and a restart", () => {
    let saved = renameSaved(
      rememberConnected([], tv, 20),
      rememberConnected([], tv, 20)[0],
      "  Living room  ",
    );
    expect(saved[0].label).toBe("Living room");
    saved = rememberConnected(saved, { ...tv, endpoint: "192.0.2.15:48000" }, 30);
    saved = loadSavedDevices(JSON.stringify(saved), [], null);
    expect(savedName(saved[0])).toBe("Living room");
    expect(saved[0].name).toBe("Smart TV Pro");
  });
  it("goes back to the TV's own name when the new name is empty", () => {
    const saved = rememberConnected([], tv, 20);
    const named = renameSaved(saved, saved[0], "Bedroom");
    const cleared = renameSaved(named, named[0], "   ");
    expect(cleared[0].label).toBeUndefined();
    expect(savedName(cleared[0])).toBe("Smart TV Pro");
  });
  it("renames only that TV, and limits the length", () => {
    let saved = rememberConnected([], tv, 20);
    saved = rememberConnected(
      saved,
      { ...tv, deviceId: "android:other", model: "Other TV" },
      30,
    );
    const target = saved.find((d) => d.name === "Other TV")!;
    const named = renameSaved(saved, target, "x".repeat(200));
    expect(named.find((d) => d.name === "Other TV")!.label).toHaveLength(60);
    expect(named.find((d) => d.name === "Smart TV Pro")!.label).toBeUndefined();
  });
  it("ignores a stored name that isn't valid", () => {
    const saved = rememberConnected([], tv, 20);
    const stored = JSON.stringify([{ ...saved[0], label: 42 }]);
    expect(loadSavedDevices(stored, [], null)).toEqual([]);
  });
});
describe("hardware addresses", () => {
  const addresses = ["90:0e:b3:fd:fe:d0"];
  it("keeps them through a restart, and through a connection that can't read them", () => {
    let saved = rememberConnected([], { ...tv, hardwareAddresses: addresses }, 20);
    saved = loadSavedDevices(JSON.stringify(saved), [], null);
    expect(saved[0].hardwareAddresses).toEqual(addresses);
    saved = rememberConnected(saved, { ...tv, hardwareAddresses: [] }, 30);
    expect(saved[0].hardwareAddresses).toEqual(addresses);
  });
  it("ignores stored values that aren't addresses", () => {
    const saved = rememberConnected([], tv, 20);
    for (const bad of ["x", ["x"], [1], Array(5).fill(addresses[0])])
      expect(
        loadSavedDevices(
          JSON.stringify([{ ...saved[0], hardwareAddresses: bad }]),
          [],
          null,
        ),
      ).toEqual([]);
  });
});
