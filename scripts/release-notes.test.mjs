import assert from "node:assert/strict";
import { test } from "node:test";
import { renderReleaseNotes } from "./release-notes.mjs";

const input = {
  repository: "johnneerdael/bridgecoil",
  version: "0.1.7",
  sha: "a".repeat(40),
  changes: "## What's Changed\n* Improve launcher recovery by @author in #7\n\n**Full Changelog**: https://example.com/compare\n",
};

test("release body pins the banner and links the exact artifacts the workflow produces", () => {
  const body = renderReleaseNotes(input);
  assert.ok(body.startsWith(`![Bridgecoil. From connection to control.](https://raw.githubusercontent.com/${input.repository}/${input.sha}/site/assets/marketing/bridgecoil-readme-dark.png)`));
  assert.ok(body.includes("/releases/download/v0.1.7/Bridgecoil-0.1.7-arm64.apk"));
  assert.ok(body.includes("/releases/download/v0.1.7/SHA256SUMS"));
  assert.ok(body.includes("/commit/" + input.sha));
  assert.ok(body.includes("https://johnneerdael.github.io/bridgecoil/guide.html"));
  assert.ok(body.indexOf("## Downloads") < body.indexOf("## What's Changed"));
  assert.ok(body.includes(input.changes.trim()));
  assert.ok(!body.includes("releases/latest/download"));
  assert.ok(!body.includes(".dmg"));
});

test("empty generated notes retain useful installation and validation information", () => {
  const body = renderReleaseNotes({ ...input, changes: "" });
  assert.ok(body.includes("No generated change notes were supplied."));
  assert.ok(body.includes("real Android 9 TV box"));
  assert.ok(body.includes("Desktop packages remain source builds"));
  assert.ok(body.includes("actions/workflows/release.yml"));
  assert.ok(!body.includes("Same version, for invited testers."));
  assert.ok(!body.includes("this release workflow publishes only the Android APK"));
});

test("reject malformed repository, source and version instead of creating misleading links", () => {
  for (const patch of [{ repository: "https://github.com/owner/repo" }, { sha: "main" }, { version: "v0.1.7" }, { version: "../../latest" }]) {
    assert.throws(() => renderReleaseNotes({ ...input, ...patch }));
  }
});
