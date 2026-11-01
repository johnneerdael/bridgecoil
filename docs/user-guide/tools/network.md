# See your TV's network details

<p class="lead">A read-only view of your TV's addresses and routes.</p>

Open **Tools → Network**. It shows the output of `ip address` and `ip route`. Look for `wlan0` for Wi-Fi or `eth0` for a cable, and `inet` for the address.

![Network with its output box.](../assets/shots/network-page.png){ .bc-shot loading=lazy }

To test the connection, or change private DNS or Wi-Fi, use the Network group in [Commands](commands.md). "Direct forwarding is not available in this build." means Bridgecoil can't forward ports.
