# Bridgecoil screens

Screenshots for the user guide, taken from the signed v0.1.7 build on a phone emulator, connected to an Android TV 16 emulator. No real TV, phone or home network appears in them.

Some details come from the test setup. The TV is called `sdk_google_atv64_arm64`, it connects with Legacy ADB, and `nl.neerdael.testhome` is a small test launcher.

## Connecting

| Screen | File |
|---|---|
| Saved TVs and Quick connect | [saved-tvs](screenshots/saved-tvs.png) |
| Pairing with the TV's six-digit code | [add-device](screenshots/add-device.png) |
| Connecting to a paired TV, with the legacy option | [add-device-connect](screenshots/add-device-connect.png) |
| Help on finding the debugging options | [help](screenshots/help.png) |

## Getting around

| Screen | File |
|---|---|
| Device page | [device](screenshots/device.png) |
| Device details | [device-details](screenshots/device-details.png) |
| Navigation menu and appearance | [navigation](screenshots/navigation.png) |
| Tools, pages 1 to 3 | [1](screenshots/tools-page-1.png), [2](screenshots/tools-page-2.png), [3](screenshots/tools-page-3.png) |

## TV setup

| Screen | File |
|---|---|
| Starting the helper app's service | [tv-setup](screenshots/tv-setup.png) |
| Every change is reviewed first | [setup-start-review](screenshots/setup-start-review.png) |
| Your TV is ready, with the network cable warning | [setup-ready-wired](screenshots/setup-ready-wired.png) |

## Buttons

| Screen | File |
|---|---|
| Before setup is finished | [buttons-before-setup](screenshots/buttons-before-setup.png) |
| Nothing mapped yet | [buttons-empty](screenshots/buttons-empty.png) |
| Press the button | [buttons-listen](screenshots/buttons-listen.png) |
| Listening | [buttons-listening](screenshots/buttons-listening.png) |
| The button was found | [buttons-found](screenshots/buttons-found.png) |
| Choosing what it does | [buttons-map](screenshots/buttons-map.png) |
| Review, with the note that its normal action still happens | [buttons-review](screenshots/buttons-review.png) |
| Saved, with test, pause and remove | [buttons-saved](screenshots/buttons-saved.png) |

## Clean up and Apps

| Screen | File |
|---|---|
| Clean up, with apps you can open left unticked | [clean-up](screenshots/clean-up.png) |
| App list and ratings | [apps](screenshots/apps.png) |
| Selecting several apps | [apps-select](screenshots/apps-select.png) |
| App details | [app-detail](screenshots/app-detail.png) |
| App actions | [app-detail-actions](screenshots/app-detail-actions.png) |
| Ratings list | [app-ratings](screenshots/app-ratings.png) |

## Replacing the Home app

In this order, on a TV whose own launcher comes first.

| Step | File |
|---|---|
| 1. Choose a launcher | [home-1-choose](screenshots/home-1-choose.png) |
| 2. Try it before anything changes | [home-2-try](screenshots/home-2-try.png) |
| 3. Review making it the default | [home-3-review](screenshots/home-3-review.png) |
| 4. Android keeps the TV's launcher | [home-4-result](screenshots/home-4-result.png) |
| 5. What to do about it | [home-5-comes-first](screenshots/home-5-comes-first.png) |
| 6. Review turning the old one off | [home-6-turn-off](screenshots/home-6-turn-off.png) |
| 7. Press Home on the remote | [home-7-press-home](screenshots/home-7-press-home.png) |
| 8. Done, and how to go back | [home-8-in-charge](screenshots/home-8-in-charge.png) |

## History and undo

| Screen | File |
|---|---|
| History | [history](screenshots/history.png) |
| One change, with its recovery | [history-record](screenshots/history-record.png) |
| Reviewing the undo | [history-recovery](screenshots/history-recovery.png) |

## Other tools

| Screen | File |
|---|---|
| Remote | [remote](screenshots/remote.png) |
| Commands | [commands](screenshots/commands.png) |
| A command group | [commands-category](screenshots/commands-category.png) |
| A command's options, preview and save | [commands-options](screenshots/commands-options.png) |
| Screen capture | [screen-capture](screenshots/screen-capture.png) |
| A screenshot of the TV | [capture-screenshot](screenshots/capture-screenshot.png) |
| Files | [files](screenshots/files.png) |
| Diagnostics | [diagnostics-logs](screenshots/diagnostics-logs.png) |
| Choosing from a list instead of typing | [diagnostics-choices](screenshots/diagnostics-choices.png) |
| Log output | [diagnostics-log-output](screenshots/diagnostics-log-output.png) |
| Settings | [settings](screenshots/settings.png) |
| Network | [network](screenshots/network.png) |
| Shell | [shell](screenshots/shell.png) |

## Not captured yet

- **The start of TV setup:** choosing a helper app, installing Termux and the access prompt. They need a TV that hasn't been set up.
- **Nearby devices:** this dialog lists what's on the real network, so it needs a staged network or a mock.
- **Recording in progress, and the file actions menu.**
- **Dark mode.** Everything here is the light theme.
