import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import ButtonsTool from "./ButtonsTool.svelte";
import type {
  ButtonFollowUp,
  KeyIdentity,
  Mapping,
  MappingState,
  Platform,
  Query,
} from "./client";
import type { Device } from "../model";
afterEach(cleanup);
const device: Device = {
  id: "session",
  deviceId: "device",
  profileId: 0,
  serial: "serial",
  build: "build",
  manufacturer: "Vendor",
  model: "TV",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: [],
};
const remote: KeyIdentity = {
  source: "Vendor Remote",
  key: "KEY_UNKNOWN",
  scan: "000c0057",
};
const saved: Mapping = { key: remote, component: "org.demo/.Main", label: "Player" };
const state = (over: Partial<MappingState> = {}): MappingState => ({
  running: false,
  paused: false,
  mappings: [],
  log: [],
  ...over,
});
function platform(
  current: MappingState,
  presses: KeyIdentity[] = [remote],
  opened: { package: string; label: string }[] = [],
) {
  return {
    query: vi.fn(async (_: unknown, q: Query) => {
      if (q.kind === "mappings") return current;
      if (q.kind === "captureKeys") return { presses, opened };
      if (q.kind === "launchableApps")
        return [
          { component: "org.demo/.Main", label: "Player" },
          { component: "org.other/.Main", label: "Browser" },
        ];
      return {
        manager: "porter",
        stage: "done",
        monitor: true,
        listener: current.running,
        wired: false,
        noWirelessDebugging: false,
        networkAdb: false,
        helper: null,
        bootStart: false,
        watchdog: false,
        agent: true,
        agentReady: true,
      };
    }),
    prepare: vi.fn(async () => ({ id: "review" })),
  } as unknown as Platform;
}
const open = (api: Platform, onreview = vi.fn(), onsetup = vi.fn()) =>
  render(ButtonsTool, { device, api, reload: 0, onreview, onsetup });

it("starts by inviting a first mapping", async () => {
  open(platform(state()));
  expect(await screen.findByText(/No buttons remapped yet/)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Remap a button" })).toBeTruthy();
});

it("learns a button and reviews saving it to the chosen app", async () => {
  const api = platform(state());
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remap a button" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Listen" }));
  expect(api.query).toHaveBeenCalledWith(expect.anything(), {
    kind: "captureKeys",
    seconds: 8,
  });
  await fireEvent.click(
    await screen.findByRole("button", { name: /KEY_UNKNOWN/ }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Browser" }));
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "mappings",
    change: {
      kind: "apply",
      mappings: [
        { key: remote, hold: false, component: "org.other/.Main", label: "Browser" },
      ],
    },
  });
});

const mappedTv = state({
  running: true,
  mappings: [
    { key: remote, hold: false, component: "org.demo/.Main", label: "Player" },
  ],
});
/** Maps the button, then rebuilds the screen with the saved mapping, as the app does. */
async function mapToPlayer(apps: { package: string; label: string }[]) {
  const onreview = vi.fn();
  let kept: ButtonFollowUp | null = null;
  const first = render(ButtonsTool, {
    device,
    api: platform(state(), [remote], apps),
    reload: 0,
    onreview,
    onsetup: vi.fn(),
    onfollowup: (next: ButtonFollowUp | null) => (kept = next),
  });
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remap a button" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Listen" }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /KEY_UNKNOWN/ }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Player" }));
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  first.unmount();
  return { followUp: kept as ButtonFollowUp | null };
}
function reopen(current: MappingState, followUp: ButtonFollowUp | null) {
  const api = platform(current);
  const onreview = vi.fn();
  const onfollowup = vi.fn();
  render(ButtonsTool, {
    device,
    api,
    reload: 1,
    onreview,
    onsetup: vi.fn(),
    followUp,
    onfollowup,
  });
  return { api, onreview, onfollowup };
}
const stock = { package: "org.stock", label: "Stock app" };

it("offers to turn off what the button opened, after the screen rebuilds", async () => {
  const { followUp } = await mapToPlayer([stock]);
  const { api, onreview, onfollowup } = reopen(mappedTv, followUp);
  expect(
    await screen.findByText("This button still opens Stock app"),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Turn off Stock app" }),
  );
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "package",
    package: "org.stock",
    action: "disable",
  });
  expect(onfollowup).toHaveBeenCalledWith(null);
});

it("lets the viewer keep the app the button opened", async () => {
  const { followUp } = await mapToPlayer([stock]);
  const { api, onfollowup } = reopen(mappedTv, followUp);
  await fireEvent.click(await screen.findByRole("button", { name: "Keep it" }));
  expect(onfollowup).toHaveBeenCalledWith(null);
  expect(api.prepare).not.toHaveBeenCalled();
});

it("says so when a press opened no app", async () => {
  const { followUp } = await mapToPlayer([]);
  reopen(mappedTv, followUp);
  expect(await screen.findByText(/didn't open an app/)).toBeTruthy();
});

it("offers nothing when the review was cancelled", async () => {
  const { followUp } = await mapToPlayer([stock]);
  reopen(state(), followUp);
  await screen.findByRole("button", { name: "Remap a button" });
  expect(screen.queryByText("This button still opens Stock app")).toBeNull();
});

it("maps holding a button without touching its tap mapping", async () => {
  const api = platform(state({ running: true, mappings: [saved] }));
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remap a button" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Listen" }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /KEY_UNKNOWN/ }),
  );
  await fireEvent.change(await screen.findByLabelText("When you"), {
    target: { value: "hold" },
  });
  await fireEvent.click(await screen.findByRole("button", { name: "Browser" }));
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "mappings",
      change: {
        kind: "apply",
        mappings: [
          saved,
          { key: remote, hold: true, component: "org.other/.Main", label: "Browser" },
        ],
      },
    }),
  );
});

it("can make a button act as a different one", async () => {
  const api = platform(state());
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remap a button" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Listen" }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /KEY_UNKNOWN/ }),
  );
  await fireEvent.change(await screen.findByLabelText("It should"), {
    target: { value: "button" },
  });
  await fireEvent.click(await screen.findByRole("button", { name: "Mute" }));
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "mappings",
      change: {
        kind: "apply",
        mappings: [
          { key: remote, hold: false, component: "", send: "VOLUME_MUTE", label: "Mute" },
        ],
      },
    }),
  );
});

it("warns that listening can trigger the button's normal action", async () => {
  open(platform(state()));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remap a button" }),
  );
  expect(await screen.findByText(/normal action/)).toBeTruthy();
});

it("explains silence instead of failing", async () => {
  open(platform(state(), []));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remap a button" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Listen" }));
  expect(await screen.findByText(/Nothing reached/)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Listen" })).toBeTruthy();
});

it("adds to existing mappings and replaces a button's old target", async () => {
  const api = platform(state({ running: true, mappings: [saved] }));
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remap a button" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Listen" }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /KEY_UNKNOWN/ }),
  );
  expect(await screen.findByText(/Right now this opens\s+Player/)).toBeTruthy();
  await fireEvent.click(await screen.findByRole("button", { name: "Browser" }));
  await waitFor(() => expect(api.prepare).toHaveBeenCalled());
  const [, action] = (api.prepare as ReturnType<typeof vi.fn>).mock.calls[0];
  expect(action.change.mappings).toHaveLength(1);
  expect(action.change.mappings[0].label).toBe("Browser");
});

it("lists mappings with what the TV reports and their state", async () => {
  open(platform(state({ running: true, mappings: [saved] })));
  expect(await screen.findByText("Player")).toBeTruthy();
  expect(screen.getByText(/Vendor Remote/)).toBeTruthy();
  expect(screen.getByText(/Listening for buttons/)).toBeTruthy();
});

it("flags a listener that has stopped", async () => {
  open(platform(state({ mappings: [saved] })));
  expect(await screen.findByText(/isn't running/)).toBeTruthy();
});

it("pauses, resumes and removes through review", async () => {
  const api = platform(state({ running: true, mappings: [saved] }));
  open(api);
  await fireEvent.click(await screen.findByRole("button", { name: "Pause" }));
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "mappings",
      change: { kind: "pause", paused: true },
    }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Remove all" }));
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "mappings",
      change: { kind: "removeAll" },
    }),
  );
});

it("removing the only mapping removes everything", async () => {
  const api = platform(state({ running: true, mappings: [saved] }));
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remove Player" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "mappings",
      change: { kind: "removeAll" },
    }),
  );
});

it("removing one of several keeps the others", async () => {
  const other: Mapping = {
    key: { ...remote, scan: "000c0051" },
    component: "org.other/.Main",
    label: "Browser",
  };
  const api = platform(state({ running: true, mappings: [saved, other] }));
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Remove Player" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "mappings",
      change: { kind: "apply", mappings: [other] },
    }),
  );
});

it("only counts a press made after the test began", async () => {
  const current = state({
    running: true,
    mappings: [saved],
    log: ["1793431510 started", "1793431512 1:KEY_UNKNOWN org.demo/.Main"],
  });
  open(platform(current));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Test a button" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "I pressed it" }),
  );
  expect(await screen.findByText(/didn't see a press/)).toBeTruthy();
  current.log = [...current.log, "1793431600 1:KEY_UNKNOWN org.demo/.Main"];
  await fireEvent.click(
    await screen.findByRole("button", { name: "I pressed it" }),
  );
  expect(await screen.findByText(/It worked: Player opened/)).toBeTruthy();
});

it("says so when a test press never arrived", async () => {
  const api = platform(state({ running: true, mappings: [saved], log: [] }));
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Test a button" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "I pressed it" }),
  );
  expect(await screen.findByText(/didn't see a press/)).toBeTruthy();
});

it("doesn't call a failed launch a success", async () => {
  const current = state({ running: true, mappings: [saved], log: ["1 started"] });
  open(platform(current));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Test a button" }),
  );
  current.log = [...current.log, "2 failed org.demo/.Main"];
  await fireEvent.click(
    await screen.findByRole("button", { name: "I pressed it" }),
  );
  expect(await screen.findByText(/couldn't open Player/)).toBeTruthy();
});

