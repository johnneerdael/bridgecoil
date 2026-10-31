# Screenshot maintenance

The canonical images are `docs/guide/screenshots/*.png`. Their captions and capture context are recorded in `docs/guide/README.md`. The current 51 captures come from signed v0.1.7 on a phone emulator connected to an Android TV 16 emulator. They all show the light theme.

Run `python3 scripts/build-site.py` to copy the website and original PNGs into ignored `dist-site/`. The PNGs are copied without resizing, cropping or recoloring. Do not track duplicate captures here. The Pages workflow watches `docs/guide/**` so image updates trigger publication.

- `site/index.html`: paired captures for Device, Apps, Remote, Tools and History. Each panel is static HTML with native image links; `site/site.js` switches panels and supports arrow-key navigation.
- `site/guide.html`: task-specific captures alongside connection, app management, launcher, mapping, toolkit, files and recovery instructions.
- `site/screenshots.html`: every capture, grouped and captioned to match the source catalogue, with links to the full-size PNGs.
- `README.md`: six selected portrait captures linked directly to the canonical files.

When adding captures, update the catalogue and relevant page markup, set descriptive alt text and captions, and preserve the original aspect ratio. Keep the full-size link and image source in sync. If adding another theme, only introduce theme-specific selection once those actual captures exist. See the catalogue's Not captured yet section for current gaps.

After changes, run `python3 scripts/build-site.py`, `python3 scripts/check-site.py`, and check desktop/phone layouts in both website themes. The checker validates the staged output, including every image link. Build output must remain ignored.
