import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import { afterEach, describe, it, expect, vi } from "vitest";
import LauncherTool from "./LauncherTool.svelte";
import ScreenshotTool from "./ScreenshotTool.svelte";
import type { Platform } from "./client";
import type { Device } from "../model";
afterEach(() => {
  cleanup();
  localStorage.clear();
});
const device: Device = {
  id: "session",
  deviceId: "device",
  profileId: 10,
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
const stock = "com.vendor.home/.Main";
const fresh = "org.fresh.launcher/.Home";
function platform(current: string, launch = { status: "sent" }) {
  return {
    query: vi.fn(async () => ({ homes: [stock, fresh], current, stock: [stock] })),
    prepare: vi.fn(async () => ({ id: "review" })),
    remote: vi.fn(async () => launch),
    latestLaunchers: vi.fn(async () => []),
    installTool: vi.fn(async () => "v1"),
  } as unknown as Platform;
}
const open = (api: Platform, onreview = vi.fn(), oninstall = vi.fn()) =>
  render(LauncherTool, { device, api, reload: 0, onreview, oninstall });
const projectivy = "com.spocky.projengmenu/.Main";
const listed = (over: object) => ({
  tool: "projectivy",
  name: "Projectivy",
  package: "com.spocky.projengmenu",
  installed: true,
  version: "4.72",
  official: true,
  ...over,
});
function withCatalog(api: Platform, row: object, latest = "4.73") {
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [stock, projectivy],
    current: stock,
    stock: [stock],
    catalog: [row],
  });
  (api.latestLaunchers as ReturnType<typeof vi.fn>).mockResolvedValue([
    { tool: "projectivy", version: latest },
  ]);
  return api;
}

it("installs a listed launcher and lists it afterwards", async () => {
  const api = withCatalog(
    platform(stock),
    listed({ installed: false, version: null, official: null }),
  );
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Install Projectivy" }),
  );
  await waitFor(() =>
    expect(api.installTool).toHaveBeenCalledWith(
      "session",
      "projectivy",
      expect.any(Function),
    ),
  );
  await waitFor(() => expect(api.query).toHaveBeenCalledTimes(2));
});

it("offers an update when a newer version is out", async () => {
  const api = withCatalog(platform(stock), listed({}));
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Update Projectivy" }),
  );
  expect(screen.getByText("4.72 → 4.73")).toBeTruthy();
  await waitFor(() => expect(api.installTool).toHaveBeenCalled());
});

it("compares versions as numbers", async () => {
  const api = withCatalog(platform(stock), listed({ version: "4.9" }), "4.73");
  open(api);
  expect(await screen.findByRole("button", { name: "Update Projectivy" })).toBeTruthy();
});

it("offers no update when the TV is up to date or ahead", async () => {
  const api = withCatalog(platform(stock), listed({ version: "4.74" }));
  open(api);
  expect(await screen.findByText("4.74")).toBeTruthy();
  await waitFor(() => expect(api.latestLaunchers).toHaveBeenCalled());
  await Promise.resolve();
  expect(screen.queryByRole("button", { name: "Update Projectivy" })).toBeNull();
});

it("leaves a build signed by someone else alone", async () => {
  const api = withCatalog(platform(stock), listed({ official: false }));
  open(api);
  expect(await screen.findByText("Your own build")).toBeTruthy();
  await waitFor(() => expect(api.latestLaunchers).toHaveBeenCalled());
  await Promise.resolve();
  expect(screen.queryByRole("button", { name: "Update Projectivy" })).toBeNull();
  expect(screen.getByRole("button", { name: "Try Projectivy" })).toBeTruthy();
});

it("still lists launchers when GitHub can't be reached", async () => {
  const api = withCatalog(
    platform(stock),
    listed({ installed: false, version: null, official: null }),
  );
  (api.latestLaunchers as ReturnType<typeof vi.fn>).mockRejectedValue(new Error("offline"));
  open(api);
  expect(await screen.findByRole("button", { name: "Install Projectivy" })).toBeTruthy();
  expect(screen.getByText("Not installed")).toBeTruthy();
});

it("takes a launcher from a file", async () => {
  const oninstall = vi.fn();
  open(platform(stock), vi.fn(), oninstall);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Install from a file" }),
  );
  expect(oninstall).toHaveBeenCalled();
});

it("shows a listed launcher's name rather than its package", async () => {
  open(onProjectivy());
  await fireEvent.click(await screen.findByRole("button", { name: "Home opened it" }));
  expect(await screen.findByText("In use: Projectivy")).toBeTruthy();
});

const google = "com.google.tv.launcher/.Home";
function onLauncherWithStockOff(component: string | null) {
  const api = onProjectivy();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [projectivy],
    current: projectivy,
    stock: [],
    catalog: [listed({})],
    restorable: [{ package: "com.google.tv.launcher", component }],
  });
  localStorage.setItem("bridgecoil.home.proven.device", "com.spocky.projengmenu");
  return api;
}

it("restores the TV's own launcher in one reviewed step", async () => {
  const api = onLauncherWithStockOff(google);
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Restore your TV's launcher" }),
  );
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "restoreHome",
    component: google,
  });
});

it("just turns it back on where Android can't name its activity", async () => {
  const api = onLauncherWithStockOff(null);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Restore your TV's launcher" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "package",
      package: "com.google.tv.launcher",
      action: "enable",
    }),
  );
});

it("offers no restore when the TV's launcher is still on", async () => {
  open(withCatalog(platform(stock), listed({})));
  await screen.findByRole("button", { name: "Install from a file" });
  expect(screen.queryByRole("button", { name: /Restore/ })).toBeNull();
});

function onProjectivy() {
  const api = platform(projectivy);
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [projectivy],
    current: projectivy,
    stock: [],
    catalog: [listed({})],
  });
  (api.latestLaunchers as ReturnType<typeof vi.fn>).mockResolvedValue([
    { tool: "projectivy", version: "4.73" },
  ]);
  return api;
}

// A TV already on its own launcher would otherwise never reach the list.
it("asks about the Home button once, then goes straight to the list", async () => {
  open(onProjectivy());
  await fireEvent.click(await screen.findByRole("button", { name: "Home opened it" }));
  cleanup();
  open(onProjectivy());
  expect(await screen.findByRole("button", { name: "Update Projectivy" })).toBeTruthy();
  expect(screen.queryByText(/Press Home/)).toBeNull();
  expect(screen.queryByText(/in charge/)).toBeNull();
});

it("asks again after the launcher changes", async () => {
  localStorage.setItem("bridgecoil.home.proven.device", "org.other.launcher");
  open(onProjectivy());
  expect(await screen.findByText(/Press Home/)).toBeTruthy();
});

it("keeps offering to turn stock off after the Home button was proven", async () => {
  localStorage.setItem("bridgecoil.home.proven.device", "org.fresh.launcher");
  open(platform(fresh));
  expect(
    await screen.findByRole("button", { name: "Turn off com.vendor.home" }),
  ).toBeTruthy();
});

it("shows the current launcher and the alternatives", async () => {
  open(platform(stock));
  expect(await screen.findByText(/org\.fresh\.launcher/)).toBeTruthy();
  expect(screen.getByText(/In use/)).toBeTruthy();
});

it("tries a launcher before anything is changed", async () => {
  const api = platform(stock);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
  );
  await waitFor(() =>
    expect(api.remote).toHaveBeenCalledWith(expect.anything(), {
      kind: "launch",
      component: fresh,
    }),
  );
  expect(api.prepare).not.toHaveBeenCalled();
  expect(await screen.findByText(/Did it open/)).toBeTruthy();
});

it("only offers making it the default after the try worked", async () => {
  const api = platform(stock);
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Yes, it works" }));
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "switchHome",
    component: fresh,
    replacing: ["com.vendor.home"],
  });
});

// A launcher left on can take Home straight back, as Projectivy did on a TCL.
it("turns off the launcher it replaces, in the same step", async () => {
  const api = onProjectivy();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [projectivy, fresh],
    current: projectivy,
    stock: [],
    catalog: [listed({})],
  });
  localStorage.setItem("bridgecoil.home.proven.device", "com.spocky.projengmenu");
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "Yes, it works" }));
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "switchHome",
      component: fresh,
      replacing: ["com.spocky.projengmenu"],
    }),
  );
  await waitFor(() =>
    expect(localStorage.getItem("bridgecoil.home.proven.device")).toBe(
      "org.fresh.launcher",
    ),
  );
});

it("doesn't try to turn off something that isn't a launcher", async () => {
  const api = platform("com.android.tv.settings/.FallbackHome");
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Home opened something else" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Use org.fresh.launcher instead" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "switchHome",
      component: fresh,
      replacing: [],
    }),
  );
});

// A launcher left on could take Home back straight after a success.
it("turns off everything that could keep Home, not only the one in use", async () => {
  const api = onProjectivy();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [fresh, projectivy, "org.third/.Home"],
    current: fresh,
    stock: [],
    catalog: [listed({})],
    overriding: ["com.spocky.projengmenu", "org.third"],
  });
  open(api);
  await fireEvent.click(await screen.findByRole("button", { name: "Turn off Projectivy" }));
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "switchHome",
      component: fresh,
      replacing: ["com.spocky.projengmenu", "org.third"],
    }),
  );
});

it("says when another launcher is still taking over Home", async () => {
  const api = onProjectivy();
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [fresh, projectivy],
    current: fresh,
    stock: [],
    catalog: [listed({})],
    overriding: ["com.spocky.projengmenu"],
  });
  const onreview = vi.fn();
  open(api, onreview);
  expect(await screen.findByText("Projectivy is still taking over Home")).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Turn off Projectivy" }));
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "switchHome",
      component: fresh,
      replacing: ["com.spocky.projengmenu"],
    }),
  );
});

it("changes nothing when the try failed", async () => {
  const api = platform(stock);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
  );
  await fireEvent.click(await screen.findByRole("button", { name: "No" }));
  expect(api.prepare).not.toHaveBeenCalled();
  expect(await screen.findByText(/org\.fresh\.launcher/)).toBeTruthy();
});

it("keeps stock switched on until the Home button is proven", async () => {
  const api = platform(fresh);
  open(api);
  await screen.findByText(/Press Home/);
  expect(screen.queryByRole("button", { name: /Turn off/ })).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Home opened it" }));
  expect(
    await screen.findByRole("button", { name: "Turn off com.vendor.home" }),
  ).toBeTruthy();
});

it("turns stock off for this profile through review, with a way back", async () => {
  const api = platform(fresh);
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Home opened it" }),
  );
  expect(screen.getByText(/History/)).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Turn off com.vendor.home" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "package",
      package: "com.vendor.home",
      action: "disable",
    }),
  );
});

it("offers going back when Home didn't open the new launcher", async () => {
  const api = platform(fresh);
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Home opened something else" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Use com.vendor.home instead" }),
  );
  await waitFor(() =>
    expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
      kind: "switchHome",
      component: stock,
      replacing: ["org.fresh.launcher"],
    }),
  );
});

it("names the launcher in use the way the rest of the page does", async () => {
  open(platform(stock));
  expect(await screen.findByText("In use: com.vendor.home")).toBeTruthy();
});

describe("when the TV's own launcher comes first", () => {
  const outranked = () => {
    const api = platform(stock);
    (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
      homes: [stock, fresh],
      current: stock,
      stock: [stock],
      chosen: "org.fresh.launcher",
    });
    return api;
  };

  it("explains it and offers turning stock off through review", async () => {
    const api = outranked();
    const onreview = vi.fn();
    open(api, onreview);
    expect(await screen.findByText(/comes first/)).toBeTruthy();
    await fireEvent.click(
      screen.getByRole("button", { name: "Turn off com.vendor.home" }),
    );
    await waitFor(() =>
      expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
        kind: "switchHome",
        component: fresh,
        replacing: ["com.vendor.home"],
      }),
    );
    expect(onreview).toHaveBeenCalled();
  });

  it("lets someone keep the launcher they have", async () => {
    const api = outranked();
    open(api);
    await fireEvent.click(
      await screen.findByRole("button", { name: "Keep com.vendor.home" }),
    );
    await waitFor(() =>
      expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
        kind: "defaultHome",
        component: stock,
      }),
    );
  });

  it("remembers the choice on a TV that can't report it", async () => {
    const api = platform(stock);
    (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
      homes: [stock, fresh],
      current: stock,
      stock: [stock],
      chosen: null,
      roles: false,
    });
    const first = open(api);
    await fireEvent.click(
      await screen.findByRole("button", { name: "Try org.fresh.launcher" }),
    );
    await fireEvent.click(
      await screen.findByRole("button", { name: "Yes, it works" }),
    );
    await waitFor(() => expect(api.prepare).toHaveBeenCalled());
    first.unmount();
    const again = open(api);
    expect(await screen.findByText(/comes first/)).toBeTruthy();
    await fireEvent.click(
      screen.getByRole("button", { name: "Keep com.vendor.home" }),
    );
    await waitFor(() => expect(api.prepare).toHaveBeenCalledTimes(2));
    again.unmount();
    open(api);
    await screen.findByRole("button", { name: "Try org.fresh.launcher" });
    expect(screen.queryByText(/comes first/)).toBeNull();
  });

  it("stays quiet when nothing else was chosen", async () => {
    const api = platform(stock);
    (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
      homes: [stock, fresh],
      current: stock,
      stock: [stock],
      chosen: "com.vendor.home",
    });
    open(api);
    await screen.findByRole("button", { name: "Try org.fresh.launcher" });
    expect(screen.queryByText(/comes first/)).toBeNull();
  });
});

it("takes a screenshot on request and shows it", async () => {
  const api = {
    query: vi.fn(async () => ({ png: "iVBORw0KGgo=" })),
  } as unknown as Platform;
  render(ScreenshotTool, { device, api });
  await fireEvent.click(
    screen.getByRole("button", { name: "Take a screenshot" }),
  );
  const image = (await screen.findByRole("img")) as HTMLImageElement;
  expect(image.src).toContain("iVBORw0KGgo=");
});

it("keeps a way back after the new launcher is proven", async () => {
  const api = platform(fresh);
  const onreview = vi.fn();
  open(api, onreview);
  await fireEvent.click(await screen.findByRole("button", { name: "Home opened it" }));
  await fireEvent.click(
    await screen.findByRole("button", { name: "Use com.vendor.home instead" }),
  );
  await waitFor(() => expect(onreview).toHaveBeenCalled());
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "switchHome",
    component: stock,
    replacing: ["org.fresh.launcher"],
  });
});

it("says where to go when stock is off and nothing else is left", async () => {
  const api = platform(fresh);
  (api.query as ReturnType<typeof vi.fn>).mockResolvedValue({
    homes: [fresh],
    current: fresh,
    stock: [],
  });
  open(api);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Home opened something else" }),
  );
  expect(await screen.findByText(/undo turning off the launcher/)).toBeTruthy();
});
