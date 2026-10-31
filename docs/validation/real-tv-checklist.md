# Real TV checklist

This is item J of the take-back-control plan, the part that needs someone at a physical TV. Work down the list and note the first step that doesn't match. Later steps depend on earlier ones.

On a TV with a different app holding the Shizuku name, use Porter. Shizuku-Next can't install there.

## Connect

1. If an old Bridgecoil is on the phone, you can leave it or uninstall it. It has a different package ID, so the two install side by side, and the new one needs the TV paired again.
2. Install the APK from the Android release workflow's latest run, or from a release.
3. On the TV, unlock Developer options and turn on **Wireless debugging**. If it asks, allow it on this network. If it turns itself off, the TV is probably on a network cable. Android only allowed it on Wi-Fi in testing, so note this, unplug the cable and join Wi-Fi.
4. Pair with the code the TV shows, then connect.

**Expect:** the Device page opens and the status says Connected. A "Connected to your TV" service is running on the phone, though its notification only shows if notifications are allowed.

## TV setup

5. Open **Tools → TV setup** and install the helper app.
6. Start it, then install Termux and Termux:Boot.
7. If the TV shows **Unsafe app blocked**, use the real remote to choose **More details**, then **Install anyway**.
8. Connect Termux, then choose **Ask the TV**. On the TV choose **Allow all the time** for Porter or **Always allow** for Shizuku-Next.
9. Under **Survive restarts**, do all three steps. In the helper app, turn on start at boot and pair it with wireless debugging.

**Expect:** the setup ends at **Your TV is ready**.

## Buttons

10. Open **Tools → Buttons → Remap a button**, tap **Listen** and press a button on the real remote.
11. Map a tap to an app, then test it.
12. Map holding the same button to something else, then test both.

**Expect:** a tap and a hold each do their own thing, and the button's normal action still happens first. Note any button that never shows up, because that varies by TV.

## Restart

13. Turn the phone's Wi-Fi off and press the mapped button.
14. Unplug the TV for 30 seconds, so it's a cold boot and not standby. Plug it back in and wait about two minutes.
15. With the phone still away from the TV, press the mapped button.
16. Reconnect and open **Tools → Buttons**.

**Expect:** the button works in both step 13 and step 15. Buttons says it came back by itself after the TV restarted. Note if the TV is on Wi-Fi or Ethernet, since only that one is proven.

## Clean up and Home

17. Open **Tools → Clean up**. Check that apps you use aren't ticked, then turn off one app you don't use.
18. Press through the TV and confirm nothing important broke, then turn it back on.
19. If another launcher is installed, go through **Tools → Home app**. If it says **Your TV's own launcher comes first**, turn that one off through review. Press Home on the real remote.
20. Wait a minute, unplug the TV for 30 seconds and plug it back in. Check which launcher opens.
21. Turn the stock launcher back on, from History or Apps.

**Expect:** Clean up matches what's on the TV, and the way back works every time. Note if the TV has more than one stock launcher, because the card can then appear once for each.

## Afterwards

Mark item J in `docs/superpowers/plans/2026-10-31-take-back-control.md` only for what was actually seen, and record the TV model, Android version and network here.
