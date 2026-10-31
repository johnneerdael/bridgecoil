# Bridgecoil brand site implementation plan

**Goal:** Publish the Bridgecoil product site on GitHub Pages and align the README and future releases with the supplied brand kit.

**Design:** Use unchanged outlined logos, Manrope/Inter/IBM Plex Mono fonts, graphite/citron dark colors and the supplied light semantic palette. Build a static landing page and task-oriented guide; leave clearly labelled screenshot slots rather than using simulated brand-kit UI as product evidence.

**Architecture:** Publish only `site/` with GitHub Actions. Use relative asset links for the `/bridgecoil/` project path. Progressive enhancement adds saved System/Dark/Light selection and workspace preview tabs; core navigation and downloads work without JavaScript. Generate release Markdown with a shared Node script, pinned banner and version-specific APK/checksum links; preserve GitHub-generated changes.

**Constraints:** Preserve current app functionality, signing and version numbers. Describe arm64 Android distribution separately from iOS/desktop source builds. Retain emulator versus physical-TV validation boundaries. No new application release.

- [x] Import selected original assets and font licenses; create landing page, guide and screenshot handoff.
- [x] Update README with adaptive banner, feature overview, setup, actual platform status and limitations.
- [x] Add release-note generator, meaningful tests, workflow integration and documentation.
- [x] Validate site links/assets, workflow syntax, generated release body and desktop/mobile/light/dark appearance.
- [x] Commit, deliver through GitHub, configure Pages and verify the deployed site.

## Delivery evidence

- Published from commit `5d26599b76bf3bbab975421323a19d35731081e4`; [Website run 41205646565](https://github.com/johnneerdael/bridgecoil/actions/runs/41205646565) passed checks and deployment.
- Homepage, guide, stylesheet and pinned release banner returned HTTP 200. Live browser inspection confirmed fonts loaded, no broken images and no horizontal overflow at 1440 px.
- Local browser checks covered 390, 1280 and 1440 px layouts, both themes, saved theme across pages, System preference changes, reduced motion, keyboard workspace tabs, FAQ expansion and script-blocked fallback content.
- Automated axe WCAG A/AA scans reported zero violations on the landing page in both themes and the light guide. Decorative arrows/overlays required manual contrast review; this is not a full assistive-technology audit.
- Static link/asset checker, JavaScript syntax checks, three release-renderer tests, actionlint and Git whitespace checks passed. Independent review found no important issues.
- Repository homepage points to the site. App versions and signing settings are unchanged; no app release was created. Product captures remain pending.
