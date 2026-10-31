# App Ratings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show UAD-NG community ratings on the Apps screen, group the list by removal risk, show everything known in app detail, and block risky actions on Unsafe packages by default.

**Architecture:** A new `adb_core::ratings` module parses, validates, stores and fetches the list. Rust owns ordering because paging happens there, so `Query::Packages` joins ratings before `paged`. `Workspace` enforces the Unsafe block, and the Svelte layer only presents it.

**Tech Stack:** Rust 2024, Tauri 2, Svelte 5, Vitest. New dependency: `ureq` 3 with its `rustls` feature.

**Spec:** `docs/superpowers/specs/2026-10-31-app-ratings-design.md`

## Global Constraints

- Never use the word "safe" as a label or claim. "Unsafe" is only the level's name.
- Ratings are opt-in. With no stored list, the Apps screen behaves exactly as it does today, plus one dismissible card.
- No bundled list. No dependency on, or code copied from, `uad-core`.
- Only two new internet requests exist, both to GitHub. No analytics, device data or package names are sent.
- A failed, invalid or rejected download never replaces a stored list.
- Acceptance rule: at least 1,000 entries with a recognised `removal` level, at least one `Unsafe`, and a recognised count at least half the stored list's. No override.
- Reject response bodies over 8 MiB.
- List text is untrusted. Render it as text only, never as HTML, and never put it in a shell command.
- No group or level ever preselects an app. There is no "select all" for a group.
- Blocked Unsafe actions are exactly `disable`, `remove` and `clearData`.
- Do not hold the ADB backend lock during network calls.
- Colour never stands alone: every rated row shows the icon and the level's name.
- **Do not commit.** The worktree holds Codex's uncommitted work and Codex edits it concurrently. Re-read a file immediately before editing it, and leave Codex's in-progress files alone.

## Review Focus

- `removal` is a number, or `dependencies` is a string, in one entry: that entry still loads, with the bad field treated as missing.
- GitHub rate limits and answers 403 with a JSON object instead of a commit array: treated as a failed check, and the stored list stays.
- An Unsafe app the user installed themselves: it stays in "Installed by you", and the three actions are still blocked.
- The filter or search changes while a page is loading, or the list ends exactly at the collapsed Unsafe header: no duplicate rows, no endless "loading".
- Ratings are removed while the Apps screen is open: the next load returns to the flat list, with no headers left behind.

---

### Task 1: Parse and validate the list

**Files:**
- Create: `crates/adb-core/src/ratings/mod.rs`
- Modify: `crates/adb-core/src/lib.rs`
- Test: `crates/adb-core/tests/ratings_list.rs`

**Interfaces:**
- Produces:
  - `enum Level { Recommended, Advanced, Expert, Unsafe }`, serialised camelCase.
  - `struct Rating { level: Option<Level>, source: Option<String>, description: String, dependencies: Vec<String>, needed_by: Vec<String> }`, serialised camelCase.
  - `struct RatingList { entries: HashMap<String, Rating> }` with `get(&self, package: &str) -> Option<&Rating>` and `recognised(&self) -> usize`.
  - `parse_list(raw: &str) -> Result<RatingList, PocError>`.
  - `check_acceptable(new: &RatingList, stored: Option<&RatingList>) -> Result<(), PocError>`, error kind `"ratingsRejected"`.
  - `const MAX_LIST_BYTES: usize = 8 * 1024 * 1024`, `const MIN_RECOGNISED: usize = 1000`.

- [ ] **Step 1: Write failing tests**

Build a `sample_list(recommended: usize, unsafe_count: usize) -> String` helper in the test file. Tests:
- `parses_all_fields`: a full entry gives `level == Some(Level::Expert)`, its source, description, `dependencies` and `needed_by` (JSON key `neededBy`).
- `levels_ignore_case`: `"unsafe"` and `"UNSAFE"` give `Level::Unsafe`.
- `unknown_levels_are_not_rated`: `"Unlisted"`, `""` and a missing `removal` give `level == None`, and the entry remains.
- `wrongly_typed_fields_are_treated_as_missing`: `{"removal":5,"description":7,"dependencies":"x","neededBy":null,"list":[]}` loads with `level == None`, empty description and empty arrays.
- `non_string_array_items_are_dropped`: `"dependencies":["a",1]` gives `["a"]`.
- `non_object_entries_are_skipped`: `{"a":3,"b":{"removal":"Expert"}}` has one entry.
- `invalid_json_is_an_error`, and `a_top_level_array_is_an_error`.
- `unknown_fields_are_ignored`.
- `accepts_a_normal_list`: `check_acceptable(&sample_list(1000, 1), None)` is ok.
- `rejects_too_few_recognised`: 999 recognised.
- `rejects_no_unsafe`: 2000 recognised, none Unsafe.
- `rejects_losing_most_of_the_stored_list`: new 1000 against stored 3000.
- `accepts_exactly_half_the_stored_list`: new 1500 against stored 3000.

- [ ] **Step 2: Run them to confirm they fail**

Run: `node scripts/cargo.mjs test -p adb-core --test ratings_list`
Expected: compile error, `ratings` not found.

- [ ] **Step 3: Implement**

Parse into `serde_json::Value` and read fields by hand, because a typed struct would reject a whole file for one bad field. Add `pub mod ratings;` to `lib.rs`.

- [ ] **Step 4: Run them to confirm they pass**

Run: `node scripts/cargo.mjs test -p adb-core --test ratings_list`
Expected: all pass.

### Task 2: Store and fetch the list

**Files:**
- Create: `crates/adb-core/src/ratings/store.rs`, `crates/adb-core/src/ratings/fetch.rs`
- Modify: `crates/adb-core/src/ratings/mod.rs`, `crates/adb-core/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/workspace.rs`
- Test: `crates/adb-core/tests/ratings_store.rs`

**Interfaces:**
- Consumes: `parse_list`, `check_acceptable`, `RatingList`, `MAX_LIST_BYTES` from Task 1.
- Produces:
  - `struct Commit { sha: String, date: String }`.
  - `trait ListSource { fn latest_commit(&self) -> Result<Commit, PocError>; fn download(&self, sha: &str) -> Result<String, PocError>; }`.
  - `struct GithubSource`, implementing `ListSource` over blocking `ureq`.
  - `struct RatingsStatus { stored: bool, commit: Option<String>, committed_at: Option<String>, packages: usize }`, serialised camelCase.
  - `struct Ratings`, with:
    - `open(dir: &Path) -> Self`: loads the stored list, and treats a corrupt file as no list.
    - `status(&self) -> RatingsStatus`.
    - `current(&self) -> Option<Arc<RatingList>>`.
    - `update(&self, source: &dyn ListSource) -> Result<bool, PocError>`: `true` when the list changed.
    - `update_if_due(&self, source: &dyn ListSource, now_ms: i64) -> Result<bool, PocError>`: a no-op with no stored list or within 24 hours of the last check.
    - `remove(&self) -> Result<(), PocError>`.
  - Tauri commands `ratings_status`, `ratings_update`, `ratings_update_if_due`, `ratings_remove`.

- [ ] **Step 1: Write failing tests**

Use a `FakeSource` that counts calls and returns scripted results, plus `tempfile::tempdir()`. Tests:
- `first_update_stores_the_list_and_commit`: `status()` reports the commit, date and package count.
- `a_new_instance_reloads_what_was_stored`.
- `same_commit_downloads_nothing`: the second `update` returns `false` and `download` ran once.
- `failed_check_keeps_the_stored_list`.
- `failed_download_keeps_the_stored_list`.
- `invalid_json_keeps_the_stored_list`.
- `a_rejected_list_keeps_the_stored_list`: the error kind is `"ratingsRejected"`.
- `corrupt_stored_file_counts_as_no_list`.
- `remove_clears_memory_and_disk`, checked with a fresh `open`.
- `due_check_needs_a_stored_list`: `download` and `latest_commit` never run.
- `due_check_runs_once_a_day`: at `t` and `t + 23h` it calls once, at `t + 24h` it calls again.
- `failed_due_check_still_waits_a_day`: a failure records the check time.

- [ ] **Step 2: Run them to confirm they fail**

Run: `node scripts/cargo.mjs test -p adb-core --test ratings_store`
Expected: compile error.

- [ ] **Step 3: Implement `store.rs`**

Files are `uad_lists.json` and `uad_lists.meta.json` in the given directory. Write each to a temp file in the same directory and rename, list first. Hold the list in `RwLock<Option<Arc<RatingList>>>`.

- [ ] **Step 4: Implement `fetch.rs`**

Add `ureq = { version = "=<newest 3.x at least two weeks old>", default-features = false, features = ["rustls"] }` to `adb-core`. Its `rustls` feature bundles root certificates, so Android needs no platform verifier. Send a `User-Agent`, which the GitHub API requires, and use a 30-second timeout.

URLs:
- `https://api.github.com/repos/Universal-Debloater-Alliance/universal-android-debloater-next-generation/commits?path=resources%2Fassets%2Fuad_lists.json&per_page=1`
- `https://raw.githubusercontent.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation/{sha}/resources/assets/uad_lists.json`

Add a pure `parse_commit(body: &str) -> Result<Commit, PocError>`, reading `[0].sha` and `[0].commit.committer.date`. Add tests for it in `ratings_store.rs`: a real-shaped array parses, and a rate-limit object `{"message":"API rate limit exceeded"}` and an empty array are errors. Validate that `sha` is 40 hex characters before putting it in a URL, and test that too.

Enforce `MAX_LIST_BYTES` while reading the body.

- [ ] **Step 5: Add the Tauri commands**

Manage `Arc<Ratings>`, opened from `data.join("ratings")`, in `setup`. Run updates with `tauri::async_runtime::spawn_blocking`. `ratings_update` and `ratings_update_if_due` take no backend state, so they can't hold its lock. Register all four in `generate_handler!`.

- [ ] **Step 6: Run the tests and workspace check**

Run: `node scripts/cargo.mjs test -p adb-core --test ratings_store && node scripts/cargo.mjs check --workspace`
Expected: tests pass and the workspace compiles.

### Task 3: Group and sort packages

**Files:**
- Create: `crates/adb-core/src/ratings/groups.rs`
- Modify: `crates/adb-core/src/ratings/mod.rs`, `crates/adb-core/src/workspace/queries.rs`, `src-tauri/src/workspace.rs`
- Test: `crates/adb-core/tests/ratings_groups.rs`

**Interfaces:**
- Consumes: `RatingList`, `Rating`, `Level`, `Ratings::current`.
- Produces:
  - `enum Group { Installed, Recommended, Advanced, NotRated, Expert, Unsafe }`, in that declared order, deriving `Ord`, serialised camelCase.
  - `group_for(rating: Option<&Rating>, user_installed: bool) -> Group`.
  - `queries::query_rated(d: &mut dyn ShellDriver, t: &Target, q: Query, ratings: Option<&RatingList>) -> Result<Value>`. `queries::query` delegates with `None`, so existing tests stay untouched.
  - Page JSON gains `groups: [{ id, count }]`, in order, when a list is given. Items gain `group` and `rating`.
  - `Query::Package` JSON gains `rating` and `installedRelated: string[]`.

- [ ] **Step 1: Write failing tests**

Use a fake driver in the style of `crates/adb-core/tests/workspace_actions.rs`. Tests:
- `group_for_covers_every_case`, including a user-installed Unsafe app giving `Group::Installed`.
- `orders_groups_then_label_then_package`: mixed packages come back as Installed, Recommended, Advanced, NotRated, Expert, Unsafe, alphabetical by label inside each.
- `counts_cover_the_whole_filtered_result`: with `page_size` 2, `groups` still counts everything.
- `search_filters_before_counting`.
- `user_filter_only_has_the_installed_group` and makes no extra `pm` call.
- `failed_user_lookup_falls_back_to_rating_groups` and sets `metadataNote`.
- `no_list_keeps_todays_shape`: no `groups` key, label-sorted, and only one `pm list` call.
- `package_detail_includes_its_rating_and_marks_related_packages_on_this_tv`.
- `package_detail_makes_no_extra_call_without_related_packages`.

- [ ] **Step 2: Run them to confirm they fail**

Run: `node scripts/cargo.mjs test -p adb-core --test ratings_groups`
Expected: compile error.

- [ ] **Step 3: Implement**

In `Query::Packages`, when a list is given and the filter is not `"user"`, run `pm list packages --user {user} -3`. Extend the existing sort key to `(group, label, package)`. For `"user"`, every package is `Installed`.

Wire `workspace_query` to pass `ratings.current()`. Take the `Arc` before locking the backend.

- [ ] **Step 4: Run the tests**

Run: `node scripts/cargo.mjs test -p adb-core`
Expected: all pass, including existing query tests.

### Task 4: Enforce the Unsafe block

**Files:**
- Modify: `crates/adb-core/src/workspace/engine.rs`, `crates/adb-core/src/workspace/bulk.rs`, `src-tauri/src/workspace.rs`, `src-tauri/src/lib.rs`
- Test: `crates/adb-core/tests/ratings_block.rs`

**Interfaces:**
- Consumes: `Ratings`, `Level`.
- Produces:
  - `Workspace::with_ratings(self, ratings: Arc<Ratings>) -> Self`.
  - `Workspace::prepare_with(&self, d, target, action, allow_unsafe: bool) -> Result<Review>`. `prepare` delegates with `false`, so bulk and recovery get no override.
  - `Review.rating: Option<Level>`.
  - `prepare_action` gains `allow_unsafe: Option<bool>`.
  - Error kind `"unsafeRating"`.

- [ ] **Step 1: Write failing tests**

- `unsafe_packages_refuse_the_three_actions` for `Disable`, `Remove` and `ClearData`, with no state-changing command sent.
- `other_actions_stay_available_on_unsafe_packages`, for `Enable`, `ForceStop`, `Restore`, `ClearCache` and `ResetEnabled`.
- `an_explicit_override_allows_the_action`.
- `the_block_applies_to_an_unsafe_app_the_user_installed`.
- `expert_packages_are_not_blocked`.
- `bulk_moves_blocked_packages_to_excluded_with_the_reason`, while a `Restore` bulk includes them.
- `review_carries_the_rating`.
- `no_list_means_no_block`.

- [ ] **Step 2: Run them to confirm they fail**

Run: `node scripts/cargo.mjs test -p adb-core --test ratings_block`
Expected: compile error.

- [ ] **Step 3: Implement**

Put the check in `prepare_with`, before `probe`. The refusal message must say the package is rated Unsafe by the community list and can break vital parts of Android. In `lib.rs`, build `Workspace::new(...).with_ratings(ratings.clone())`.

- [ ] **Step 4: Run the tests**

Run: `node scripts/cargo.mjs test -p adb-core`
Expected: all pass.

### Task 5: Client types, rating copy and badge

**Files:**
- Create: `src/lib/workspace/ratings.ts`, `src/lib/workspace/RatingBadge.svelte`
- Modify: `src/lib/workspace/client.ts`, `src/lib/theme/tokens.css`, `src/styles.css`, `tests/ui/platform.ts`
- Test: `src/lib/workspace/ratings.test.ts`

**Interfaces:**
- Consumes: the JSON from Tasks 2 to 4.
- Produces:
  - Types `Level`, `Group`, `Rating`, `RatingsStatus`. `AppMetadata` gains optional `group` and `rating`. `PackageDetail` gains optional `rating` and `installedRelated`. `Review` gains optional `rating`.
  - `Platform` gains `ratingsStatus(): Promise<RatingsStatus>`, `ratingsUpdate(): Promise<boolean>`, `ratingsUpdateIfDue(): Promise<boolean>` and `ratingsRemove(): Promise<void>`. `prepare` gains an optional third argument, `allowUnsafe?: boolean`.
  - `levelInfo: Record<Level | "notRated", { label, icon, tone, explanation }>`.
  - `groupLabels: Record<Group, string>`.
  - `RatingBadge.svelte` with prop `level: Level | null`.

Exact copy:

| Key | Label | Icon | Explanation |
|---|---|---|---|
| `recommended` | Recommended | `check` | Pointless on most devices, or easy to get back from Google Play. |
| `advanced` | Advanced | `settings` | May break minor features, or has a better replacement. |
| `expert` | Expert | `warning` | Breaks important features. Removing it shouldn't stop the device starting, but that isn't guaranteed. |
| `unsafe` | Unsafe | `close` | Can break vital parts of Android and may stop the device starting. |
| `notRated` | Not rated | `info` | UAD has no rating for this. That means unknown, not low risk. |

The `installed` group label is "Installed by you".

- [ ] **Step 1: Write failing tests**

- `every_level_has_a_label_icon_and_explanation`.
- `no_copy_calls_anything_safe`: no label or explanation matches `/\bsafe\b/i`.
- `the_badge_shows_the_icon_and_name`: for each level, the rendered text contains the label and an icon is present.
- `the_badge_treats_null_as_not_rated`.

- [ ] **Step 2: Run them to confirm they fail**

Run: `npx vitest run src/lib/workspace/ratings.test.ts`
Expected: FAIL, module not found.

- [ ] **Step 3: Implement**

Add `--bc-unsafe` and `--bc-unsafeSoft` to both themes in `tokens.css`, with a magenta clearly different from `--bc-danger`, which is pink in the dark theme. Adjust the Expert badge if the two still look alike. Tones use `--bc-success`, `--bc-warning`, `--bc-danger`, `--bc-unsafe` and `--muted`, with text contrast of at least 4.5:1 in both themes. Not rated must not look disabled, so keep its text at full `--ink` strength.

Extend the fixtures in `tests/ui/platform.ts` so every group appears.

- [ ] **Step 4: Run the tests and type check**

Run: `npx vitest run src/lib/workspace/ratings.test.ts && npm run -s check`
Expected: pass, 0 errors.

### Task 6: Grouped Apps list

**Files:**
- Modify: `src/lib/workspace/AppsPage.svelte`, `src/styles.css`
- Test: `src/lib/workspace/apps-ratings.test.ts`

**Interfaces:**
- Consumes: `AppMetadata.group`, `.rating`, page `groups`, `RatingBadge`, `groupLabels`.

- [ ] **Step 1: Write failing tests**

- `shows_headers_with_counts_in_order`.
- `each_row_shows_its_rating_badge`, including a rated app under "Installed by you".
- `unsafe_starts_collapsed_and_expands`: rows are hidden until the header is tapped.
- `collapsed_unsafe_stops_paging`: scrolling doesn't request a page past the first Unsafe row, and expanding resumes it.
- `a_list_ending_at_collapsed_unsafe_stops_loading`: no "Reading more apps…" stays on screen.
- `search_expands_every_group`.
- `changing_the_filter_mid_load_shows_no_stale_rows`.
- `no_list_shows_the_flat_list_and_no_headers`.
- `removing_ratings_returns_to_the_flat_list_on_the_next_load`.
- `no_group_preselects_or_offers_select_all`: entering Select mode leaves `selected` empty, and no "select all" control exists.

- [ ] **Step 2: Run them to confirm they fail**

Run: `npx vitest run src/lib/workspace/apps-ratings.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement**

Render a header whenever an item's `group` differs from the previous one, using `result.groups` for the count. Hold `unsafeOpen = $state(false)`. In `loadMore`, return early when the next unloaded item would be in the Unsafe group and `unsafeOpen` is false and no search is active. Use `result.groups` to compute that: the sum of the counts before `unsafe` is where loading stops. Hide Unsafe rows in the same condition. Keep the header visible so it can be expanded.

- [ ] **Step 4: Run the tests**

Run: `npx vitest run`
Expected: all pass.

### Task 7: Opt-in card and Ratings sheet

**Files:**
- Create: `src/lib/workspace/RatingsSheet.svelte`
- Modify: `src/lib/workspace/AppsPage.svelte`
- Test: `src/lib/workspace/ratings-sheet.test.ts`

**Interfaces:**
- Consumes: the `Platform` ratings methods.
- Produces: `RatingsSheet.svelte` with props `open: boolean`, `api: Platform`, `onclose: () => void`, `onchange: () => void`. `onchange` reloads the list.

- [ ] **Step 1: Write failing tests**

- `offers_ratings_only_when_none_are_stored`.
- `nothing_downloads_until_asked`: `ratingsUpdate` isn't called on load.
- `the_card_names_the_source_and_github`.
- `dismissing_keeps_the_card_away_after_a_reload`, using `localStorage` key `bridgecoil.ratingsCardDismissed`.
- `the_summary_line_opens_the_sheet_after_dismissal`.
- `a_failed_download_shows_a_retry_and_keeps_apps_usable`.
- `the_sheet_shows_source_commit_and_date`, with a UAD-NG link.
- `check_for_update_reports_failure`.
- `remove_ratings_returns_to_the_card`.
- `the_daily_check_runs_once_on_open_and_stays_silent_on_failure`.

- [ ] **Step 2: Run them to confirm they fail**

Run: `npx vitest run src/lib/workspace/ratings-sheet.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement**

The card's text: "Get community ratings. Downloads the Universal Android Debloater list from GitHub to show what each app does and how risky it is to remove." Its buttons are "Download" and "Not now". Build the sheet on `Modal.svelte`. Show the short commit and the date, and credit UAD-NG with "Community list, mostly based on phones."

Call `ratingsUpdateIfDue` on mount, ignore errors, and reload only if it returns `true`. The sheet is reached from the "Profile … · N apps" summary line, so add a "Ratings" text action there.

- [ ] **Step 4: Run the tests**

Run: `npx vitest run`
Expected: all pass.

### Task 8: App detail, blocked actions and reviews

**Files:**
- Create: `src/lib/workspace/RatingDetail.svelte`
- Modify: `src/lib/workspace/AppsPage.svelte`, `src/lib/workspace/ActionReview.svelte`, `src/lib/workspace/BulkReview.svelte`
- Test: `src/lib/workspace/rating-detail.test.ts`

**Interfaces:**
- Consumes: `PackageDetail.rating`, `.installedRelated`, `Review.rating`, `prepare(target, action, allowUnsafe)`.
- Produces: `RatingDetail.svelte` with props `rating: Rating | null`, `installedRelated: string[]`, `listDate: string | null`.

- [ ] **Step 1: Write failing tests**

- `shows_level_explanation_description_and_source`.
- `list_text_renders_as_text_only`: a description containing `<img src=x onerror=alert(1)>` creates no `img` element.
- `dependencies_mark_what_is_on_this_tv`.
- `empty_dependencies_say_none_listed`, with the note that this doesn't prove there are none.
- `an_unlisted_package_says_not_rated`.
- `the_section_is_absent_without_a_stored_list`.
- `the_heading_badge_shows_on_every_tab`.
- `a_long_description_keeps_the_detail_usable`: a 1,513-character description leaves the tabs reachable.
- `unsafe_hides_the_three_actions_behind_a_warning`.
- `the_override_reveals_them_for_this_view_only`: reopening the app hides them again.
- `the_override_sends_allow_unsafe`, and other actions never do.
- `expert_shows_no_block`.
- `reviews_show_the_rating_prominently_for_expert_and_unsafe`, in both single and bulk reviews.

- [ ] **Step 2: Run them to confirm they fail**

Run: `npx vitest run src/lib/workspace/rating-detail.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement**

Place `RatingDetail` at the top of the Overview tab, and a compact `RatingBadge` in the detail heading. At the top of Actions, for Unsafe apps, show the warning and a "Show these actions anyway" button. Filter `disable`, `remove` and `clearData` out of `actions` until it's pressed. Reset it in `inspect()`.

- [ ] **Step 4: Run the tests**

Run: `npx vitest run`
Expected: all pass.

### Task 9: Final verification

**Files:**
- Modify: `tests/ui/Harness.svelte` only if a new screen needs a fixture.
- Create: `docs/validation/app-ratings.md`

- [ ] **Step 1: Run every automated check**

Run: `npx vitest run && npm run -s check && node scripts/cargo.mjs test --workspace && node scripts/cargo.mjs clippy --workspace --all-targets -- -D warnings`
Expected: all pass, except clippy issues in files that aren't part of this plan, which are listed rather than fixed.

- [ ] **Step 2: Check the real list**

Run a throwaway test that passes the real `uad_lists.json` through `parse_list` and `check_acceptable`.
Expected: 5,000 or more entries are recognised, and `com.android.tv.settings` is Unsafe and `com.google.android.tvlauncher` is Expert.

- [ ] **Step 3: Check the real network path**

Run `GithubSource::latest_commit` and `download` once from a throwaway test on the Mac.
Expected: a 40-character hash and an accepted list.

- [ ] **Step 4: Review the screens at phone width**

Use the UI harness on 412×915 in both themes. Check each level is distinguishable, headers don't overflow, and there is no horizontal scroll.

- [ ] **Step 5: Test on the TV, once an APK exists**

Install only when asked. Then check the opt-in download on the phone, the list order and the Unsafe block, without applying any action. Record the share of packages in Not rated in `docs/validation/app-ratings.md`. If it dominates, report that rather than changing the rule.
