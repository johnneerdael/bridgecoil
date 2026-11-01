# TV compatibility

<p class="lead">Bridgecoil works with Android TV and Google TV devices that allow ADB. What differs is how they connect and whether they keep mappings through a restart.</p>

| Your TV | Connection | First time | Starts after a restart |
|---|---|---|---|
| Android 11 or later, on Wi-Fi | Wireless debugging, encrypted | Six-digit code | Through wireless debugging |
| Android 11 or later, on a cable | Often unavailable | Join Wi-Fi instead | Depends on the TV keeping ADB on |
| Android 10 or earlier | Legacy ADB, unencrypted | Prompt on the TV | Needs ADB left listening on port 5555 |

## Tested on

| Device | Android | Connection | Tested |
|---|---|---|---|
| TCL Smart TV Pro | 14 | Wi-Fi, wireless debugging | Setup, pairing, cold boot, launchers |
| Ugoos AM6 | 9 | Cable, port 5555 | Setup with the approval prompt, button detection, launcher switching with undo, launcher updates, screen capture |
| Android TV emulator | 16 | Legacy ADB | Most screens |

Other TVs should work the same way, and brands differ, so you may find something new. If you do, [report it](../help/report-a-problem.md).

!!! warning "Android 10 and earlier"
    On the Ugoos, the current boot script hasn't been through a cold boot yet. An earlier version of setup, which used a helper app, has.

## Brand differences to expect

- Menu names for Developer options vary.
- Some TVs restrict which apps receive system broadcasts. TCL's `TclAppBoot` does this, which is why Bridgecoil doesn't rely on helper apps there.
- Some stock launchers outrank anything else. See [If the Home button doesn't change](../home-screen/if-home-doesnt-change.md).
- A system update can turn stock apps back on or reset settings.
- Android 14 can leave the pairing screen hidden from network searches. Bridgecoil reads the port from Android's own records.
