import { endpointHost, type PairRecord } from "./connection-model";
import type { Device } from "./model";

export const savedDevicesKey = "bridgecoil.devices.v1";
export const labelLimit = 60;
export type SavedDevice = {
  discoveryName?: string;
  deviceId: string | null;
  /** The TV's own model name. It's replaced on every connection. */
  name: string;
  /** The name someone chose. It wins over `name` wherever a TV is named. */
  label?: string;
  host: string;
  endpoint: string | null;
  legacy: boolean;
  pairedAt: number | null;
  lastConnectedAt: number | null;
};
const timestamp = (v: unknown): v is number | null =>
  v === null ||
  (typeof v === "number" &&
    Number.isFinite(v) &&
    v >= 0 &&
    v <= 8640000000000000);
function validEndpoint(value: unknown): value is string {
  if (
    typeof value !== "string" ||
    value.length > 320 ||
    /[\s\x00-\x1f]/.test(value)
  )
    return false;
  const host = endpointHost(value),
    port = value.slice(value.lastIndexOf(":") + 1);
  return (
    !!host &&
    host.length <= 255 &&
    /^\d{1,5}$/.test(port) &&
    +port > 0 &&
    +port <= 65535
  );
}
function bounded(records: SavedDevice[]): SavedDevice[] {
  return records
    .sort(
      (a, b) =>
        (b.lastConnectedAt ?? b.pairedAt ?? 0) -
        (a.lastConnectedAt ?? a.pairedAt ?? 0),
    )
    .slice(0, 32);
}
export function loadSavedDevices(
  raw: string | null,
  pairs: PairRecord[],
  previousEndpoint: string | null,
): SavedDevice[] {
  let records: SavedDevice[] = [];
  try {
    const parsed: unknown = JSON.parse(raw ?? "[]");
    if (Array.isArray(parsed))
      records = parsed
        .filter(
          (v): v is SavedDevice =>
            !!v &&
            typeof v === "object" &&
            (v.deviceId === null ||
              (typeof v.deviceId === "string" && v.deviceId.length <= 128)) &&
            (v.discoveryName === undefined ||
              (typeof v.discoveryName === "string" &&
                v.discoveryName.length <= 160)) &&
            (v.label === undefined ||
              (typeof v.label === "string" &&
                v.label.length > 0 &&
                v.label.length <= labelLimit)) &&
            typeof v.name === "string" &&
            v.name.length > 0 &&
            v.name.length <= 160 &&
            typeof v.host === "string" &&
            v.host.length > 0 &&
            v.host.length <= 255 &&
            (v.endpoint === null ||
              (validEndpoint(v.endpoint) &&
                endpointHost(v.endpoint) === v.host)) &&
            typeof v.legacy === "boolean" &&
            timestamp(v.pairedAt) &&
            timestamp(v.lastConnectedAt),
        )
        .slice(0, 32);
  } catch {
    /* Corrupt optional metadata does not affect the app's pairing key. */
  }
  if (
    validEndpoint(previousEndpoint) &&
    !records.some((r) => r.endpoint === previousEndpoint)
  ) {
    const host = endpointHost(previousEndpoint)!;
    records.push({
      deviceId: null,
      name: host,
      host,
      endpoint: previousEndpoint,
      legacy: false,
      pairedAt: null,
      lastConnectedAt: 0,
    });
  }
  for (const pair of pairs)
    records = rememberPaired(
      records,
      `${pair.host.includes(":") ? `[${pair.host}]` : pair.host}:1`,
      pair.pairedAt,
    );
  return bounded(records);
}
export function rememberPaired(
  records: SavedDevice[],
  pairingEndpoint: string,
  now = Date.now(),
): SavedDevice[] {
  const host = endpointHost(pairingEndpoint);
  if (!host) return records;
  const existing = records.find((r) => r.host === host);
  const entry: SavedDevice = existing
    ? { ...existing, pairedAt: now }
    : {
        deviceId: null,
        name: host,
        host,
        endpoint: null,
        legacy: false,
        pairedAt: now,
        lastConnectedAt: null,
      };
  return bounded([entry, ...records.filter((r) => r !== existing)]);
}
export function rememberConnected(
  records: SavedDevice[],
  device: Device,
  now = Date.now(),
): SavedDevice[] {
  const host = endpointHost(device.endpoint);
  if (!host || !validEndpoint(device.endpoint)) return records;
  const deviceId = device.deviceId.startsWith("session:")
    ? null
    : device.deviceId;
  const previous =
    records.find((r) => deviceId && r.deviceId === deviceId) ??
    records.find((r) => r.host === host && !r.deviceId);
  return bounded([
    {
      deviceId,
      ...(previous?.discoveryName
        ? { discoveryName: previous.discoveryName }
        : {}),
      ...(previous?.label ? { label: previous.label } : {}),
      name: device.model.slice(0, 160) || host,
      host,
      endpoint: device.endpoint.trim(),
      legacy: !device.encrypted,
      pairedAt: previous?.pairedAt ?? null,
      lastConnectedAt: now,
    },
    ...records.filter(
      (r) => r !== previous && !(!r.deviceId && r.host === host),
    ),
  ]);
}
const sameDevice = (a: SavedDevice, b: SavedDevice) =>
  a.deviceId === b.deviceId && a.host === b.host && a.endpoint === b.endpoint;
export const savedName = (saved: SavedDevice) => saved.label ?? saved.name;
/** An empty name goes back to the TV's own. */
export function renameSaved(
  records: SavedDevice[],
  saved: SavedDevice,
  name: string,
): SavedDevice[] {
  const label = name.trim().slice(0, labelLimit);
  return records.map((r) => {
    if (!sameDevice(r, saved)) return r;
    const { label: _old, ...rest } = r;
    return label ? { ...rest, label } : rest;
  });
}
export const homePickKey = (deviceId: string) => `bridgecoil.home.${deviceId}`;
export const homeProvenKey = (deviceId: string) =>
  `bridgecoil.home.proven.${deviceId}`;
/**
 * What's left once a saved TV is deleted. Its pairing and last address go too,
 * because loadSavedDevices rebuilds a TV from either one on the next launch.
 * Another saved TV can share the address, and then both of those are still in use.
 */
export function deleteSaved(
  records: SavedDevice[],
  pairs: PairRecord[],
  lastEndpoint: string | null,
  saved: SavedDevice,
) {
  const devices = records.filter((r) => !sameDevice(r, saved));
  const shared = devices.some((r) => r.host === saved.host);
  return {
    devices,
    pairs: shared ? pairs : pairs.filter((p) => p.host !== saved.host),
    lastEndpoint:
      lastEndpoint &&
      endpointHost(lastEndpoint) === saved.host &&
      !devices.some((r) => r.endpoint === lastEndpoint)
        ? null
        : lastEndpoint,
  };
}
