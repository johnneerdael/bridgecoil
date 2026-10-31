import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import {
  nextVersion,
  notesProblem,
  prNotes,
  publicNotes,
  renderChanges,
  stamp,
  testflightFocus,
} from "./next-release.mjs";

test("the next version follows the newest tag of the app's major and minor", () => {
  assert.equal(nextVersion("0.1.7", ["v0.1.5", "v0.1.7"]), "0.1.8");
  assert.equal(nextVersion("0.1.7", ["v0.1.10", "v0.1.9"]), "0.1.11");
  assert.equal(nextVersion("0.2.0", ["v0.1.7"]), "0.2.0");
  assert.equal(nextVersion("0.1.20", ["v0.1.7"]), "0.1.20");
});

const template = `## Summary

Something

<!-- ## Release notes in a comment doesn't count -->
## Release notes

<!-- Public, one bullet per change. -->

- Fix pairing on older TVs,
  even when the code expires
* Show the app's name in History

## Validation

- [x] tests
`;

test("a PR's notes are its bullets, with wrapped lines joined", () => {
  assert.deepEqual(prNotes(template), [
    "Fix pairing on older TVs, even when the code expires",
    "Show the app's name in History",
  ]);
});

test("a PR can't merge with notes that can't be published", () => {
  assert.equal(notesProblem(template), "");
  assert.equal(notesProblem("## Release notes\n\n- Internal: tidy the CI cache"), "");
  assert.match(notesProblem("## Summary\n\nNo notes"), /Fill in/);
  assert.match(notesProblem("## Release notes\n\n<!-- todo -->\n\n-\n"), /Fill in/);
  assert.match(notesProblem("## Release notes\n\n- Internal:"), /after `Internal:`/);
  assert.match(notesProblem(`## Release notes\n\n- ${"x".repeat(301)}`), /under 300/);
});

test("notes come from PRs and from feat and fix commits, never internal changes", () => {
  const notes = publicNotes(
    [
      "fix: hear remote buttons on Android 10 and older",
      "docs: record the run",
      "ci: cache Rust",
      "test: longer timeout",
      "feat(ios): add discovery",
      "Better pairing (#12)",
      "Merge pull request #13 from fork/branch",
      "Tidy CI (#14)",
      "fix: hear remote buttons on Android 10 and older",
    ],
    {
      12: "## Release notes\n\n- Pair in one step\n- Internal: rename a module",
      13: "## Release notes\n\n- Show more apps",
      14: "## Release notes\n\n- Internal: tidy CI",
    },
  );
  assert.deepEqual(notes, [
    "Hear remote buttons on Android 10 and older",
    "Add discovery",
    "Pair in one step",
    "Show more apps",
  ]);
});

test("internal changes alone release nothing", () => {
  assert.deepEqual(
    publicNotes(
      ["docs: a", "ci: b", "chore: c", "fix(ci): d", "fix(build): e", "fix(release): f", "fix(test): g"],
      {},
    ),
    [],
  );
});

test("changes render as one bullet per note with a changelog link", () => {
  assert.equal(
    renderChanges(["One", "Two"], "https://example/compare"),
    "## Changes\n\n- One\n- Two\n\n**Full changelog**: https://example/compare\n",
  );
});

test("TestFlight notes stay within Apple's limit without cutting a note in half", () => {
  const focus = testflightFocus(Array.from({ length: 40 }, (_, i) => `${i} ${"x".repeat(90)}`));
  assert.ok(focus.length <= 1500);
  assert.ok(focus.split("\n").every((line) => line.endsWith("x")));
});

test("stamping sets the version everywhere the build reads it", () => {
  const root = mkdtempSync(join(tmpdir(), "bridgecoil-stamp-"));
  mkdirSync(join(root, "src-tauri"));
  const files = {
    "package.json": '{\n  "name": "a",\n  "version": "0.1.7",\n  "dependencies": {"b": "1.0.0"}\n}\n',
    "src-tauri/tauri.conf.json": '{\n  "version": "0.1.7"\n}\n',
    "src-tauri/Cargo.toml": '[package]\nname = "a"\nversion = "0.1.7"\n\n[dependencies]\nb = { version = "1.0.0" }\n',
  };
  for (const [name, text] of Object.entries(files)) writeFileSync(join(root, name), text);
  stamp("0.1.8", root);
  assert.match(readFileSync(join(root, "package.json"), "utf8"), /"version": "0.1.8"/);
  assert.match(readFileSync(join(root, "package.json"), "utf8"), /"b": "1.0.0"/);
  assert.match(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"), /"0.1.8"/);
  const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
  assert.match(cargo, /^version = "0.1.8"$/m);
  assert.match(cargo, /b = \{ version = "1.0.0" \}/);
});
