import type { SavedDevice } from "./saved-devices";
import { endpointHost, type PairRecord } from "./connection-model";
export type DiscoveredService = {
  name: string;
  kind: "connect" | "pairing" | "legacy";
  host: string;
  endpoint: string;
};
export type DiscoveryResult = {
  services: DiscoveredService[];
  warnings: string[];
};
export const serviceLabel: Record<DiscoveredService["kind"], string> = {
  connect: "Wireless connection",
  pairing: "Pairing port",
  legacy: "Legacy ADB",
};
// A TV may advertise many IPv6 addresses. Show one usable address per service.
export function displayServices(
  services: DiscoveredService[],
): DiscoveredService[] {
  const rows = new Map<string, DiscoveredService>();
  for (const service of services) {
    const key = `${service.kind}/${service.name}`;
    const previous = rows.get(key);
    if (
      !previous ||
      (previous.host.includes(":") && !service.host.includes(":"))
    )
      rows.set(key, service);
  }
  return [...rows.values()];
}

/** Advertisements are routing hints only; native connect still checks the saved device identity. */
export function reconnectService(
  saved: SavedDevice,
  services: DiscoveredService[],
  pairs: PairRecord[],
): DiscoveredService | null {
  const candidates = services.filter(
    (s) => s.kind === (saved.legacy ? "legacy" : "connect"),
  );
  const guid = pairs.find((p) => p.host === saved.host)?.guid;
  let matches = saved.discoveryName
    ? candidates.filter((s) => s.name === saved.discoveryName)
    : [];
  if (!matches.length && saved.deviceId && guid && guid.length >= 6) {
    const prefix = guid.startsWith("adb-") ? guid : `adb-${guid}`;
    matches = candidates.filter(
      (s) =>
        s.name === prefix ||
        s.name.startsWith(`${prefix}-`) ||
        s.name.startsWith(`${prefix} (`),
    );
  }
  if (!matches.length)
    matches = candidates.filter((s) => endpointHost(s.endpoint) === saved.host);
  // Multiple distinct instances require a user choice, even if they share an address.
  if (new Set(matches.map((s) => s.name)).size !== 1) return null;
  return (
    matches.find((s) => s.host === saved.host) ??
    matches.find((s) => !s.host.includes(":")) ??
    matches[0] ??
    null
  );
}
