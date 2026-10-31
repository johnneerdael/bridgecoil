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
9. Under **Survive restarts**, do all three steps. In the helper app, open Settings and turn on **Start on boot**. On Android 11 or later, also pair it with wireless debugging.

**Expect:** the setup ends at **Your TV is ready**, with no warning that start on boot is off. The helper app shows no battery warning, and Shizuku-Next's watchdog is on.

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

## Runs on a real TV

**Ugoos AM6, Android 9 (`userdebug`, rooted with Magisk), 32-bit, on Ethernet, with network ADB on port 5555. 2026-10-31.** The phone was a CPH2747 on Android 16, with the signed app.

| Step | Result |
|---|---|
| Find, then connect with legacy ADB | Passed, with the real approval prompt on the TV |
| Pair with a code | Not possible here. Android 9 has no wireless debugging |
| Helper app, recognised by certificate, started | Passed with Shizuku-Next, after fixing the inspector |
| Termux and Termux:Boot | Passed. Play Protect blocked only Termux:Boot, as "Blocked by Play Protect" |
| Access prompt | Passed, `uid=2000(shell)` |
| Survive restarts, ending at **Your TV is ready** | Passed, once start on boot and the watchdog are on |
| Listen hears a real press | Passed, after replacing `getevent` on older Android |
| A mapped button opens its app | Passed, Search and an unassigned button |
| Cold boot, with the phone not connected | Passed. The helper started itself and the listener was back about 62 seconds after power-on, then a real press opened the mapped app |

This run found what no emulator could:

- `Context.createContextAsUser` doesn't exist on Android 9, so app details couldn't be read and setup stalled.
- `getevent` holds its output until 8 KB has built up when it isn't on a terminal, so Listen heard nothing and a mapping would have been late.
- Setup called the TV ready with start on boot off and the watchdog never set, and didn't exempt the helper from battery limits.
- A TV on network ADB was told it had no wireless debugging. A helper can start itself over that port, on any Android version, without Wi-Fi.
- Pages rebuild after each change, which wiped the offer to turn off what a button opens.

A button keeps its normal action until the app it opens is off. A button Android has no key code for, shown as `keyCode=0`, has no normal action, so a mapping is already a full replacement.

**TCL Smart TV Pro, Android 14, 32-bit, on Wi-Fi. 2026-10-31.** This TV already runs the owner's own remote script, Termux and Shevery, a Shizuku-type helper Bridgecoil can't verify.

| Step | Result |
|---|---|
| Pair with a code, then connect | Passed. The device page showed Wireless ADB · TLS, and it reconnected after an app update |
| Find the helper already on the TV | Passed, Shevery was offered with the warning, and no Porter was needed |
| Termux gets the helper's shell tool | Passed, `MANAGER_APPLICATION_ID=com.hamondev.shevery` |
| Access prompt | Passed once Shevery was open, `uid=2000(shell)` |
| Listen, then a mapped button opens its app | Passed, a real press opened Videoland, next to the owner's own listener |
| **Stop** on a listener that restarts itself | Passed, it stayed down, then started again |

This run found:

- TCL's firmware drops the helper's request while its app is closed (`TclAppBoot: Skipping delivery`), so **Ask the TV** seemed to do nothing. It now opens the helper first.
- `getevent` ends when any input device goes away, which stopped the owner's listener for good. Bridgecoil's listener now restarts its own event stream.
- A pressed button that needed **Check access** to continue was unclear, so setup now watches for the answer itself.

Not proven on any TV yet: a full remap with **Turn off** on a button that opens an app, the listener restarting after a real device dropout, start at boot through wireless debugging, Clean up and the launcher handover.

## Afterwards

Mark item J in `docs/superpowers/plans/2026-10-31-take-back-control.md` only for what was actually seen, and record the TV model, Android version and network here.
