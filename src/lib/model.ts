export type Device = {
  id: string;
  deviceId: string;
  profileId: number;
  serial: string;
  build: string;
  manufacturer: string;
  model: string;
  endpoint: string;
  encrypted: boolean;
  androidVersion: string;
  sdk: number;
  abis: string[];
  hardwareAddresses?: string[];
};
export type Selection = {
  id: string;
  sessionId: string;
  name: string;
  package: string;
  versionCode: number;
  versionName?: string;
  bytes: number;
  sha256: string;
  minSdk?: number;
  abis: string[];
  /** How many APKs this TV gets, out of how many are in the file. */
  parts: number;
  available: number;
  gameData: number;
  warnings: string[];
};
export function canInstall(
  device: Device | null,
  selection: Selection | null,
  busy: boolean,
): boolean {
  return (
    !busy &&
    device !== null &&
    selection !== null &&
    device.id === selection.sessionId
  );
}
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  return "The operation could not be completed.";
}
