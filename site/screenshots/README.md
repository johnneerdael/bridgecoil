# Screenshot handoff

Product screenshots are deliberately pending. The brand kit’s simulated UI is not a product capture. No missing image URLs are published.

Capture the actual application with non-sensitive device details. Suggested set:

| File | Content | Placement |
| --- | --- | --- |
| `device-dark.webp` / `device-light.webp` | Device and active profile | Homepage workspace; README overview |
| `apps-dark.webp` / `apps-light.webp` | App list and review | Apps tab; guide Apps section |
| `remote-dark.webp` / `remote-light.webp` | Remote and button learning | Remote tab; guide Remote section |
| `tools-dark.webp` / `tools-light.webp` | Toolkit and capability states | Tools tab; guide TV toolkit section |
| `history-dark.webp` / `history-light.webp` | Actual per-operation results | History tab; guide Results section |

Use consistent desktop and phone viewport sizes, crop only when the context remains clear, and write alt text for the workflow shown. Do not include pairing codes, real IP addresses, credentials or private package data.

When captures are ready, replace `.screenshot-placeholder` in `site/index.html` with a real `<picture>` and wire the selected destination in `site/site.js` to the matching capture and alt text. Use the existing `.logo-dark` / `.logo-light` theme selectors or `<picture>` media sources as appropriate; the website’s explicit theme must remain authoritative. Keep a fallback for any destination not yet captured.

Replace the README’s Screenshot status paragraph with a theme-aware `<picture>` only once both referenced files exist. Update guide sections with real captures as useful. Run `python3 scripts/check-site.py` after adding them.
