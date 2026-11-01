# Find TVs on your network

<p class="lead">Instead of typing an address, let Bridgecoil look for it.</p>

Tap the search icon, labelled **Find nearby ADB devices**. It's on the Saved TVs screen and next to the step tabs. When you're connected, use **Find devices** in the **Device connection** dialog.

The scan takes about four seconds. It only listens for what devices announce on your local network, and it never opens a connection to them. Choosing a result fills in a form, so nothing is paired or connected until you continue.

| Row | Meaning | Fills in |
|---|---|---|
| **Wireless connection** | The TV's connection port | Step 2 |
| **Pairing port** | The pairing screen is open | Step 1 |
| **Legacy ADB** | Older, unencrypted debugging | Step 2. Tick the legacy box yourself |

!!! screenshot "connect-find-empty.png"
    Nearby ADB devices showing the empty state.

The pairing port only appears while the TV's pairing screen is open, so open that screen before you search. Choosing a different TV while connected disconnects the current one.

## If nothing is found

The dialog says "No ADB services found." Check that the phone and TV are on the same network and that Wireless debugging is on. A TV on a network cable usually can't use it. Some routers also block these announcements between devices, in which case you can still type the address by hand.

On Android, if Bridgecoil doesn't have local network access, allow it in **Settings → Apps → Bridgecoil → Permissions**.
