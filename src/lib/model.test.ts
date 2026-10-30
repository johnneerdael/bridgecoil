import { describe, it, expect } from "vitest";
import { canInstall, errorMessage, type Device, type Selection } from "./model";
const device = {
  id: "a",
  model: "TV",
  endpoint: "tv:123",
  encrypted: true,
  androidVersion: "14",
  sdk: 34,
  abis: ["arm64-v8a"],
} satisfies Device;
const apk = {
  id: "f",
  sessionId: "a",
  name: "test.apk",
  package: "org.example.test",
  versionCode: 1,
  bytes: 42,
  sha256: "hash",
  abis: [],
} satisfies Selection;
describe("installation target", () => {
  it("requires a live matching session and selection", () => {
    expect(canInstall(device, apk, false)).toBe(true);
    expect(canInstall(null, apk, false)).toBe(false);
    expect(canInstall(device, null, false)).toBe(false);
    expect(canInstall({ ...device, id: "b" }, apk, false)).toBe(false);
  });
  it("blocks duplicate submit while busy", () =>
    expect(canInstall(device, apk, true)).toBe(false));
});
it("preserves structured native errors without rendering objects", () => {
  expect(
    errorMessage({ kind: "unknownInstall", message: "Reconnect to verify." }),
  ).toBe("Reconnect to verify.");
  expect(errorMessage("Connection refused")).toBe("Connection refused");
});
