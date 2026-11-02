# Bulk uninstall and app-list continuity

> **For agentic workers:** Use superpowers:executing-plans to implement inline, then obtain one independent review.

**Goal:** Uninstall 10–20 selected apps with one batch confirmation and continue browsing at the same position after single or bulk changes.

**Architecture:** Reuse `prepareBulk`/`applyBulk` and their existing 32-app limit, exclusions, verification, history and cancellation. Keep AppsPage mounted across operation results; reload its loaded pages in place, preserve the first surviving visible row and prune disappeared selections. New devices/profiles/search/filter views still reset.

**Tech Stack:** Svelte 5, TypeScript/Vitest, Rust integration tests; existing brand styles.

**Spec:** The user's request in this session: bulk delete 10/20 apps, one confirmation for the batch, and no return to the top after uninstall.

## Design and constraints

- Add Uninstall to the existing selection menu, keeping Disable as its established default. Add explicit Select shown (up to 32) and Clear selection controls; never preselect apps.
- Show all included/excluded packages together in the batch review, with expandable impact details and one apply confirmation. Keep stop-after-failure and cancel-remaining behavior.
- Uninstall still deletes added apps and their data for every profile; system/unsafe/current-home protections remain enforced by the backend. No TV is actually modified during development.
- Keep the existing fonts, colors, spacing and mobile viewport. No new animation or dependency.
- Preserve search/filter, loaded depth, selection mode and scroll anchor on operation refresh. Successful removals disappear; remaining selections survive. Discard stale asynchronous results after a target/view change.
- Prefer preserving the component and refreshing its data over rebuilding then reconstructing a second copy of its state.

## Tasks

- [x] Add failing tests: select 20 for uninstall in one preparation; single-operation refresh preserves the list node/depth/position; partial batch preserves remaining selection; review shows every included/excluded app without paging.
- [x] Expose uninstall and bounded explicit selection controls in AppsPage; keep existing protections and review API.
- [x] Replace bulk review paging with a compact expandable list and one confirmation.
- [x] Keep AppsPage mounted; implement same-view refresh with bounded page reload and scroll-anchor restoration, including stale-response cancellation.
- [x] Verify Rust bulk uninstall with per-app readback and system/launcher exclusions; run frontend tests/typecheck/build and relevant core tests.
- [x] Inspect mobile and desktop fixture screenshots; update the user guide and beta testing notes. Obtain one independent review, resolve material findings, and deliver a PR with release notes.

## Review focus

- Refresh while a load-more request is pending must not append stale rows.
- Removing the top visible app or deleting the last page must leave a sensible nearby position.
- Changing device/profile/filter/search must reset the old selection and discard old responses.
- A partial/cancelled batch must not claim all selected apps were removed.
- Mobile controls and a 20-app review must fit without horizontal overflow.

## Evidence

Baseline: 26 focused frontend tests pass on `e36b1b0`.

Implementation evidence: 294 frontend tests, seven focused Rust tests, Svelte checks, production build, strict guide build and site link checks pass. Independent review findings were fixed with a regression for an uninstall followed by refresh failure; paging waits for a successful explicit retry. Mobile and desktop fixture QA and a 0-pixel surviving-row anchor check passed. Delivery continues through the PR and release workflows.
