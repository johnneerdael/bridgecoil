import { render, cleanup } from "@testing-library/svelte";
import { afterEach, it, expect } from "vitest";
import RatingBadge from "./RatingBadge.svelte";
import { groupLabels, levelInfo, type Level } from "./ratings";
afterEach(cleanup);
const levels: Level[] = ["recommended", "advanced", "expert", "unsafe"];

it("every level has a label, icon and explanation", () => {
  for (const info of Object.values(levelInfo)) {
    expect(info.label).toBeTruthy();
    expect(info.icon).toBeTruthy();
    expect(info.explanation).toBeTruthy();
  }
});

it("no copy calls anything safe", () => {
  const text = [
    ...Object.values(levelInfo).flatMap((i) => [i.label, i.explanation]),
    ...Object.values(groupLabels),
  ].join("\n");
  expect(text).not.toMatch(/\bsafe\b/i);
});

it("not rated says unknown rather than low risk", () => {
  expect(levelInfo.notRated.explanation).toContain("unknown, not low risk");
});

it("shows the icon and name on every badge", () => {
  for (const level of levels) {
    const { container, unmount } = render(RatingBadge, { level });
    expect(container.textContent).toContain(levelInfo[level].label);
    expect(container.querySelector("svg")).toBeTruthy();
    unmount();
  }
});

it("treats a missing level as not rated", () => {
  const { container } = render(RatingBadge, { level: null });
  expect(container.textContent).toContain("Not rated");
});
