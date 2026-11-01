# Turn on debugging on your TV

<p class="lead">Android keeps its management tools switched off until you turn them on. You do this once, on the TV, with your remote.</p>

Debugging is a standard Android feature called ADB. Bridgecoil can't turn it on for you, because Android doesn't let an outside device do that. It's what allows Bridgecoil to change things a normal app can't touch, so only pair devices you own.

## Step 1: turn on Developer options

1. On the TV, open **Settings → System → About**.
2. Select **Android TV OS build** seven or so times, until the TV says you're a developer.

Menu names vary between brands. On some TVs the same entry is under **Device Preferences → About**, and on others it's called **Build**.

!!! screenshot "tv-developer-options.png"
    On the TV: Settings with Developer options visible.

## Step 2: turn on debugging

Go back to Settings and open **Developer options**.

=== "Android 11 and later"

    Turn on **Wireless debugging**. This is the encrypted connection, and the one to use if your TV has it.

    Android usually allows wireless debugging only on Wi-Fi. If the option is missing or turns itself off, your TV is probably on a network cable. Unplug the cable and join Wi-Fi.

=== "Android 10 and earlier"

    Turn on **USB debugging** or **Network debugging**, whichever your TV offers. These TVs have no wireless debugging, so you'll connect with the older, unencrypted method. Some boxes, such as an Ugoos, offer a setting that keeps network ADB listening on port 5555.

!!! screenshot "tv-wireless-debugging.png"
    On the TV: the main Wireless debugging screen with its IP address and port.

## Which connection will you use?

| | Wireless debugging | Legacy ADB |
|---|---|---|
| Android version | 11 and later | Any |
| Encrypted | Yes, with TLS | No |
| First time | Type a six-digit code from the TV | Accept a prompt on the TV |
| Network | Wi-Fi | Wi-Fi or cable |

Use legacy ADB only on a network you trust, because anyone who can watch your network traffic can read it.

Next: [connect your phone to the TV](first-connection.md).
