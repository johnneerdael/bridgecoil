import { readFileSync } from "node:fs";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";

export function renderReleaseNotes({ repository, version, sha, changes = "" }) {
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) throw new Error("Expected owner/repository");
  if (!/^\d+\.\d+\.\d+(?:-[A-Za-z0-9]+(?:[.-][A-Za-z0-9]+)*)?$/.test(version)) throw new Error("Expected a version without the v prefix");
  if (!/^[a-f0-9]{40}$/i.test(sha)) throw new Error("Expected a full source commit SHA");
  const repo = `https://github.com/${repository}`;
  const download = `${repo}/releases/download/v${version}`;
  const [owner, name] = repository.split("/");
  const site = `https://${owner}.github.io/${name}/`;
  return `![Bridgecoil. From connection to control.](https://raw.githubusercontent.com/${repository}/${sha}/site/assets/marketing/bridgecoil-readme-dark.png)

**The visual ADB workspace.** Connect to your Android TV, manage apps and put your remote to work.

[Website](${site}) · [Get started](${site}guide.html) · [Report an issue](${repo}/issues)

## Downloads

| Download | Install on | Notes |
| --- | --- | --- |
| [Bridgecoil ${version} — Android APK](${download}/Bridgecoil-${version}-arm64.apk) | Android phone, arm64 | Signed with the Bridgecoil release key. |
| [SHA256SUMS](${download}/SHA256SUMS) | Any computer | SHA-256 checksums for the attached APK. |
| TestFlight beta | iPhone and iPad | Same version, for invited testers. |

Install the APK on your phone, enable Wireless debugging on the TV and pair them on the same local network. Keep Wi-Fi enabled on the TV. iOS and desktop are source-build targets; this release workflow publishes only the Android APK.

${changes.trim() || "## Changes\n\nNo generated change notes were supplied."}

## Before you start

Guided TV setup has been exercised on Android TV 16 emulators with secure ADB and no root, and on a real Android 9 TV box. Firmware, key capture and restart behavior vary between TVs. Review the [compatibility notes](${site}guide.html#limits).

<details>
<summary>Build and source verification</summary>

- Source: [\`${sha.slice(0, 12)}\`](${repo}/commit/${sha}).
- The release workflow runs app checks and verifies the APK signing certificate before publishing.
- Download \`SHA256SUMS\` beside the APK and run \`sha256sum -c SHA256SUMS\` (Linux) or \`shasum -a 256 -c SHA256SUMS\` (macOS).
- Checksums verify downloaded bytes; they do not independently establish the publisher’s identity. The signing certificate is documented in [Releasing Bridgecoil](${repo}/blob/${sha}/docs/release.md#signing).

</details>
`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const { values } = parseArgs({ options: {
    repository: { type: "string" }, version: { type: "string" },
    sha: { type: "string" }, changes: { type: "string" },
  } });
  process.stdout.write(renderReleaseNotes({
    ...values, changes: values.changes ? readFileSync(values.changes, "utf8") : "",
  }));
}
