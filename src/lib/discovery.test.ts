import { it, expect } from "vitest";
import {
  reconnectService,
  displayServices,
  type DiscoveredService,
} from "./discovery";
import type { SavedDevice } from "./saved-devices";
const tv: SavedDevice = {
  deviceId: "android:tv",
  name: "TV",
  host: "192.0.2.12",
  endpoint: "192.0.2.12:40000",
  legacy: false,
  pairedAt: 1,
  lastConnectedAt: 2,
};
const service = (
  kind: DiscoveredService["kind"],
  port: number,
  host = tv.host,
  name = "adb-tv",
): DiscoveredService => ({ kind, name, host, endpoint: `${host}:${port}` });
it("shows one row per service rather than repeating all its IPv6 addresses", () => {
  const ipv4 = service("connect", 45000);
  const ipv6 = {
    ...ipv4,
    host: "2001:db8::1",
    endpoint: "[2001:db8::1]:45000",
  };
  const pairing = service("pairing", 35000);
  expect(displayServices([ipv6, ipv4, pairing])).toEqual([ipv4, pairing]);
});
it("refreshes only the connection port and never substitutes a pairing or legacy port", () => {
  const connect = service("connect", 45000);
  expect(
    reconnectService(
      tv,
      [service("pairing", 35000), service("legacy", 5555), connect],
      [],
    ),
  ).toBe(connect);
  expect(
    reconnectService(
      tv,
      [service("pairing", 35000), service("legacy", 5555)],
      [],
    ),
  ).toBeNull();
});
it("uses a learned instance name across DHCP address changes", () => {
  const moved = service("connect", 45000, "192.0.2.15");
  expect(
    reconnectService({ ...tv, discoveryName: "adb-tv" }, [moved], []),
  ).toBe(moved);
});
it("matches a pairing GUID only when the saved device also has a stable identity", () => {
  const moved = service("connect", 45000, "192.0.2.15", "adb-guid123-random");
  const pairs = [{ host: tv.host, guid: "guid123", pairedAt: 1 }];
  expect(reconnectService(tv, [moved], pairs)).toBe(moved);
  expect(
    reconnectService({ ...tv, deviceId: null }, [moved], pairs),
  ).toBeNull();
});
it("does not guess between two advertised instances at a reused host", () => {
  expect(
    reconnectService(
      tv,
      [
        service("connect", 45000),
        service("connect", 45001, tv.host, "adb-other"),
      ],
      [],
    ),
  ).toBeNull();
});
