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

## Delete a saved TV

Use this when you've finished with a TV, such as one that belongs to someone else. Disconnect first, because Saved TVs only shows while you're not connected.

1. Tap **Delete** on the TV's row.
2. Read the warning, which lists what goes.
3. Tap **Delete TV**. **Cancel** leaves everything as it was.

| Removed from your phone | Stays where it is |
|---|---|
| The TV's address and pairing record | Everything on the TV, including apps, mappings and Termux |
| Its records in History, and the option to undo those changes | The TV's trust in your phone |
| Shell output from it, and the Home app remembered for it | Screenshots, recordings and files you saved |

This can't be undone. Your other saved TVs, their History records and your saved commands aren't touched.

The TV still trusts your phone afterwards, so you can add it again without a new pairing code. To end that, remove your phone in the TV's debugging settings. See [Taking away access](../how-it-works/privacy.md#taking-away-access).

If a TV never gave Bridgecoil a stable ID, its History records can't be matched to it, so they stay. The warning says so when this applies, and **Clear records** in History removes them.

## Limits

Bridgecoil doesn't connect by itself when it opens. There's no button to rename a saved TV yet.
