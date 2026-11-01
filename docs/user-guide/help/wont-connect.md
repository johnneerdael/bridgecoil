# The TV won't connect

<p class="lead">Find the message you're seeing. **Details** on the banner shows the technical text.</p>

## First checks

1. Are the phone and TV on the same network? A guest network often keeps devices apart.
2. Is **Wireless debugging** still on? It can turn itself off.
3. Is the TV on Wi-Fi? Android usually allows wireless debugging only on Wi-Fi.
4. Are you using the right port? The pairing port and the connection port are different.

## Messages

| Message | Cause | What to do |
|---|---|---|
| "The TV closed the TLS handshake." | You used the pairing port to connect | Use the address on the main Wireless debugging screen |
| "No ADB service answered on this port." | The port changed | Check the TV's current address |
| "The TV did not answer on this address." | The address is wrong, or the TV is off or asleep | Check the address, and that the TV is awake |
| "The TV rejected this app's key." | This installation hasn't paired | Use **Pair TV**. Reinstalling the app creates a new key |
| "The TV has not authorized this app's key." | No pairing or approval yet | Pair, or use legacy ADB and approve the prompt |
| "This port uses unencrypted legacy ADB." | The TV offers legacy ADB here | Tick **Allow legacy, unencrypted ADB**, on a network you trust |
| "TV authorization was not completed." | The prompt wasn't approved | Approve it on the TV and try again |
| "Pairing failed" | Wrong code, or the screen closed | Check the address, the code and that the pairing screen is open |
| "Pairing timed out." | The pairing screen closed | Reopen it and check its current port |
| "That address belongs to a different device." | Another TV has this address | Use [Update connection address](../connect/update-address.md) |
| "No connection port was found for this paired TV." | The TV can't be found | See [Find TVs on your network](../connect/find-tvs.md) |
| "ADB connected, but the TV's device inspection did not finish." | The TV didn't answer in time | Try again |
| "Local network access is off" | Your phone is blocking the app | Allow it in **Settings → Apps → Bridgecoil → Permissions**, or on iOS in **Settings → Privacy & Security → Local Network** |

## The approval prompt

**Approve debugging on the TV** means the TV should be showing a prompt. Look for **Allow USB debugging?** and tick the option to always allow. You have about two and a half minutes.

## The connection dropped

Bridgecoil goes back to your saved TVs when the connection is lost. Android may also close the connection a few seconds after you leave the app, including behind a file picker. Tap **Quick connect →** to get back.
