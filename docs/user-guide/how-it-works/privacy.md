# What leaves your phone and TV

<p class="lead">Bridgecoil has no account, no advertising and no analytics. Your TV talks to your phone directly, with no server of ours in between.</p>

The [privacy page](https://johnneerdael.github.io/bridgecoil/privacy.html) has the formal version.

## Between your phone and TV

Commands, app details, files, screenshots and logs travel directly over your home network. With wireless debugging that's encrypted with TLS. Legacy ADB isn't encrypted, so anyone able to read your network traffic could read it too.

## To the internet

Bridgecoil contacts GitHub, and only for these:

| When | What it requests |
|---|---|
| You download ratings, or open Apps with ratings saved (at most once a day) | The Universal Android Debloater list, from `api.github.com` and `raw.githubusercontent.com` |
| You open Home app | The latest release of each listed launcher, from `api.github.com` |
| You install a tool or launcher | The release file, from `github.com` |

These are ordinary downloads. They carry no information about your TV, though GitHub sees your IP address as it does for any visitor.

The agent and listener on the TV don't contact the internet. The agent's only traffic is local: a local network announcement to find the ADB port, and a connection to the TV itself.

## Finding TVs

**Find nearby ADB devices** listens for announcements on your local network for about four seconds, and doesn't connect to anything.

## On your phone

History stays on your phone. It includes the commands you ran, and their output, up to 64 KB each. That means anything you typed into [Shell](../tools/shell.md) is stored, so don't put passwords there. **Clear records** removes it.

Saved TVs store their address, name and a hash of the device ID. Your ADB key stays in the app's private data. Files and screenshots go only where you save them.

## On iPhone

If you test through TestFlight, Apple may collect crash and usage data. That's outside Bridgecoil.

## Taking away access

Disconnecting doesn't remove your phone's permission. To do that, remove it on the TV, in **Wireless debugging** under paired devices, or use **Revoke USB debugging authorizations** in Developer options. See [Remove everything](remove-everything.md).
