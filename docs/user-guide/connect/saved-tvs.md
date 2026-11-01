# Reconnect to a saved TV

<p class="lead">Every TV you pair is saved on your phone, so you can come back to it with one tap.</p>

Open Bridgecoil while you're not connected and you see **Saved TVs**. Each row shows the TV's name, its address and its status: **Previously connected**, or **Paired · connection address needed** if you paired but never connected. **Legacy ADB** is added for a TV that uses the older connection.

![Saved TVs with two TVs and a Quick connect button on each.](../assets/shots/connect-saved-tvs.png){ .bc-shot loading=lazy }

## Quick connect

Tap **Quick connect →**. Bridgecoil scans your network for a few seconds to find the TV's current port, connects, and checks that the TV is the one you saved. If the scan can't run, it tries the saved address.

| What you see | What it means | What to do |
|---|---|---|
| "No connection port was found for this paired TV." | Wireless debugging is off, or the TV isn't on this network | Turn it on, or enter the address from the main Wireless debugging screen |
| "That address belongs to a different device." | Another TV now has this address | Use **Update connection address** |
| The connection form with an error | Quick connect couldn't finish | Read the error, or use **← Saved devices** to go back |

## Add another TV

Tap **Add device** and follow [Connect your phone to the TV](../get-started/first-connection.md). You can save up to 32 TVs, and only one is connected at a time.

## Limits

Bridgecoil doesn't connect by itself when it opens. There's no button to rename or forget a saved TV yet. To clear the list, clear Bridgecoil's data in your phone's settings, which also removes its pairing key, so you'd need to pair each TV again.
