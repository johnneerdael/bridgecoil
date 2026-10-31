# Bridgecoil brand site implementation plan

**Goal:** Publish the Bridgecoil product site on GitHub Pages and align the README and future releases with the supplied brand kit.

**Design:** Use unchanged outlined logos, Manrope/Inter/IBM Plex Mono fonts, graphite/citron dark colors and the supplied light semantic palette. Build a static landing page and task-oriented guide; leave clearly labelled screenshot slots rather than using simulated brand-kit UI as product evidence.

**Architecture:** Publish only `site/` with GitHub Actions. Use relative asset links for the `/bridgecoil/` project path. Progressive enhancement adds saved System/Dark/Light selection and workspace preview tabs; core navigation and downloads work without JavaScript. Generate release Markdown with a shared Node script, pinned banner and version-specific APK/checksum links; preserve GitHub-generated changes.

**Constraints:** Preserve current app functionality, signing and version numbers. Describe arm64 Android distribution separately from iOS/desktop source builds. Retain emulator versus physical-TV validation boundaries. No new application release.

- [x] Import selected original assets and font licenses; create landing page, guide and screenshot handoff.
- [x] Update README with adaptive banner, feature overview, setup, actual platform status and limitations.
- [x] Add release-note generator, meaningful tests, workflow integration and documentation.
- [x] Validate site links/assets, workflow syntax, generated release body and desktop/mobile/light/dark appearance.
- [ ] Commit, deliver through GitHub, configure Pages and verify the deployed site.
