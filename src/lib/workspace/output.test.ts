import { it, expect } from "vitest";
import { outputPages, resultText } from "./output";
it("bounds newline-heavy and wide output without losing Unicode or whitespace", () => {
  const raw = "🙂漢字\n".repeat(30) + "long ".repeat(80);
  const pages = outputPages(raw, 12, 3);
  expect(pages.join("")).toBe(raw);
  expect(pages.length).toBeGreaterThan(10);
  expect(pages.every((p) => p.split("\n").length <= 4)).toBe(true);
});

it("turns a bulk result into plain lines", () => {
  const output = JSON.stringify({
    excluded: [],
    items: [
      { package: "org.one", status: "verified", message: null, operationId: "a" },
      { package: "org.two", status: "failed", message: "It refused", operationId: "b" },
    ],
  });
  expect(resultText("bulkPackages", output)).toBe(
    "Done: org.one\nFailed: org.two. It refused",
  );
});

it("says which apps a bulk change left out", () => {
  const output = JSON.stringify({
    items: [],
    excluded: [{ package: "org.home", reason: "It's the launcher in use." }],
  });
  expect(resultText("bulkPackages", output)).toBe(
    "Left out: org.home. It's the launcher in use.",
  );
});

it("leaves other output and anything unreadable alone", () => {
  expect(resultText("shell", "{}")).toBe("{}");
  expect(resultText("bulkPackages", "not json")).toBe("not json");
});
