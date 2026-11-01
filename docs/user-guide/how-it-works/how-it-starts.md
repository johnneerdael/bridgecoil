# How it starts after a restart

<p class="lead">Nothing on your phone is involved. This is what happens on the TV.</p>

1. **Android finishes starting** and sends the boot event to apps.
2. **Termux:Boot receives it** and runs every script in `~/.termux/boot`, including `00-bridgecoil`.
3. **The boot script calls the agent**, which looks for the TV's wireless debugging port.
4. **The agent connects to the TV's own ADB** at `127.0.0.1`, using its key, and gets a shell.
5. **The shell runs `boot.sh`**, which starts `listener.sh`.
6. **Your buttons work.**

If ADB isn't ready at step 3, the script tries again every 3 seconds. Once it succeeds, it checks once a minute, which also restarts the listener if it ever stopped. Starting a listener that's already running does nothing.

## How long it takes

On a TCL Smart TV Pro with Android 14, mappings were back about 54 seconds after power on. The wait was for step 1. Your TV will differ, and a slow TV can take a couple of minutes. Standby doesn't restart anything, so buttons keep working when the TV wakes.

## Finding the port

Wireless debugging picks a new port each time it starts, and Termux can't read it directly. The agent asks the network for `_adb-tls-connect._tcp`, the same way your phone finds the TV, then checks that something is listening on that port. If there's no wireless debugging, it tries the classic ADB port, 5555.

## Why `boot.sh` is separate

Once `00-bridgecoil` is on a TV, Bridgecoil has no reason to touch it. The jobs live in `boot.sh` instead, which Bridgecoil can update over ADB when you save a mapping. A new version of the app can change what runs at boot without going through setup again.

## Checking that it worked

Buttons shows **Listening for buttons. It came back by itself after the TV restarted.** when the listener has outlived a restart. TV setup then says **Proven on this TV**.
