// Development-only fixture adapter. Never imports native invoke, fetch, sockets or a real ADB client.
import type {
  Platform,
  Target,
  Query,
  Review,
  Action,
  Operation,
  PackageDetail,
  AppMetadata,
  PackageAction,
  BulkReview,
} from "../../src/lib/workspace/client";
import type { Device } from "../../src/lib/model";
import type { Group, Rating } from "../../src/lib/workspace/ratings";
export const device: Device = {
  id: "fixture-session",
  deviceId: "fixture-device",
  profileId: 0,
  serial: "FIXTURE",
  build: "fixture/validation",
  manufacturer: "Fixture vendor",
  model: "Fixture living room TV",
  endpoint: "192.0.2.42:37123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: ["arm64-v8a"],
};
const apps: AppMetadata[] = [
  {
    package: "org.bridgecoil.fixture.player",
    label: "Fixture Player",
    icon: null,
  },
  { package: "org.bridgecoil.fixture.home", label: "Fixture Home", icon: null },
  {
    package: "org.bridgecoil.fixture.tools",
    label: "Fixture Tool with a deliberately long display name",
    icon: null,
  },
  {
    package: "org.bridgecoil.fixture.extra",
    label: "اختبار — Fixture RTL name",
    icon: null,
  },
  {
    package: "org.bridgecoil.fixture.last",
    label: "Last fixture app",
    icon: null,
  },
];
apps.push(
  ...Array.from({ length: 200 }, (_, i) => ({
    package: `org.bridgecoil.fixture.app${i}`,
    label: `Fixture app ${String(i + 1).padStart(3, "0")}`,
    icon: null,
  })),
);
const target = (d: Device): Target => ({
  deviceId: d.deviceId,
  sessionId: d.id,
  profileId: d.profileId,
});
const reviews = new Map<string, Review>(),
  bulks = new Map<string, BulkReview>();
let counter = 0;
const records: Operation[] = Array.from({ length: 9 }, (_, i) => ({
  id: `fixture-op-${i}`,
  target: target(device),
  title: `Fixture operation ${i + 1}`,
  kind: "fixture",
  status: i % 3 === 0 ? "unknown" : i % 3 === 1 ? "verified" : "sent",
  output: "Fixture output only. No TV operation occurred.\n".repeat(14),
  error:
    i % 3 === 0
      ? "Fixture interrupted operation. Inspect before retrying."
      : null,
  recovery: null,
  createdMs: Date.now() - i * 60000,
}));
const fixtureGroups: Group[] = [
  "installed",
  "recommended",
  "advanced",
  "notRated",
  "expert",
  "unsafe",
];
const fixtureRating = (group: Group): Rating | null =>
  group === "installed" || group === "notRated"
    ? null
    : {
        level: group,
        source: "Oem",
        description: "Fixture rating. No list was downloaded.",
        dependencies: ["com.fixture.one"],
        neededBy: [],
      };
function makeReview(t: Target, action: Action): Review {
  const review: Review = {
    id: `fixture-review-${++counter}`,
    target: t,
    title: `Fixture ${action.kind} change`,
    effect:
      "Fixture review: this action is simulated entirely in memory. No device connection or mutation occurs.",
    currentState: "Fixture current state",
    losses:
      action.kind === "package" &&
      ["remove", "clearData"].includes(action.action)
        ? [
            "Fixture data-loss review. App data would be deleted by this kind of real operation.",
          ]
        : [],
    reversible: false,
    commands: [JSON.stringify(action)],
    expiresMs: Date.now() + 600000,
  };
  reviews.set(review.id, review);
  return review;
}
function result(
  t: Target,
  title: string,
  status: Operation["status"] = "verified",
): Operation {
  const row: Operation = {
    id: `fixture-result-${++counter}`,
    target: t,
    title,
    kind: "fixture",
    deviceLabel: "Fixture living room TV",
    status,
    output: "UI fixture result only. No TV operation occurred.",
    error: null,
    recovery: null,
    createdMs: Date.now(),
  };
  records.unshift(row);
  return row;
}
async function query<T>(t: Target, q: Query): Promise<T> {
  let value: unknown;
  switch (q.kind) {
    case "device":
      value = {
        shellIdentity: "Fixture uid=2000(shell)",
        storage: "Fixture /data: 3.8 GB available",
        bootId: "fixture-boot",
        uptimeSeconds: "120",
      };
      break;
    case "profiles":
      value = [
        { id: 0, name: "Owner", running: true },
        { id: 10, name: "Kids fixture", running: false },
      ];
      break;
    case "packages": {
      const rows = apps.filter((a) =>
        `${a.label} ${a.package}`
          .toLowerCase()
          .includes(q.search.toLowerCase()),
      );
      const rated = rows
        .map((app, index) => {
          const group = fixtureGroups[Math.floor((index * 6) / rows.length)];
          return { ...app, group, rating: fixtureRating(group) };
        })
        .filter((app, index) =>
          q.filter === "system"
            ? app.group !== "installed"
            : q.filter === "disabled"
              ? app.group === "recommended" && index % 3 === 0
              : true,
        );
      value = {
        items: rated.slice(q.page * q.pageSize, (q.page + 1) * q.pageSize),
        total: rated.length,
        page: q.page,
        pageSize: q.pageSize,
        groups: fixtureGroups
          .map((id) => ({
            id,
            count: rated.filter((app) => app.group === id).length,
          }))
          .filter((group) => group.count),
      };
      break;
    }
    case "package":
      value = {
        rating: fixtureRating(
          fixtureGroups[
            Math.floor(
              (apps.findIndex((a) => a.package === q.package) * 6) / apps.length,
            )
          ],
        ),
        installedRelated: ["com.fixture.one"],
        package: q.package,
        state: { installed: true, enabled: 0, stopped: false, version: 1 },
        versionName: "1.0",
        activities: [`${q.package}/.MainActivity`],
        permissions: ["android.permission.CAMERA: granted=false"],
        raw: "Fixture PackageManager output\n".repeat(30),
        metadata: {
          ...apps.find((a) => a.package === q.package),
          package: q.package,
          label: apps.find((a) => a.package === q.package)?.label ?? null,
          icon: null,
          permissions: [
            {
              name: "android.permission.CAMERA",
              granted: false,
              protection: 1,
            },
            {
              name: "android.permission.RECORD_AUDIO",
              granted: true,
              protection: 1,
            },
          ],
          signers: ["00".repeat(32)],
        },
      } satisfies PackageDetail;
      break;
    case "appOps":
      value = [
        {
          op: "SYSTEM_ALERT_WINDOW",
          name: "Display over other apps",
          mode: { packageMode: "default", uidMode: null },
          error: null,
        },
        {
          op: "GET_USAGE_STATS",
          name: "Usage access",
          mode: { packageMode: "allow", uidMode: "ignore" },
          error: null,
        },
        {
          op: "WRITE_SETTINGS",
          name: "Modify system settings",
          mode: null,
          error: "Fixture unsupported operation",
        },
      ];
      break;
    case "launchers":
      value = {
        homes: [
          "org.bridgecoil.fixture.home/.MainActivity",
          "org.bridgecoil.fixture.extra/.Home",
        ],
        stock: ["org.bridgecoil.fixture.home/.MainActivity"],
        current: "org.bridgecoil.fixture.home/.MainActivity",
      };
      break;
    case "toolkit":
      value = {
        shizuku: "other",
        shizukuVersion: null,
        porter: { installed: false, official: false, version: null },
        termux: { installed: false, official: false, version: null },
        termuxBoot: { installed: false, official: false, version: null },
        manager: null,
        serverRunning: false,
        canConfigureTerminal: false,
        terminalReady: false,
        shellReady: false,
        authorized: false,
        monitor: false,
        listener: false,
        wired: false,
        noWirelessDebugging: false,
        networkAdb: false,
        bootStart: false,
        watchdog: false,
        stage: "manager",
      };
      break;
    case "diagnostics":
      value = {
        output:
          `Fixture ${q.section} output\n` +
          "12:00:00.000 2000 I Fixture: No device connection.\n".repeat(80),
      };
      break;
    case "network":
      value = {
        output: "Fixture address: 192.0.2.42\nNo real network inspection.",
        forwarding: "Fixture only",
      };
      break;
    case "settings":
      value = [
        {
          namespace: "global",
          key: "window_animation_scale",
          name: "Window animation",
          value: "1",
          error: null,
        },
        {
          namespace: "global",
          key: "transition_animation_scale",
          name: "Transition animation",
          value: "1",
          error: null,
        },
        {
          namespace: "system",
          key: "screen_brightness",
          name: "Brightness",
          value: "128",
          error: null,
        },
      ];
      break;
    case "mappings":
      value = {
        running: true,
        paused: false,
        mappings: [
          {
            key: { source: "Fixture Remote", key: "KEY_UNKNOWN", scan: "000c0057" },
            component: "org.bridgecoil.fixture/.Main",
            label: "Fixture Player",
          },
        ],
        log: [],
      };
      break;
    case "captureKeys":
      value = {
        presses: [{ source: "Fixture Remote", key: "KEY_UNKNOWN", scan: "000c0051" }],
      };
      break;
    case "screenshot":
      value = { png: "iVBORw0KGgo=" };
      break;
    case "launchableApps":
      value = [
        { component: "org.bridgecoil.fixture/.Main", label: "Fixture Player" },
        { component: "org.bridgecoil.other/.Main", label: "Fixture Browser" },
      ];
      break;
    case "files": {
      const row = (name: string, folder: boolean, canOpen = folder) => ({
        name,
        path: `${q.path.replace(/\/$/, "")}/${name}`,
        kind: folder ? "folder" : "file",
        link: false,
        canOpen,
        readable: true,
        size: folder ? 4096 : 2048,
        modified: 1700000000,
      });
      const entries = [
        row("Folder one", true),
        row("Locked folder", true, false),
        row("Fixture file.txt", false),
        row("Unicode — 漢字.txt", false),
        row("Last file.png", false),
      ];
      value = {
        path: q.path,
        status: "ok",
        writable: true,
        entries,
        total: entries.length,
        truncated: false,
      };
      break;
    }
  }
  return value as T;
}
export const api: Platform = {
  native: true,
  query,
  prepare: async (t, a) => makeReview(t, a),
  ratingsStatus: async () => ({
    stored: true,
    commit: "5c179a04daf55afe51e35ed2aeaa60c9b07a03d8",
    committedAt: "2026-10-24T07:42:13Z",
    packages: 5382,
  }),
  saveTvFile: async () => true,
  saveFile: async () => true,
  ratingsUpdate: async () => false,
  ratingsUpdateIfDue: async () => false,
  ratingsRemove: async () => {},
  installTool: async (_session, _tool, onprogress) => {
    onprogress({ stage: "uploading", sent: 1, total: 2 });
    return "v1";
  },
  chooseUpload: async () => ({
    id: "fixture-upload",
    name: "Fixture video.mp4",
    size: 1048576,
  }),
  upload: async (_session, _id, _folder, _name, onprogress) => onprogress(1048576),
  cancelUpload: async () => {},
  apply: async (id) => {
    const r = reviews.get(id)!;
    reviews.delete(id);
    return result(r.target, r.title);
  },
  discard: async (id) => {
    reviews.delete(id);
  },
  remote: async (t, a) => result(t, `Fixture ${a.kind}`, "sent"),
  history: async (page, pageSize) => ({
    items: records.slice(page * pageSize, (page + 1) * pageSize),
    total: records.length,
    page,
    pageSize,
  }),
  recovery: async () => {
    throw new Error("Fixture has no recorded recovery.");
  },
  clearHistory: async () => {
    records.splice(0);
  },
  profile: async (t, id) => ({
    ...device,
    profileId: id,
    id: `fixture-profile-${id}`,
  }),
  prepareBulk: async (t, packages, action: PackageAction) => {
    const r: BulkReview = {
      id: `fixture-bulk-${++counter}`,
      target: t,
      items: packages.map((packageId) => ({
        package: packageId,
        review: makeReview(t, { kind: "package", package: packageId, action }),
      })),
      excluded: [],
      expiresMs: Date.now() + 600000,
    };
    bulks.set(r.id, r);
    return r;
  },
  applyBulk: async (id) => {
    const r = bulks.get(id)!;
    bulks.delete(id);
    return result(r.target, "Fixture bulk result");
  },
  cancelBulk: async () => {},
  discardBulk: async (id) => {
    bulks.delete(id);
  },
};
