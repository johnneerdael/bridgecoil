# Connect your phone to the TV

<p class="lead">The first connection takes two steps: pair once, then connect. After that, Quick connect does it in one tap.</p>

**You need:**

- [ ] Your phone and TV on the same network
- [ ] [Debugging turned on](prepare-your-tv.md) on the TV

Pairing and connecting use different ports on the TV, which is where most first-time problems come from. The pairing port only exists while the pairing screen is open. The connection port is on the main Wireless debugging screen.

=== "Wireless debugging"

    1. On the TV, open **Wireless debugging** and choose **Pair device with pairing code**. Leave this screen open.
    2. In Bridgecoil, open **Add device**. Tap the search icon, labelled **Find nearby ADB devices**, and choose the row marked **Pairing port**. This fills in the address for you.
    3. Type the six digits from the TV into **Six-digit pairing code** and tap **Pair TV →**.
    4. Bridgecoil moves to **2 Connect**. Tap the search icon again and choose the row marked **Wireless connection**.
    5. Tap **Connect to TV →**.

    If the search finds nothing, type the addresses from the TV yourself. The pairing address is on the pairing screen, and the connection address is on the main Wireless debugging screen.

=== "Legacy ADB"

    1. In Bridgecoil, open **Add device**, then choose **2 Connect**.
    2. Enter the TV's address, for example `192.168.1.42:5555`.
    3. Tick **Allow legacy, unencrypted ADB**.
    4. Tap **Connect to TV →**.
    5. The TV shows a prompt asking to allow debugging. Tick the option to always allow, then choose **Allow** with your remote.

    Bridgecoil only offers its key to the TV when that box is ticked. Selecting a legacy TV from the search results doesn't tick it for you.

!!! screenshot "connect-add-device.png"
    Add device on step 1, with a pairing address and code filled in.

!!! screenshot "connect-nearby-devices.png"
    Nearby ADB devices showing a Pairing port row and a Wireless connection row.

!!! screenshot "connect-progress.png"
    The Connecting to TV progress screen with Cancel attempt.

When it works, you land on the [Device page](find-your-way-around.md) with a green **Connected** badge.

While it works, you can stop with **Cancel attempt**. **Help ?** explains where to find the debugging options on your TV.

## Next time

Open the app and tap **Quick connect →** next to your TV. Wireless debugging can change its port, so Quick connect looks up the current one first, then checks that it reached the same TV.

Your TV may ask you to approve the connection again if you reinstall Bridgecoil, because each installation has its own key.

## If it doesn't connect

The most common cause is using the pairing port to connect. See [The TV won't connect](../help/wont-connect.md) for every message and what to do about it.
