# Bridgecoil website

The public product site lives in `site/` and the user guide in `docs/user-guide/`. Both publish to <https://johnneerdael.github.io/bridgecoil/>, with the guide under `/guide/`. `scripts/build-site.py` stages those pages and the canonical `docs/guide/screenshots/*.png` captures into ignored `dist-site/`. Only that staged directory is uploaded; application source, design experiments and private development artifacts stay out of the Pages artifact.

The site uses plain HTML, CSS and JavaScript. Staging copies the original screenshots without transforming them; no frontend compilation is required. Fonts and original artwork are served locally. It contains no analytics, device access or connection to an ADB endpoint. Theme preference is stored locally when browser storage is available.

## Preview and check

From the repository root:

The guide needs its toolchain: `python3 -m pip install -r docs/site-requirements.txt`.

```sh
python3 scripts/build-site.py
python3 -m http.server 4391 --bind 127.0.0.1
```

Open <http://127.0.0.1:4391/dist-site/>. Re-run the staging command after source edits. Keeping the `/dist-site/` prefix in local previews catches links that accidentally assume a domain-root deployment. The production prefix is `/bridgecoil/`.

```sh
python3 scripts/docs-shots.py --check
python3 scripts/build-site.py
python3 scripts/check-site.py
node --check site/theme.js
node --check site/site.js
node --test scripts/release-notes.test.mjs
actionlint .github/workflows/pages.yml .github/workflows/release.yml
```

The static check validates page links, anchors, asset and font references, image alt attributes and single page headings. Review the landing page and guide at desktop and phone widths, in both themes, after layout changes. Check keyboard navigation, the workspace tabs, FAQ disclosures, reduced motion and theme persistence. Core content and download links remain usable with JavaScript disabled.

## Publish

In repository Settings → Pages, select **GitHub Actions** as the build source. The `Website` workflow validates changes on pull requests and deploys `dist-site/` on relevant pushes to `main`, including updates under `docs/guide/`. It can also be dispatched manually from `main`. Pull requests have no Pages deployment permission and do not use release signing secrets.

The deploy job uses the `github-pages` environment with `pages: write` and `id-token: write`. Keep any environment restrictions consistent with deployment from `main`. No custom domain is configured; update canonical URLs, social metadata, README links and the release renderer together if a domain is added later.

## Brand and content

- `site/assets/tokens/bridgecoil.css`: unchanged semantic light/dark tokens from Brand Kit v1.0.
- `site/assets/logos/`: unchanged outlined SVG logos. Use the supplied variants rather than retyping the wordmark.
- `site/assets/fonts/`: Manrope, Inter and IBM Plex Mono with their OFL licenses.
- `site/assets/marketing/`: original light/dark README and social artwork. The dark README image also heads release notes.
- `site/styles.css`: site composition, accessibility, responsive layouts and reduced-motion handling.
- `site/theme.js`: saved Dark / Light / System preference, applied before paint.
- `site/site.js`: progressive enhancement for the workspace destination tabs.

The homepage previews five destinations, the guide illustrates relevant tasks, and `site/screenshots.html` presents all 51 captures with captions. The originals live in `docs/guide/screenshots/`; keep that directory as the only tracked image source. See [screenshot maintenance](../site/screenshots/README.md). Current captures show signed v0.1.7 in light mode on Android emulators and stay unchanged when the website theme changes. Dark captures and the other gaps listed in the source catalogue remain pending.

Keep platform availability tied to the actual release workflow. It currently publishes Android arm64 APKs and checksums; iOS and desktop are source-build targets. Preserve the distinction between emulator validation and physical-TV acceptance. Update the user guide, README and release renderer together when those facts change.

## User guide

The guide is built with MkDocs Material, the same way as TagGroove and CueRithm, and `scripts/build-site.py` runs it strictly. Read `docs/user-guide/_meta/style.md` before you write a page. Pages are named after what the reader wants to do, and they describe what the code does.

Screenshots still to capture are placeholders in the pages. `python3 scripts/docs-shots.py` lists them in `docs/user-guide/_meta/screenshots.md`, and CI fails if that list is out of date.
