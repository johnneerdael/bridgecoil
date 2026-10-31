// Decides whether main has something to release, and what to call it.
//
// Versions come from tags, so nothing has to commit a bump back to main. Notes come from each
// merged PR's "Release notes" section, or the subject of a feat: or fix: commit pushed directly.
import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const TESTFLIGHT_FOCUS_LIMIT = 1500;

export function nextVersion(base, tags) {
  const [major, minor, patch] = base.split(".").map(Number);
  const released = tags
    .map((tag) => tag.match(new RegExp(`^v${major}\\.${minor}\\.(\\d+)$`)))
    .filter(Boolean)
    .map((match) => Number(match[1]));
  return `${major}.${minor}.${released.length ? Math.max(patch, ...released.map((p) => p + 1)) : patch}`;
}

function section(body, name) {
  const text = (body ?? "").replace(/<!--[\s\S]*?-->/g, "");
  const parts = text.split(/^##[ \t]+(.+?)[ \t]*$/m);
  for (let i = 1; i < parts.length - 1; i += 2) {
    if (parts[i].trim().toLowerCase() === name) return parts[i + 1];
  }
  return "";
}

/** Bullets from a PR's Release notes section, with wrapped lines joined back together. */
export function prNotes(body) {
  const lines = [];
  for (const raw of section(body, "release notes").split("\n")) {
    const text = raw.trim();
    if (!text) continue;
    if (/^[-*]\s/.test(text) || !lines.length) lines.push(text.replace(/^[-*]\s*/, ""));
    else lines[lines.length - 1] += ` ${text}`;
  }
  return lines.filter(Boolean);
}

/** Why a PR's notes can't be published, or nothing when they can. */
export function notesProblem(body) {
  const notes = prNotes(body);
  if (!notes.length) return "Fill in the Release notes section, or write `Internal:` and what changed.";
  if (notes.some((note) => /^internal:\s*$/i.test(note))) return "Say what changed after `Internal:`.";
  if (notes.some((note) => note.length > 300)) return "Keep each release note under 300 characters.";
  return "";
}

const isInternal = (note) => /^internal:/i.test(note);
const pullRequest = (subject) =>
  subject.match(/\(#(\d+)\)$/)?.[1] ?? subject.match(/^Merge pull request #(\d+)/)?.[1];

function commitNote(subject) {
  const match = subject.match(/^(feat|fix)(\([^)]*\))?!?:\s*(.+)$/);
  if (!match) return "";
  return match[3][0].toUpperCase() + match[3].slice(1);
}

/** The user-facing notes for these commit subjects, oldest first. */
export function publicNotes(subjects, bodies) {
  const notes = [];
  for (const subject of subjects) {
    const pr = pullRequest(subject);
    const found = pr ? prNotes(bodies[pr]).filter((n) => !isInternal(n)) : [commitNote(subject)];
    for (const note of found) if (note && !notes.includes(note)) notes.push(note);
  }
  return notes;
}

export function renderChanges(notes, compareUrl) {
  const bullets = notes.map((note) => `- ${note}`).join("\n");
  return `## Changes\n\n${bullets}\n\n**Full changelog**: ${compareUrl}\n`;
}

export function testflightFocus(notes) {
  const kept = [];
  let length = 0;
  for (const note of notes) {
    length += note.length + 3;
    if (length > TESTFLIGHT_FOCUS_LIMIT) break;
    kept.push(`- ${note}`);
  }
  return kept.join("\n");
}

const VERSION_FILES = [
  ["package.json", /("version":\s*")[^"]+(")/],
  ["src-tauri/tauri.conf.json", /("version":\s*")[^"]+(")/],
  ["src-tauri/Cargo.toml", /^(version\s*=\s*")[^"]+(")/m],
];

/** Sets the app version for this build only. It's never committed. */
export function stamp(version, root = ".") {
  for (const [file, pattern] of VERSION_FILES) {
    const path = `${root}/${file}`;
    const before = readFileSync(path, "utf8");
    if (!pattern.test(before)) throw new Error(`No version found in ${file}`);
    writeFileSync(path, before.replace(pattern, `$1${version}$2`));
  }
}

const run = (command, ...args) => execFileSync(command, args, { encoding: "utf8" }).trim();

function plan({ repository, head, output, changesPath }) {
  const base = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
  const tags = run("git", "tag", "--list", "v*").split("\n").filter(Boolean);
  const merged = run("git", "tag", "--list", "v*", "--merged", head, "--sort=-v:refname");
  const previous = merged.split("\n").filter(Boolean)[0];
  const span = previous ? `${previous}..${head}` : head;
  const subjects = run("git", "log", "--first-parent", "--reverse", "--format=%s", span)
    .split("\n")
    .filter(Boolean);
  const bodies = {};
  for (const subject of subjects) {
    const pr = pullRequest(subject);
    if (pr) bodies[pr] = run("gh", "api", `repos/${repository}/pulls/${pr}`, "--jq", ".body");
  }
  const notes = publicNotes(subjects, bodies);
  const version = nextVersion(base, tags);
  const compare = `https://github.com/${repository}/compare/${previous ?? head}...v${version}`;
  writeFileSync(changesPath, renderChanges(notes, compare));
  appendFileSync(
    output,
    `release=${notes.length > 0}\nversion=${version}\nfocus<<NOTES\n${testflightFocus(notes)}\nNOTES\n`,
  );
  console.log(notes.length ? `Releasing ${version}:\n${notes.join("\n")}` : "Nothing user-facing to release.");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [command, argument] = process.argv.slice(2);
  if (command === "stamp") stamp(argument);
  else if (command === "check-pr") {
    const problem = notesProblem(process.env.PR_BODY);
    if (problem) {
      console.error(`::error::${problem}`);
      process.exit(1);
    }
  } else if (command === "plan") {
    plan({
      repository: process.env.GITHUB_REPOSITORY,
      head: process.env.RELEASE_SHA,
      output: process.env.GITHUB_OUTPUT,
      changesPath: argument,
    });
  } else {
    console.error("Usage: next-release.mjs plan <changes.md> | stamp <version> | check-pr");
    process.exit(2);
  }
}
