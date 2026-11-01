# Set up your TV

<p class="lead">TV setup puts what's needed on your TV so that button mappings keep working after a restart. It takes about ten minutes.</p>

**You need:**

- [ ] Bridgecoil [connected to your TV](first-connection.md)
- [ ] Your remote nearby, because the TV will ask you things
- [ ] The TV switched on and on your network

You only need setup for button mappings that survive a restart. Everything else in Bridgecoil works without it. See [What Bridgecoil installs on your TV](../how-it-works/what-gets-installed.md) before you start, so that you know what you're agreeing to.

Open **Tools → TV setup**. You'll see three steps: **Set up Termux**, **Connect to the TV** and **Survive restarts**. The app moves through them by checking what's really on the TV, so you can leave and come back at any point.

!!! screenshot "setup-step-1.png"
    TV setup on Set up Termux, with Install Termux showing and the Play Protect note.

## Step 1: set up Termux

1. Tap **Install Termux**. Bridgecoil downloads it from GitHub, [checks it](../how-it-works/how-downloads-are-checked.md) and installs it.
2. Look at the TV. Google Play Protect will probably stop it with **Unsafe app blocked** or **Blocked by Play Protect**, because Termux is built for an older version of Android. With your remote, open **More details** or **Details**, then choose **Install anyway**.
3. Tap **Install Termux:Boot** and do the same.
4. Tap **Prepare Termux**, review it and apply. Termux opens on the TV once to finish setting itself up. If this step doesn't finish, give it a minute and try again.

!!! screenshot "setup-play-protect.png"
    On the TV: Play Protect's warning with More details visible.

## Step 2: connect to the TV

Bridgecoil puts its agent into Termux by itself, and you'll see **Getting Termux ready…** for a moment. What happens next depends on your TV.

=== "Android 11 and later"

    1. On the TV, open **Wireless debugging** in Developer options, and choose **Pair device with pairing code**.
    2. In Bridgecoil, type the six digits and tap **Connect**.

    This pairs the agent, which is separate from pairing your phone. The TV's pairing screen stays open only for a limited time, so reopen it if it closes.

=== "Android 10 and earlier"

    1. The TV asks to allow a connection. Choose **Always allow** with your remote.
    2. Bridgecoil moves on by itself. You'll see **Waiting for the TV. This moves on by itself.**

!!! screenshot "setup-pair-code.png"
    Connect to the TV with the Pairing code field and the 2 second note.

Your TV may show a Termux:Boot page for about 2 seconds afterwards. That's normal, and it closes by itself. Android doesn't send its boot event to an app that has never been opened, so Bridgecoil opens it once. If it has already run, you won't see this.

## Step 3: survive restarts

Normally this finishes by itself during step 2, and you go straight to **Your TV is ready**. If you see **Termux is connected. One step left.**, tap **Keep mappings running**.

!!! screenshot "setup-ready.png"
    Your TV is ready, with Remap buttons and the Shizuku section.

## Prove it works

**Your TV is ready** starts with "Recovery after a restart hasn't been proven on this TV yet." [Map a button](../buttons/remap-a-button.md), restart the TV, wait a minute or two, then open **Buttons**. Once your mapping has come back by itself, the page says **Proven on this TV**.

## Shizuku, for other apps

Bridgecoil doesn't need Shizuku. If another app asks for it, open **Shizuku, for other apps** on the ready screen and choose **Install Shizuku-Next** or **Install Porter**. **Start {name}** starts it until the next restart.

## If something goes wrong

| Message | What to do |
|---|---|
| "A different Termux build is on this TV, and Bridgecoil can't set that one up." | Remove it in Apps, which also deletes its files. This usually means it came from Google Play or F-Droid |
| "This TV blocks the access Bridgecoil needs to set up Termux" | This TV won't allow it. Mappings still work until the TV restarts |
| "Bridgecoil can't find the TV's pairing screen. Check that it's still open." | Reopen the pairing screen on the TV and try again |
| "Paired, but Termux can't reach the TV yet." | Reopen TV setup to try again |
| "The TV didn't allow this. Reopen TV setup to try again." | The prompt timed out or was refused |
| "Bridgecoil has no agent for this TV's processor." | Your TV isn't one of the three processor types Bridgecoil builds for |

See [TV setup problems](../help/setup-problems.md) for more.

## Undo

**Remove the startup script** removes `00-bridgecoil`, so mappings stop after the next restart. [Remove everything](../how-it-works/remove-everything.md) takes the rest off.
