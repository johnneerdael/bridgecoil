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
      value = {
        items: rows.slice(q.page * q.pageSize, (q.page + 1) * q.pageSize),
        total: rows.length,
        page: q.page,
        pageSize: q.pageSize,
      };
      break;
    }
    case "package":
      value = {
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
        current: "org.bridgecoil.fixture.home/.MainActivity",
      };
      break;
    case "toolkit":
      value = [
        {
          name: "Shizuku",
          package: "moe.shizuku.privileged.api",
          installed: true,
          authorized: "unknown",
          running: "unknown",
          startup: "unknown",
        },
        {
          name: "Termux",
          package: "com.termux",
          installed: true,
          authorized: "unknown",
          running: "unknown",
          startup: "unknown",
        },
        {
          name: "Termux:Boot",
          package: "com.termux.boot",
          installed: false,
          authorized: "unknown",
          running: "unknown",
          startup: "unknown",
        },
      ];
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
    case "files": {
      const entries = [
        `${q.path}/Folder one`,
        `${q.path}/Fixture file.txt`,
        `${q.path}/Unicode — 漢字.txt`,
        `${q.path}/Last file.png`,
      ];
      value = {
        path: q.path,
        entries: {
          items: entries.slice(q.page * q.pageSize, (q.page + 1) * q.pageSize),
          total: entries.length,
          page: q.page,
          pageSize: q.pageSize,
        },
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
