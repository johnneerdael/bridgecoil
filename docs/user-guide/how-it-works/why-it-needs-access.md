# Why it needs this much access

<p class="lead">Android doesn't let an ordinary app see your remote's buttons or start other apps in the background. Everything below follows from that.</p>

## The problem

To remap a button, something on the TV has to watch for that button and react, every time, with no phone nearby. Three Android rules stand in the way:

1. An installed app can't read the remote's raw button events. Only the shell user can, and that's the user ADB gives you.
2. Nothing stays running through a restart unless the system starts it.
3. A phone that's off can't help.

So Bridgecoil needs a shell on the TV that starts by itself after a restart.

## How each piece solves part of that

**ADB** is Android's own management tool. It's what lets Bridgecoil on your phone change the TV at all.

**Termux** gives the agent a home. It's a well-known app that provides a Linux environment on Android. Bridgecoil only uses it as a place to keep and run its files, and you never need to open it.

**Termux:Boot** runs a script when the TV starts. This is what gets things going with no phone.

**The agent** is the small program that connects to the TV's own ADB, from inside the TV. The agent pairs with the TV the same way your phone did, and gets the same shell. It's written in Rust, and the [source is public](https://github.com/johnneerdael/bridgecoil/tree/main/crates/tv-agent).

**The listener** is a shell script run by that shell. It's the part that watches your buttons.

## What this access can do

The agent has the same power as ADB. That's the `shell` user (`uid=2000`), so anything holding that access can:

- Install and remove apps
- Turn apps on and off, and change their permissions
- Read shared storage, such as Downloads and Movies
- Read system logs, which can include what apps write there
- Take screenshots and press buttons
- Change many system settings

It can't read another app's private files, and it isn't root.

## The risk you're accepting

The agent's key is the part to think about. Termux keeps it in `agent.pk8`, and any program running inside Termux can read that file. If you only use Termux for Bridgecoil, nothing else is in there. If you install other software in Termux, that software could use the key to get the same shell.

Pairing adds an entry to the TV's list of allowed devices. Wireless debugging stays on, because the agent needs it. Other devices on your network still can't connect without pairing, which needs the code shown on your TV. On Android 10 and older, the connection isn't encrypted, so use that only on a network you trust.

## What Bridgecoil does to limit this

- The key stays on the TV. The agent connects only to the TV itself, on `127.0.0.1`.
- The agent and listener send nothing to the internet. See [What leaves your phone and TV](privacy.md).
- Termux and the agent are checked before they're installed. See [How downloads are checked](how-downloads-are-checked.md).
- You can [remove all of it](remove-everything.md).

## What you can do without setup

Most of Bridgecoil needs no setup at all. Apps, launchers, Clean up, Files, Screen capture, Diagnostics, Settings, Commands and Shell all work over the connection from your phone, and they install nothing except the temporary inspector. Setup is only needed for button mappings to survive a restart.

## Why Shizuku isn't required

Shizuku is a popular way to give apps shell access, and earlier versions of Bridgecoil used it. On some TVs it can't start by itself at boot, and brands can block the request that connects it, as a TCL did. The agent needs neither, so Shizuku is now optional. You can still [install it](../get-started/set-up-your-tv.md#shizuku-for-other-apps) if another app wants it.
