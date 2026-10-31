# App ratings and list ordering

Adds community removal ratings to the Apps screen, using the Universal Android Debloater Next Generation (UAD-NG) list that Canta also uses. This extends `2026-10-30-adb-workspace-design.md`.

## Why

When tidying a TV, the user can see package names but not what they do or what removing them risks. They want the list itself to show this at a glance, and opening an app to show everything we know about it.

## Goals

- Colour-code the Apps list with Canta's scheme.
- Group the list so the best removal candidates are first and the riskiest are last.
- Show all known information about a package when it is opened.
- Keep the app fully usable offline and without the list.

## Non-goals

- Any rating of our own, or TV-specific corrections to UAD's data.
- Presets or one-tap debloat sets.
- Bundling the list in the app.
- Removal history or undo beyond what the Apps actions already do.

## Anti-goals

This has failed, even if it works technically, if:

- The UI implies an app is safe. A rating is community advice, and the main spec forbids blanket "safe to remove" claims. Never use the word "safe" as a label.
- A missing rating looks reassuring.
- A bad download, or being offline, breaks the Apps screen or loses a working list.
- Text from the list can run as code or commands.

## Decided by the user

- Use Canta's colour guidance.
- Group order, with "Installed by you" first, as listed under List order.

## Data

Source: `resources/assets/uad_lists.json` in `Universal-Debloater-Alliance/universal-android-debloater-next-generation`. It is a JSON object of package name to entry, about 1.6 MB and 5,382 packages on 2026-10-31.

| Field | Use |
|---|---|
| `removal` | `Recommended`, `Advanced`, `Expert` or `Unsafe`, matched ignoring case. Anything else, including a missing value, counts as Not rated |
| `description` | Plain text, may contain newlines and URLs |
| `list` | `Oem`, `Aosp`, `Google`, `Carrier` or `Misc`, shown as the source |
| `dependencies`, `neededBy` | Package names. Sparse: 43 and 27 entries respectively have any |

Ignore unknown fields. Tolerate missing or wrongly typed optional fields per entry. Skip an entry that is not an object.

Known limits to respect in the wording: each package has one rating with no device or Android version, and the list is phone-focused.

## Download and consent

The main spec says no internet access is required, so ratings are opt-in. These are the designer's calls:

- With no list stored, the Apps screen works exactly as it does today, with one dismissible card: "Get community ratings", naming the source and that it downloads from GitHub.
- The download starts only from that card or the Ratings sheet. If dismissed, the card stays away and the Ratings sheet remains the way back. The sheet is reached from the summary line on the Apps screen.
- After the first download, check for a newer version at most once a day when Apps opens. A check failure is silent and keeps the stored list.
- The Ratings sheet shows the source with a link, the commit, its date, and "Check for update" and "Remove ratings". Removing returns the app to the no-list state.

Fetch sequence, in Rust:

1. `GET https://api.github.com/repos/Universal-Debloater-Alliance/universal-android-debloater-next-generation/commits?path=resources%2Fassets%2Fuad_lists.json&per_page=1` for the latest commit's hash and date.
2. If the hash is new, `GET https://raw.githubusercontent.com/.../{hash}/resources/assets/uad_lists.json`, so every rating traces to one commit.
3. Reject bodies over 8 MiB. Parse fully and run the acceptance rule below before replacing anything.
4. Write to the app data directory atomically, with the commit hash and date alongside.

Acceptance rule (designer's call): a download is accepted only if at least 1,000 entries have a recognised `removal` level, at least one is `Unsafe`, and the recognised count is at least half the stored list's. This stops a renamed field or bad commit from quietly turning every rating into Not rated and lifting the Unsafe block. A rejected update keeps the stored list and is reported when the user asked for it. There is no override.

A failed or invalid download never replaces a stored list. These are the only new internet requests: no analytics, no device data, and no package names are sent. A corrupt stored file is treated as no list.

Licensing: the file is in a GPL-3.0 repo with no separate data licence found. Do not depend on `uad-core` or copy its code, and do not bundle the file. Credit UAD-NG in the Ratings sheet. Get this reviewed before a store release.

## List order

Each app belongs to exactly one group:

| Order | Group | Rule | Colour and icon |
|---|---|---|---|
| 1 | Installed by you | In `pm list packages -3` for the profile, whatever its rating | Group header is neutral |
| 2 | Recommended | System app, `Recommended` | Green, `check` |
| 3 | Advanced | System app, `Advanced` | Yellow, `settings` |
| 4 | Not rated | System app, not in the list or no usable level | Grey, `info` |
| 5 | Expert | System app, `Expert` | Red, `warning` |
| 6 | Unsafe | System app, `Unsafe` | Magenta, `close` |

Inside a group, sort by label ignoring case, then package ID. This matches the existing sort.

An app in "Installed by you" that UAD does rate keeps its rating badge on its row.

Colour never stands alone: each rated row shows the icon and the level's name. Use theme tokens that meet contrast in both themes: `--bc-success`, `--bc-warning` and `--bc-danger`, plus a new magenta token. `--bc-danger` is pink in the dark theme, so adjust Expert or Unsafe until they are clearly different.

"Not rated" means UAD has no opinion. Its grey must not read as disabled, and its explanation must say unknown, not low risk.

Headers show the group name and count. All groups are open except Unsafe, which starts collapsed. A search expands all groups so results are never hidden. Filters apply first, and the order applies within the filtered result.

With no list stored, show the current flat list and no headers.

## Backend

Extend `Query::Packages` in `crates/adb-core/src/workspace/queries.rs`:

- Rust owns ordering because paging happens there. Sort by `(group, label, package)` before `paged`.
- Add a `pm list packages --user {user} -3` call when the filter does not already imply it. If it fails, fall back to rating groups and set `metadataNote`.
- Each item gains `group` and `rating`, or null. The page gains `groups`, an array of `{ id, count }` in order. Counts cover the whole filtered result, not the page.
- `Query::Package` also returns `rating`.

Collapsed Unsafe works with paging because it is always last: the client stops requesting pages once it reaches the collapsed group, and resumes when expanded.

New Tauri commands cover list status, download, update check and removal. Do not hold the ADB backend lock during network calls.

## App detail

The detail view has Overview, Actions, Access and Details tabs and opens on Overview. Designer's call: put the Rating section at the top of Overview, so it shows on opening. Add a compact level badge in the detail heading, so it stays visible on every tab. Show the Unsafe warning at the top of Actions, next to what it applies to.

The Rating section contains:

- The level badge and its plain explanation, using UAD's definitions from `AppBadgeInfo.kt` in Canta, reworded where they imply safety.
- The description, as plain text. URLs may be shown but need not be links.
- Source (`list`).
- "Needs" and "Needed by", marking which are on this TV. When empty, say "None listed", with a note that this doesn't prove there are none.
- "Community rating, mostly based on phones", plus the list date.

Without a rating, show "Not rated" and that UAD has no entry for this package. Without a stored list, leave the section out.

List text is untrusted remote data. Render it as text only, never as HTML, and never put it in a shell command.

## Safeguards

- No group or level ever preselects an app. There is no "select all" for a group.
- For `Unsafe` packages, `disable`, `remove` and `clearData` are blocked by default in both single and bulk flows. Other actions stay available.
- Bulk (designer's call): Unsafe rows stay selectable, because actions such as restore must still work. `Bulk::prepare` already moves refused packages into `excluded`, so the three blocked actions appear there with the reason.
- Detail: the three blocked actions are replaced by an "Unsafe" warning and a per-app "Show these actions anyway" control. It lasts for that detail view only and is never remembered.
- Rust enforces this. `prepare` refuses those actions on an Unsafe package unless the request carries an explicit override, so a UI bug can't skip the block. `prepareBulk` has no override.
- Action and bulk reviews show the package's rating when there is one. For Expert and Unsafe it is prominent.

## Failure states

| State | Behaviour |
|---|---|
| Offline at first download | Card shows the failure and a retry. Apps stays usable |
| Offline later | Stored ratings keep working, with the list date in the sheet |
| Rate limited by GitHub | Same as offline |
| Invalid download | Stored list kept, failure shown only if the user asked for the update |
| Corrupt stored list | Treated as no list, and the card returns |
| `-3` lookup fails | Rating groups only, with the existing "some details unavailable" note |

## Testing

- Rust: parsing with bad and partial entries, case handling, unknown levels, group assignment, sort order, counts across pages, size limit, failed download keeping the old list, Unsafe enforcement with and without override.
- Frontend: headers and counts, Unsafe collapsed and resuming paging, search expanding groups, no-list behaviour matching today, detail with and without a rating, bulk exclusion, no HTML rendering of list text.
- Fixtures must include real entries such as `com.android.tv.settings` (Unsafe) and `com.google.android.tvlauncher` (Expert).
- Manual, on the real TV: note the share of packages in Not rated. If it dominates, report that rather than changing the rule.

## Left to the builder

- The HTTP client. `adb-core` has none today, and it must build for Android, iOS and desktop.
- Where the stored list lives in memory, and the file format for its metadata.
- Exact component structure, spacing and header styling.
- Final wording, within the anti-goals.
- The exact magenta value.
- Whether the Ratings sheet reuses `Modal.svelte`, which is the expected default.
