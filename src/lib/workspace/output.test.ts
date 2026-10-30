import { it, expect } from "vitest";
import { outputPages } from "./output";
it("bounds newline-heavy and wide output without losing Unicode or whitespace", () => {
  const raw = "🙂漢字\n".repeat(30) + "long ".repeat(80);
  const pages = outputPages(raw, 12, 3);
  expect(pages.join("")).toBe(raw);
  expect(pages.length).toBeGreaterThan(10);
  expect(pages.every((p) => p.split("\n").length <= 4)).toBe(true);
});
