# How your buttons work

<p class="lead">Android keeps handling every button as normal. The listener watches alongside it and reacts to the ones you mapped.</p>

## What happens when you press a button

1. Your remote sends the press to the TV.
2. Android acts on it as it always does. If the button opens YouTube, YouTube starts.
3. The listener sees the same press as an input event. On Android 11 and later it reads `getevent -l`. On Android 10 and earlier it reads the input device directly, because `getevent` holds back its output there.
4. When you let go, the listener checks whether this button is mapped. It identifies a button by the remote it came from, its scan code and its key name.
5. If it's mapped, and mappings aren't paused, it waits 0.2 seconds, then runs `am start` for an app or `input keyevent` for another button.

The listener can't stop Android acting on a button, which is why your mapping follows the button's normal action. To get rid of that action, [turn off the app it opens](../buttons/replace-what-a-button-opens.md). A disabled app can't open, so the listener's action is all that's left.

## What the listener records

The listener sees every key on every input device, because that's how `getevent` works. It only writes down what it acted on, in lines such as `1793499 opened com.example/.Main`. It doesn't keep a record of other presses, and it sends nothing anywhere. You can read it with [Shell](../tools/shell.md): `cat /data/local/tmp/bridgecoil/listener.log`.

## How Bridgecoil finds your button

When you tap **Listen**, a short script reads input events for 8 seconds, and reads the system log from that moment to see which app the press opened. That's how it can offer to turn the app off. Bridgecoil ignores apps it started itself. This only sees the main profile.

## Press and hold

A press under 0.6 seconds counts as a tap. Holding it for 0.6 seconds or longer counts as a hold. If a button has only a press mapping, a long hold fires it too. Because the action fires on release, nothing happens until you let go.

## When an input device disappears

Android's `getevent` stops whenever any input device goes away, for example a Bluetooth remote going to sleep. The listener starts reading again a second later and keeps going until it's told to stop.
