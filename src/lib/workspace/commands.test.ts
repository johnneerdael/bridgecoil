import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, it, expect, vi } from "vitest";
import CommandsTool from "./CommandsTool.svelte";
import { buildCommand, catalog, defaults, shellQuote } from "./commands";
import type { Platform, Query } from "./client";
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
function platform() {
  return {
    query: vi.fn(async (_: unknown, q: Query) =>
      q.kind === "launchableApps"
        ? [{ component: "org.demo/.Main", label: "Player" }]
        : {},
    ),
    prepare: vi.fn(async () => ({ id: "review" })),
  } as unknown as Platform;
}
const all = catalog.flatMap((group) => group.commands);

it("covers the major areas of a TV", () => {
  expect(catalog.map((g) => g.name)).toEqual(
    expect.arrayContaining([
      "Apps",
      "Display",
      "Input",
      "Network",
      "Power",
      "Settings",
      "System",
      "Logs",
    ]),
  );
  expect(all.length).toBeGreaterThanOrEqual(30);
});

it("gives every command and option a plain explanation", () => {
  for (const command of all) {
    expect(command.title, command.id).toBeTruthy();
    expect(command.about, command.id).toBeTruthy();
    for (const option of command.options ?? []) {
      expect(option.label, command.id).toBeTruthy();
      expect(option.about, command.id).toBeTruthy();
      if (option.kind === "choice") {
        expect(option.choices.length, command.id).toBeGreaterThan(0);
        for (const choice of option.choices)
          expect(choice.about, `${command.id} ${choice.value}`).toBeTruthy();
      }
    }
  }
});

it("has unique ids", () => {
  expect(new Set(all.map((c) => c.id)).size).toBe(all.length);
});

it("builds a complete command from every default", () => {
  for (const command of all) {
    const built = buildCommand(command, {
      ...defaults(command),
      app: "org.demo",
    });
    expect(built, command.id).toBeTruthy();
    expect(built, command.id).not.toMatch(/undefined|\{|\}/);
  }
});

it("marks anything that changes the TV", () => {
  const reboot = all.find((c) => c.id === "reboot")!;
  expect(reboot.effect).not.toBe("read");
  const uptime = all.find((c) => c.id === "uptime")!;
  expect(uptime.effect).toBe("read");
});

it("flags commands that can lose data or lock someone out", () => {
  const danger = all.filter((c) => c.effect === "danger").map((c) => c.id);
  expect(danger).toEqual(expect.arrayContaining(["clear-app", "uninstall-app"]));
  for (const command of all.filter((c) => c.effect === "danger"))
    expect(command.warning, command.id).toBeTruthy();
});

it("treats switching off an app as risky, because it can lock you out", () => {
  const disable = all.find((c) => c.id === "disable-app")!;
  expect(disable.effect).toBe("danger");
  expect(disable.warning).toMatch(/Settings/);
});

it("targets the connected profile rather than assuming the first", () => {
  for (const id of ["disable-app", "enable-app", "clear-app", "uninstall-app"]) {
    const command = all.find((c) => c.id === id)!;
    expect(
      buildCommand(command, { ...defaults(command), app: "org.demo", profile: "10" }),
      id,
    ).toContain("--user 10");
  }
});

it("quotes picked values so they stay one argument", () => {
  expect(shellQuote("a b")).toBe("'a b'");
  expect(shellQuote("it's")).toBe(`'it'"'"'s'`);
});

it("runs nothing without review and shows the command first", async () => {
  const api = platform();
  const onreview = vi.fn();
  render(CommandsTool, { device, api, onreview });
  await fireEvent.click(await screen.findByRole("button", { name: /^System/ }));
  await fireEvent.click(await screen.findByRole("button", { name: /^How long/ }));
  expect(screen.getByText("uptime")).toBeTruthy();
  expect(api.prepare).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole("button", { name: "Run on the TV" }));
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "shell",
    command: "uptime",
  });
});

it("updates the preview as options change and explains the choice", async () => {
  render(CommandsTool, { device, api: platform(), onreview: vi.fn() });
  await fireEvent.click(await screen.findByRole("button", { name: /^Display/ }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /^Change text and icon size/ }),
  );
  const select = screen.getByLabelText("Screen density") as HTMLSelectElement;
  await fireEvent.change(select, { target: { value: "240" } });
  expect(screen.getByText("wm density 240")).toBeTruthy();
  expect(screen.getByText(/Larger text/)).toBeTruthy();
});

it("picks apps from the TV rather than asking for a package name", async () => {
  const api = platform();
  render(CommandsTool, { device, api, onreview: vi.fn() });
  await fireEvent.click(await screen.findByRole("button", { name: /^Apps/ }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /^Force stop/ }),
  );
  const select = (await screen.findByLabelText("App")) as HTMLSelectElement;
  expect(select.tagName).toBe("SELECT");
  await waitFor(() => expect(select.value).toBe("org.demo"));
  expect(screen.getByText("am force-stop org.demo")).toBeTruthy();
});

it("warns before a dangerous command", async () => {
  render(CommandsTool, { device, api: platform(), onreview: vi.fn() });
  await fireEvent.click(await screen.findByRole("button", { name: /^Apps/ }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /^Delete an app's data/ }),
  );
  expect(await screen.findByRole("note")).toBeTruthy();
});

it("uses the device's profile when running", async () => {
  const api = platform();
  render(CommandsTool, {
    device: { ...device, profileId: 10 },
    api,
    onreview: vi.fn(),
  });
  await fireEvent.click(await screen.findByRole("button", { name: /^Apps/ }));
  await fireEvent.click(
    await screen.findByRole("button", { name: /^Delete an app's data/ }),
  );
  await screen.findByText("pm clear --user 10 org.demo");
});

it("keeps a filled-in command to run again in one step", async () => {
  localStorage.clear();
  const api = platform();
  const first = render(CommandsTool, { device, api, onreview: vi.fn() });
  await fireEvent.click(await screen.findByRole("button", { name: /^Display/ }));
  await fireEvent.click(await screen.findByRole("button", { name: /^Change animation speed/ }));
  const select = screen.getAllByRole("combobox")[0] as HTMLSelectElement;
  const chosen = select.options[1].value;
  await fireEvent.change(select, { target: { value: chosen } });
  await fireEvent.click(screen.getByRole("button", { name: "Save for later" }));
  expect(screen.getByRole("button", { name: "Saved" })).toBeTruthy();
  first.unmount();

  render(CommandsTool, { device, api, onreview: vi.fn() });
  await fireEvent.click(await screen.findByRole("button", { name: /^Change animation speed ·/ }));
  expect((screen.getAllByRole("combobox")[0] as HTMLSelectElement).value).toBe(chosen);
  await fireEvent.click(screen.getByRole("button", { name: /^Display/ }));
  await fireEvent.click(screen.getByRole("button", { name: "All commands" }));
  await fireEvent.click(screen.getByRole("button", { name: /^Forget/ }));
  expect(screen.queryByRole("button", { name: /^Change animation speed ·/ })).toBeNull();
  expect(JSON.parse(localStorage.getItem("bridgecoil.savedCommands")!)).toEqual([]);
});

it("drops a saved command the app no longer has", async () => {
  localStorage.setItem(
    "bridgecoil.savedCommands",
    JSON.stringify([{ id: "gone", title: "Gone", values: {} }]),
  );
  render(CommandsTool, { device, api: platform(), onreview: vi.fn() });
  expect(screen.queryByRole("button", { name: /^Gone/ })).toBeNull();
  localStorage.clear();
});

it("won't aim a saved command at a different app", async () => {
  const command = catalog
    .flatMap((group) => group.commands)
    .find((item) => item.options?.some((option) => option.kind === "app"))!;
  localStorage.setItem(
    "bridgecoil.savedCommands",
    JSON.stringify([{ id: command.id, title: "Kept", values: { app: "org.gone" } }]),
  );
  render(CommandsTool, { device, api: platform(), onreview: vi.fn() });
  await fireEvent.click(await screen.findByRole("button", { name: /^Kept/ }));
  expect(await screen.findByRole("alert")).toBeTruthy();
  expect(
    (screen.getByRole("button", { name: "Run on the TV" }) as HTMLButtonElement).disabled,
  ).toBe(true);
  localStorage.clear();
});
