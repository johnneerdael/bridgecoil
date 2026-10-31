# Bridgecoil website

The public product site and user guide live in `site/` and publish to <https://johnneerdael.github.io/bridgecoil/>. Only that directory is uploaded; application source, design experiments and private development artifacts stay out of the Pages artifact.

The site uses plain HTML, CSS and JavaScript with no build step. Fonts and original artwork are served locally. It contains no analytics, device access or connection to an ADB endpoint. Theme preference is stored locally when browser storage is available.

## Preview and check

From the repository root:

```sh
python3 -m http.server 4391 --bind 127.0.0.1
```

Open <http://127.0.0.1:4391/site/>. Keeping the `/site/` prefix in local previews catches links that accidentally assume a domain-root deployment. The production prefix is `/bridgecoil/`.

```sh
python3 scripts/check-site.py
node --check site/theme.js
node --check site/site.js
node --test scripts/release-notes.test.mjs
actionlint .github/workflows/pages.yml .github/workflows/release.yml
```

The static check validates page links, anchors, asset and font references, image alt attributes and single page headings. Review the landing page and guide at desktop and phone widths, in both themes, after layout changes. Check keyboard navigation, the workspace tabs, FAQ disclosures, reduced motion and theme persistence. Core content and download links remain usable with JavaScript disabled.

## Publish

In repository Settings → Pages, select **GitHub Actions** as the build source. The `Website` workflow validates changes on pull requests and deploys `site/` on relevant pushes to `main`. It can also be dispatched manually from `main`. Pull requests have no Pages deployment permission and do not use release signing secrets.

The deploy job uses the `github-pages` environment with `pages: write` and `id-token: write`. Keep any environment restrictions consistent with deployment from `main`. No custom domain is configured; update canonical URLs, social metadata, README links and the release renderer together if a domain is added later.

## Brand and content

- `site/assets/tokens/bridgecoil.css`: unchanged semantic light/dark tokens from Brand Kit v1.0.
- `site/assets/logos/`: unchanged outlined SVG logos. Use the supplied variants rather than retyping the wordmark.
- `site/assets/fonts/`: Manrope, Inter and IBM Plex Mono with their OFL licenses.
- `site/assets/marketing/`: original light/dark README and social artwork. The dark README image also heads release notes.
- `site/styles.css`: site composition, accessibility, responsive layouts and reduced-motion handling.
- `site/theme.js`: saved Dark / Light / System preference, applied before paint.
- `site/site.js`: progressive enhancement for the workspace destination tabs.

Application screenshots are intentionally unresolved. See [the screenshot handoff](../site/screenshots/README.md). Do not present the kit's simulated UI as a capture of the shipped app.

Keep platform availability tied to the actual release workflow. It currently publishes Android arm64 APKs and checksums; iOS and desktop are source-build targets. Preserve the distinction between emulator validation and physical-TV acceptance. Update the user guide, README and release renderer together when those facts change.
