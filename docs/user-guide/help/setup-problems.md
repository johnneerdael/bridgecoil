# TV setup problems

<p class="lead">If [TV setup](../get-started/set-up-your-tv.md) stops, it's safe to leave and come back. It carries on from what's really on the TV.</p>

## Play Protect blocks Termux

This is expected. Termux is built for an older version of Android, so Play Protect warns about it. On the TV, open **More details** or **Details**, then **Install anyway**. Bridgecoil has already checked it's the real Termux. If it says "Google Play Protect stopped this install on the TV.", tap **Install Termux** again and do this on the TV.

## "A different Termux build is on this TV"

Termux from Google Play or F-Droid has a different signature. Bridgecoil can't set that one up. Remove it in Apps, which also deletes its files, so save anything you need from it first.

## "This TV blocks the access Bridgecoil needs"

Bridgecoil uses `run-as` to reach Termux's files, and this TV doesn't allow that. Mappings still work until the TV restarts.

## Prepare Termux doesn't finish

Termux needs a minute on first launch. Wait, then tap it again.

## The pairing code isn't accepted

"pairing failed: the code did not match" means the digits didn't match. Each time you open the screen you get a new code. "Bridgecoil can't find the TV's pairing screen." means it has closed, so reopen it. The screen closes itself after about ten minutes.

## Mappings don't come back after a restart

1. Wait two minutes. A slow TV can take this long.
2. Check that **Wireless debugging** is still on. It has to be on for the agent to connect, and some TVs turn it off.
3. Open **TV setup**. If it isn't **Your TV is ready**, finish it.
4. Check that Termux:Boot wasn't force stopped, for example by a cleaner app. Android treats a force stopped app as if it had never run, and it misses the boot event. Reopening setup doesn't fix this by itself: open Termux:Boot once on the TV.
5. Read `monitor.log` in `.bridgecoil`, using [Shell](../tools/shell.md):

    ```sh
    run-as com.termux cat files/home/.bridgecoil/monitor.log
    ```

## Setup says ready, and a new version of the app is out

Open TV setup after updating Bridgecoil. It replaces the agent on the TV if it has changed, and your pairing is kept.
