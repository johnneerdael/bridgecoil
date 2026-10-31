import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Device } from "../model";
export type Destination =
  "device" | "apps" | "remote" | "tools" | "history" | "connect" | "install";
export const destinations = [
  { id: "device", label: "Device", note: "Connection, profiles and setup" },
  { id: "apps", label: "Apps", note: "Browse and manage applications" },
  { id: "remote", label: "Remote", note: "Control your TV and buttons" },
  { id: "tools", label: "Tools", note: "Launcher, files and diagnostics" },
  { id: "history", label: "History", note: "Results and available recovery" },
] as const;
export type Target = { deviceId: string; sessionId: string; profileId: number };
export const targetOf = (device: Device): Target => ({
  deviceId: device.deviceId,
  sessionId: device.id,
  profileId: device.profileId,
});
export type Page<T> = {
  items: T[];
  page: number;
  pageSize: number;
  total: number;
};
export type Operation = {
  id: string;
  target: Target;
  title: string;
  kind: string;
  deviceLabel?: string | null;
  status:
    | "queued"
    | "running"
    | "awaitingUser"
    | "sent"
    | "verified"
    | "failed"
    | "partial"
    | "unknown"
    | "cancelled";
  output: string;
  error: string | null;
  recovery: unknown;
  createdMs: number;
};
export type Review = {
  id: string;
  target: Target;
  title: string;
  effect: string;
  currentState: string;
  losses: string[];
  reversible: boolean;
  commands: string[];
  expiresMs: number;
};
export type BulkReview = {
  id: string;
  target: Target;
  items: { package: string; review: Review }[];
  excluded: { package: string; reason: string }[];
  expiresMs: number;
};
export function sameTarget(a: Target, b: Target) {
  return (
    a.deviceId === b.deviceId &&
    a.sessionId === b.sessionId &&
    a.profileId === b.profileId
  );
}
export type PackageAction =
  | "enable"
  | "disable"
  | "resetEnabled"
  | "forceStop"
  | "remove"
  | "restore"
  | "clearData"
  | "clearCache";
export type Action =
  | { kind: "package"; package: string; action: PackageAction }
  | { kind: "shell"; command: string }
  | { kind: "defaultHome"; component: string }
  | { kind: "setting"; namespace: string; key: string; value: string | null }
  | { kind: "permission"; package: string; permission: string; grant: boolean }
  | { kind: "appOp"; package: string; op: string; mode: string };
export type Query =
  | {
      kind:
        | "device"
        | "profiles"
        | "launchers"
        | "toolkit"
        | "settings"
        | "network";
    }
  | {
      kind: "packages";
      filter: string;
      search: string;
      page: number;
      pageSize: number;
    }
  | { kind: "package"; package: string }
  | { kind: "appOps"; package: string }
  | { kind: "diagnostics"; section: string }
  | { kind: "files"; path: string; page: number; pageSize: number };
export type RemoteAction =
  | { kind: "key"; key: string; longPress: boolean }
  | { kind: "text"; text: string }
  | { kind: "launch"; component: string };
export type Profile = { id: number; name: string; running: boolean };
export type AppMetadata = {
  package: string;
  label: string | null;
  icon: string | null;
  activities?: string[];
  permissions?: { name: string; granted: boolean; protection: number | null }[];
  signers?: string[];
  error?: string | null;
};
export type PackageDetail = {
  launchActivity?: string | null;
  metadata?: AppMetadata | null;
  package: string;
  state: {
    installed: boolean | null;
    enabled: number | null;
    stopped: boolean | null;
    version: number | null;
  };
  versionName: string;
  activities: string[];
  permissions: string[];
  raw: string;
};
export type Platform = {
  readonly native: boolean;
  prepareBulk(
    target: Target,
    packages: string[],
    action: PackageAction,
  ): Promise<BulkReview>;
  applyBulk(id: string, stopAfterFailure: boolean): Promise<Operation>;
  cancelBulk(id: string): Promise<void>;
  discardBulk(id: string): Promise<void>;
  query<T>(target: Target, query: Query): Promise<T>;
  prepare(target: Target, action: Action): Promise<Review>;
  apply(id: string): Promise<Operation>;
  discard(id: string): Promise<void>;
  remote(target: Target, action: RemoteAction): Promise<Operation>;
  history(page: number, pageSize: number): Promise<Page<Operation>>;
  recovery(id: string): Promise<Review>;
  clearHistory(): Promise<void>;
  profile(target: Target, id: number): Promise<Device>;
};
export function nativePlatform(
  ondevice: (device: Device | null) => void,
): Platform {
  async function call<T>(
    name: string,
    args?: Record<string, unknown>,
  ): Promise<T> {
    if (!isTauri())
      throw new Error(
        "Use the installed Bridgecoil app for device operations.",
      );
    try {
      return await invoke<T>(name, args);
    } finally {
      void invoke<Device | null>("connection_status")
        .then(ondevice)
        .catch(() => ondevice(null));
    }
  }
  return {
    native: isTauri(),
    prepareBulk: (target, packages, action) =>
      call("prepare_bulk", { target, packages, action }),
    applyBulk: (reviewId, stopAfterFailure) =>
      call("apply_bulk", { reviewId, stopAfterFailure }),
    cancelBulk: (reviewId) => call("cancel_bulk", { reviewId }),
    discardBulk: (reviewId) => call("discard_bulk", { reviewId }),
    query: (target, query) => call("workspace_query", { target, query }),
    prepare: (target, action) => call("prepare_action", { target, action }),
    apply: (reviewId) => call("apply_action", { reviewId }),
    discard: (reviewId) => call("discard_review", { reviewId }),
    remote: (target, action) => call("remote_action", { target, action }),
    history: (page, pageSize) => call("list_history", { page, pageSize }),
    recovery: (operationId) => call("prepare_recovery", { operationId }),
    clearHistory: () => call("clear_history"),
    profile: (target, profileId) =>
      call("select_profile", { target, profileId }),
  };
}
