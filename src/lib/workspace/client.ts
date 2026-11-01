import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import type { Device } from "../model";
import type { Group, Level, Rating, RatingsStatus } from "./ratings";
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
export type RecordingStep =
  | { kind: "start"; seconds: number }
  | { kind: "status" }
  | { kind: "stop" }
  | { kind: "discard" };
export type RecordingState = {
  running: boolean;
  size: number | null;
  path: string;
};
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
  rating?: Level | null;
};
export type BulkReview = {
  id: string;
  target: Target;
  items: { package: string; review: Review }[];
  excluded: { package: string; reason: string; rating?: Level | null }[];
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
  | { kind: "appOp"; package: string; op: string; mode: string }
  | { kind: "fileDelete"; path: string }
  | { kind: "fileMove"; from: string; to: string }
  | { kind: "fileCreateFolder"; path: string }
  | { kind: "setup"; step: SetupStep }
  | { kind: "mappings"; change: MappingChange };
export type Manager = "shizukuNext" | "porter" | "existing";
export type ToolId =
  | "shizukuNext"
  | "porter"
  | "porterCompat"
  | "termux"
  | "termuxBoot"
  | "backdrop"
  | "projectivy"
  | "monet"
  | "at4k";
export type LatestLauncher = { tool: ToolId; version: string };
/** What a mapped button's press opened, kept above the pages because they rebuild after a change. */
export type ButtonFollowUp = {
  key: KeyIdentity;
  apps: { package: string; label: string }[];
};
export type SetupStep =
  | { kind: "startServer"; manager: Manager }
  | { kind: "openTerminal" }
  | { kind: "allowBackground"; manager: Manager }
  | { kind: "deployShell"; manager: Manager }
  | { kind: "requestAccess"; manager: Manager }
  | { kind: "allowBootStart"; manager: Manager }
  | { kind: "installMonitor" }
  | { kind: "useHelper" }
  | { kind: "stopUsingHelper" }
  | { kind: "removeMonitor" };
export type KeyIdentity = { source: string; key: string; scan: string | null };
export type Mapping = {
  key: KeyIdentity;
  hold?: boolean;
  component: string;
  send?: string;
  label: string;
};
export type MappingChange =
  | { kind: "apply"; mappings: Mapping[] }
  | { kind: "pause"; paused: boolean }
  | { kind: "removeAll" };
export type MappingState = {
  running: boolean;
  paused: boolean;
  survivedRestart?: boolean;
  mappings: Mapping[];
  log: string[];
};
export type LaunchableApp = { component: string; label: string };
export type ToolStatus = {
  installed: boolean;
  official: boolean;
  version: string | null;
};
export type Stage =
  "terminal" | "pair" | "startup" | "done";
export type ToolkitSnapshot = {
  shizuku: "empty" | "shizukuNext" | "porterCompat" | "other" | "unknown";
  shizukuVersion: string | null;
  porter: ToolStatus;
  termux: ToolStatus;
  termuxBoot: ToolStatus;
  manager: Manager | null;
  helper: { package: string; label: string } | null;
  serverRunning: boolean;
  canConfigureTerminal: boolean;
  terminalReady: boolean;
  shellReady: boolean;
  authorized: boolean;
  monitor: boolean;
  listener: boolean;
  wired: boolean;
  noWirelessDebugging: boolean;
  networkAdb: boolean;
  bootStart: boolean;
  watchdog: boolean;
  agent: boolean;
  agentReady: boolean;
  stage: Stage;
};
export type InstallProgress = { stage: string; sent: number; total: number };
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
  | { kind: "logs"; level: string; package: string | null; lines: number }
  | { kind: "recording"; step: RecordingStep }
  | { kind: "files"; path: string }
  | { kind: "verifyShell" }
  | { kind: "pairAgent"; code: string }
  | { kind: "verifyAgent" }
  | { kind: "captureKeys"; seconds: number }
  | { kind: "mappings" }
  | { kind: "launchableApps" }
  | { kind: "screenshot" };
export type RemoteAction =
  | { kind: "key"; key: string; longPress: boolean }
  | { kind: "text"; text: string }
  | { kind: "launch"; component: string };
export type FileEntry = {
  name: string;
  path: string;
  kind: "folder" | "file" | "other" | "brokenLink" | "denied";
  link: boolean;
  canOpen: boolean;
  readable: boolean;
  size: number | null;
  modified: number | null;
};
export type FileListing = {
  path: string;
  status: "ok" | "denied" | "missing" | "notDirectory";
  writable: boolean;
  entries: FileEntry[];
  total: number;
  truncated: boolean;
};
export type UploadChoice = { id: string; name: string; size: number };
export type Profile = { id: number; name: string; running: boolean };
export type AppMetadata = {
  package: string;
  label: string | null;
  icon: string | null;
  activities?: string[];
  permissions?: { name: string; granted: boolean; protection: number | null }[];
  signers?: string[];
  error?: string | null;
  group?: Group;
  rating?: Rating | null;
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
  rating?: Rating | null;
  installedRelated?: string[];
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
  installTool(
    sessionId: string,
    tool: ToolId,
    onprogress: (progress: InstallProgress) => void,
  ): Promise<string>;
  installAgent(sessionId: string): Promise<void>;
  latestLaunchers(): Promise<LatestLauncher[]>;
  chooseUpload(sessionId: string): Promise<UploadChoice | null>;
  upload(
    sessionId: string,
    uploadId: string,
    folder: string,
    name: string,
    onprogress: (sent: number) => void,
  ): Promise<void>;
  cancelUpload(): Promise<void>;
  /** Each resolves false when the user closes the picker without choosing. */
  saveTvFile(
    sessionId: string,
    remote: string,
    name: string,
    onprogress: (progress: { received: number; total: number }) => void,
  ): Promise<boolean>;
  saveFile(
    name: string,
    content: { text: string } | { base64: string },
  ): Promise<boolean>;
  prepare(
    target: Target,
    action: Action,
    allowUnsafe?: boolean,
  ): Promise<Review>;
  ratingsStatus(): Promise<RatingsStatus>;
  ratingsUpdate(): Promise<boolean>;
  ratingsUpdateIfDue(): Promise<boolean>;
  ratingsRemove(): Promise<void>;
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
    installTool: (sessionId, tool, onprogress) => {
      const onProgress = new Channel<InstallProgress>();
      onProgress.onmessage = onprogress;
      return call("install_tool", { sessionId, tool, onProgress });
    },
    installAgent: (sessionId) => call("install_agent", { sessionId }),
    latestLaunchers: () => call("latest_launchers"),
    chooseUpload: (sessionId) => call("choose_upload", { sessionId }),
    upload: (sessionId, uploadId, folder, name, onprogress) => {
      const onProgress = new Channel<number>();
      onProgress.onmessage = onprogress;
      return call("start_upload", {
        sessionId,
        uploadId,
        folder,
        name,
        onProgress,
      });
    },
    cancelUpload: () => call("cancel_upload", {}),
    saveTvFile: (sessionId, remote, name, onprogress) => {
      const onProgress = new Channel<{ received: number; total: number }>();
      onProgress.onmessage = onprogress;
      return call("save_tv_file", { sessionId, remote, name, onProgress });
    },
    saveFile: (name, content) => call("save_file", { name, content }),
    prepare: (target, action, allowUnsafe) =>
      call("prepare_action", { target, action, allowUnsafe }),
    ratingsStatus: () => call("ratings_status"),
    ratingsUpdate: () => call("ratings_update"),
    ratingsUpdateIfDue: () => call("ratings_update_if_due"),
    ratingsRemove: () => call("ratings_remove"),
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
