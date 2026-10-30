import { it, expect } from "vitest";
import { parseTheme, resolveTheme, applyTheme } from "./theme";

it("follows the OS unless an explicit theme is stored", () => {
  expect(resolveTheme(parseTheme(null), true)).toBe("dark");
  expect(resolveTheme(parseTheme("bad-value"), false)).toBe("light");
  expect(resolveTheme(parseTheme("light"), true)).toBe("light");
  expect(resolveTheme(parseTheme("dark"), false)).toBe("dark");
});

it("changes root tokens and browser decoration together", () => {
  document.head.innerHTML = '<meta name="theme-color" content="">';
  applyTheme("system", true);
  expect(document.documentElement.dataset.theme).toBe("dark");
  expect(
    document.querySelector('meta[name="theme-color"]')?.getAttribute("content"),
  ).toBe("#101318");
  applyTheme("light", true);
  expect(document.documentElement.dataset.theme).toBe("light");
});
