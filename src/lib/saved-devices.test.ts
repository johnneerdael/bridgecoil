import { describe, it, expect } from "vitest";
import {
  deleteSaved,
  loadSavedDevices,
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
