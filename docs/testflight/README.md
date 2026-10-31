# TestFlight metadata

Keep the English metadata sources in this directory ready for the current beta. The app description and review notes are separate from the build-specific **What to Test** field.

Before **every beta release**, review and update `what-to-test.en-US.txt` with the testing instructions and known limitations relevant to that release. Commit it with the release changes. The iOS beta workflow adds the marketing version and exact build number, plus an optional `release_focus`, and posts the resulting text to the processed build's `en-US` beta build localization after each TestFlight upload. Empty notes or rendered text over 4,000 characters fail the workflow.

Preview without credentials:

```sh
python3 scripts/testflight_notes.py render --version 0.1.7 --build-number 3.1
```

The upload flow polls App Store Connect for up to 20 minutes, allowing in-flight API requests to finish, and requires a processed iOS build matching the app ID, bundle ID, marketing version and build number. It creates or updates `whatsNew` and reads the saved field back exactly. A passing notes step proves the text is saved for that build; tester distribution and installation remain separate checks.

## Update notes without rebuilding

If processing takes longer, the metadata step fails, or the testing instructions change after upload, run **iOS beta** from `main` with:

- `mode`: `notes`
- `app_version`: the uploaded marketing version, for example `0.1.7`
- `build_number`: the uploaded build number, for example `3.1`
- `release_focus`: optional additional instructions for this run

This mode checks out the notes source and updates the existing build. It skips Node/Rust setup, Homebrew, app tests, building, signing and artifact upload. Repeating the same request verifies the existing text without writing again. Always provide the exact version and build; there is no implicit latest-build target.

Only `APPLE_API_KEY`, `APPLE_API_ISSUER` and `APPLE_API_PRIVATE_KEY` GitHub secrets are required for notes mode. The script also supports `APPLE_API_KEY_PATH` for an existing `.p8` file. It uses the macOS runner's `xcrun altool` to sign tokens and system `curl` for HTTPS with certificate verification. Tokens stay in captured process output and curl's standard input; a temporary private key is created with owner-only permissions and removed when the script exits. An existing key file supplied by path remains owned by the caller.

API writes and signed uploads are restricted to `main`. Updating notes does not submit a build for review, invite or notify testers, change groups, or alter encryption declarations.
