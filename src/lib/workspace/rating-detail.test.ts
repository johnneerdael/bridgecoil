import {
  render,
  screen,
  fireEvent,
  cleanup,
  within,
} from "@testing-library/svelte";
import { afterEach, beforeAll, it, expect, vi } from "vitest";
import AppsPage from "./AppsPage.svelte";
import ActionReview from "./ActionReview.svelte";
import BulkReview from "./BulkReview.svelte";
import RatingDetail from "./RatingDetail.svelte";
import { levelInfo, type Level, type Rating } from "./ratings";
import type { Platform, Review } from "./client";
import type { Device } from "../model";
afterEach(cleanup);
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
    this.dispatchEvent(new Event("close"));
  };
});
const device: Device = {
  id: "session",
  deviceId: "device",
  profileId: 0,
  serial: "serial",
  build: "build",
  manufacturer: "TCL",
  model: "Smart TV Pro",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: [],
};
const rating = (level: Level | null, over: Partial<Rating> = {}): Rating => ({
  level,
  source: "Oem",
  description: "Runs the home screen.",
  dependencies: [],
  neededBy: [],
  ...over,
});
const detailProps = (value: Rating | null, related: string[] = []) => ({
  rating: value,
  installedRelated: related,
  listDate: "2026-10-24T07:42:13Z",
});
function platform(detail: Record<string, unknown>) {
  return {
    query: vi.fn(async (_: unknown, q: { kind: string }) =>
      q.kind === "packages"
        ? {
            items: [{ package: "org.demo.app", label: "App", icon: null }],
            total: 1,
            page: 0,
            pageSize: 32,
          }
        : {
            package: "org.demo.app",
            state: { installed: true, enabled: 1, stopped: false, version: 1 },
            versionName: "1",
            activities: [],
            permissions: [],
            raw: "",
            ...detail,
          },
    ),
    prepare: vi.fn(async () => ({ id: "review" })),
    ratingsStatus: vi.fn(async () => ({
      stored: true,
      commit: "a".repeat(40),
      committedAt: "2026-10-24T07:42:13Z",
      packages: 5382,
    })),
    ratingsUpdateIfDue: vi.fn(async () => false),
  };
}
async function openApp(detail: Record<string, unknown>) {
  const api = platform(detail);
  render(AppsPage, {
    device,
    api: api as unknown as Platform,
    onreview: vi.fn(),
    onbulk: vi.fn(),
    oninstall: vi.fn(),
  });
  await fireEvent.click(await screen.findByRole("button", { name: /^App/ }));
  const dialog = await screen.findByRole("dialog", { name: "Application" });
  return { api, dialog };
}
const tab = (dialog: HTMLElement, name: string) =>
  fireEvent.click(within(dialog).getByRole("button", { name }));
async function actionsTab(detail: Record<string, unknown>) {
  const opened = await openApp(detail);
  await tab(opened.dialog, "Actions");
  return opened;
}
const blocked = ["Disable", "Remove from profile", "Clear app data"];
async function allActions(dialog: HTMLElement) {
  const seen: string[] = [];
  for (;;) {
    seen.push(
      ...within(dialog)
        .getAllByRole("button")
        .filter((b) => b.closest(".action-grid"))
        .map((b) => b.textContent ?? ""),
    );
    const next = within(dialog).queryByRole("button", {
      name: "Next",
    }) as HTMLButtonElement | null;
    if (!next || next.disabled) return seen;
    await fireEvent.click(next);
  }
}

it("shows the level, explanation, description and source", () => {
  const { container } = render(
    RatingDetail,
    detailProps(rating("expert", { source: "Google" })),
  );
  const text = container.textContent ?? "";
  expect(text).toContain("Expert");
  expect(text).toContain(levelInfo.expert.explanation);
  expect(text).toContain("Runs the home screen.");
  expect(text).toContain("Google");
  expect(text).toContain("mostly based on phones");
  expect(text).toContain("2026");
});

it("renders list text as text only", () => {
  const description = "<img src=x onerror=alert(1)>";
  const { container } = render(
    RatingDetail,
    detailProps(rating("expert", { description })),
  );
  expect(container.querySelector("img")).toBeNull();
  expect(container.textContent).toContain(description);
});

it("marks dependencies that are on this TV", () => {
  const { container } = render(
    RatingDetail,
    detailProps(
      rating("expert", {
        dependencies: ["com.here", "com.gone"],
        neededBy: ["com.user"],
      }),
      ["com.here"],
    ),
  );
  const text = container.textContent ?? "";
  expect(text).toMatch(/com\.here\s*·\s*On this TV/);
  expect(text).not.toMatch(/com\.gone\s*·\s*On this TV/);
  expect(text).toContain("com.user");
});

it("says none listed for empty dependencies without promising there are none", () => {
  const { container } = render(RatingDetail, detailProps(rating("expert")));
  const text = container.textContent ?? "";
  expect(text).toContain("None listed");
  expect(text).toContain("doesn't prove there are none");
});

it("says not rated for a package UAD doesn't know", () => {
  const { container } = render(RatingDetail, detailProps(null));
  const text = container.textContent ?? "";
  expect(text).toContain("Not rated");
  expect(text).toContain("UAD has no entry for this package");
});

it("leaves the section out without a stored list", async () => {
  const { dialog } = await openApp({});
  expect(dialog.querySelector(".rating-detail")).toBeNull();
  expect(dialog.querySelector(".rating-badge")).toBeNull();
});

it("shows the section on overview when opened", async () => {
  const { dialog } = await openApp({ rating: rating("advanced") });
  expect(dialog.querySelector(".rating-detail")).toBeTruthy();
});

it("shows the heading badge on every tab", async () => {
  const { dialog } = await openApp({ rating: rating("expert") });
  for (const name of ["Overview", "Actions", "Access", "Details"]) {
    await tab(dialog, name);
    const heading = dialog.querySelector(".app-detail-heading") as HTMLElement;
    expect(within(heading).getByText("Expert"), name).toBeTruthy();
  }
});

it("keeps the tabs above a long description", async () => {
  const { dialog } = await openApp({
    rating: rating("expert", { description: "x".repeat(1513) }),
  });
  const tabs = within(dialog).getByRole("button", { name: "Actions" });
  const description = dialog.querySelector(".rating-description") as Element;
  expect(
    tabs.compareDocumentPosition(description) &
      Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
});

it("hides the three actions on unsafe apps behind a warning", async () => {
  const { dialog } = await actionsTab({ rating: rating("unsafe") });
  expect(dialog.textContent).toContain(levelInfo.unsafe.explanation);
  const seen = await allActions(dialog);
  for (const name of blocked) expect(seen).not.toContain(name);
  expect(seen).toContain("Restore existing package");
});

it("reveals them for this view only", async () => {
  const { dialog } = await actionsTab({ rating: rating("unsafe") });
  await fireEvent.click(
    within(dialog).getByRole("button", { name: "Show these actions anyway" }),
  );
  expect(await allActions(dialog)).toEqual(expect.arrayContaining(blocked));
  await fireEvent.click(
    within(dialog).getByRole("button", { name: "Close dialog" }),
  );
  await fireEvent.click(screen.getByRole("button", { name: /^App/ }));
  const reopened = await screen.findByRole("dialog", { name: "Application" });
  await tab(reopened, "Actions");
  const seen = await allActions(reopened);
  for (const name of blocked) expect(seen).not.toContain(name);
});

it("sends the override only for the blocked actions", async () => {
  const { api, dialog } = await actionsTab({ rating: rating("unsafe") });
  await fireEvent.click(
    within(dialog).getByRole("button", { name: "Show these actions anyway" }),
  );
  await fireEvent.click(within(dialog).getByRole("button", { name: "Disable" }));
  expect(api.prepare).toHaveBeenCalledWith(
    expect.anything(),
    { kind: "package", package: "org.demo.app", action: "disable" },
    true,
  );
});

it("sends no override for an allowed action on an unsafe app", async () => {
  const { api, dialog } = await actionsTab({ rating: rating("unsafe") });
  await fireEvent.click(within(dialog).getByRole("button", { name: "Enable" }));
  expect(api.prepare).toHaveBeenCalledWith(expect.anything(), {
    kind: "package",
    package: "org.demo.app",
    action: "enable",
  });
});

it("blocks nothing on expert apps", async () => {
  const { dialog } = await actionsTab({ rating: rating("expert") });
  expect(
    within(dialog).queryByRole("button", { name: "Show these actions anyway" }),
  ).toBeNull();
  expect(await allActions(dialog)).toEqual(expect.arrayContaining(blocked));
});

const review = (value: Level | null): Review => ({
  id: "r",
  target: { deviceId: "d", sessionId: "s", profileId: 0 },
  title: "Disable app",
  effect: "Hide the app.",
  currentState: "Enabled",
  losses: [],
  reversible: true,
  commands: [],
  expiresMs: Date.now() + 10000,
  rating: value,
});

it("shows the rating prominently in reviews of risky apps", () => {
  for (const level of ["expert", "unsafe"] as const) {
    const { container, unmount } = render(ActionReview, {
      review: review(level),
      api: {} as Platform,
      onclose: vi.fn(),
      onresult: vi.fn(),
    });
    expect(container.ownerDocument.body.textContent).toContain(
      levelInfo[level].explanation,
    );
    unmount();
  }
});

it("adds no warning to reviews of recommended or unrated apps", () => {
  for (const level of ["recommended", null] as const) {
    const { unmount } = render(ActionReview, {
      review: review(level),
      api: {} as Platform,
      onclose: vi.fn(),
      onresult: vi.fn(),
    });
    expect(document.body.querySelector(".rating-warning")).toBeNull();
    unmount();
  }
});

it("shows the rating in bulk reviews", () => {
  render(BulkReview, {
    review: {
      id: "b",
      target: review(null).target,
      items: [{ package: "org.demo.app", review: review("expert") }],
      excluded: [],
      expiresMs: Date.now() + 10000,
    },
    api: {} as Platform,
    deviceName: "Smart TV Pro",
    onclose: vi.fn(),
    onresult: vi.fn(),
  });
  expect(document.body.textContent).toContain(levelInfo.expert.explanation);
});
