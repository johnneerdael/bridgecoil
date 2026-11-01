# What Bridgecoil installs on your TV

<p class="lead">This is the full list. If you've finished TV setup and mapped a button, this is everything on your TV that came from Bridgecoil.</p>

Connecting to a TV installs nothing. Each item below is added by something you start: [TV setup](../get-started/set-up-your-tv.md), a button mapping, or a tool. Apps on the list can do a lot, so the [next page](why-it-needs-access.md) explains why each is needed and what the risk is.

## Apps

TV setup installs two apps:

| App | Why |
|---|---|
| Termux | Gives Bridgecoil's agent somewhere to live on the TV |
| Termux:Boot | Starts a script when the TV restarts |

Their package names are `com.termux` and `com.termux.boot`. Both are unmodified releases from their own developers, [termux-app](https://github.com/termux/termux-app) and [termux-boot](https://github.com/termux/termux-boot), downloaded straight from GitHub and [checked against a pinned signature](how-downloads-are-checked.md).

Installed only if you ask:

| App | Why you'd want it |
|---|---|
| Shizuku-Next or Porter | Another app needs Shizuku. Bridgecoil doesn't use it |
| Backdrop, Projectivy, Monet or AT4K | You chose [a new home screen](../home-screen/install-a-launcher.md) |

## Files in Termux

These live in Termux's private storage, under `/data/data/com.termux/files/home`, and belong to the Termux app.

| File | Size | What it is |
|---|---|---|
| `.bridgecoil/agent` | 1 to 2 MB | The Bridgecoil agent, a small program built for your TV's processor |
| `.bridgecoil/agent.pk8` | About 1 KB | The agent's private key. Only Termux can read it (mode 600) |
| `.bridgecoil/agent-ready` | Empty | A marker that the agent proved it has shell access |
| `.bridgecoil/monitor.log` | Up to 100 KB | Errors from the boot script |
| `.bridgecoil/monitor.pid` | Tiny | Stops two copies of the boot script running |
| `.termux/boot/00-bridgecoil` | About 1 KB | The script Termux:Boot runs at startup |

## Files in the shell folder

These live in `/data/local/tmp/bridgecoil`, which only the ADB shell user can open (mode 700).

| File | What it is |
|---|---|
| `boot.sh` | The list of jobs to run at boot and once a minute. Today that's starting the listener |
| `listener.sh` | The script that watches for your mapped buttons. It's generated for your TV's buttons |
| `mappings.json` | Your saved mappings |
| `listener.log` | What the listener opened or sent, with timestamps. Up to 100 KB |
| `listener.pid`, `events.pid`, `run.lock`, `saved_boot` | Housekeeping |
| `paused` | Exists only while mappings are paused |

## Temporary files

| File | When | Cleaned up |
|---|---|---|
| `/data/local/tmp/bridgecoil-inspector-{hash}.jar` | When Bridgecoil reads app names and icons | No. It stays, and a new version adds another. [Remove it by hand](remove-everything.md) |
| `/data/local/tmp/bridgecoil-recording.mp4` | While recording the screen | Once it's copied to your phone, or when you discard it |
| `/data/local/tmp/adb-studio-{id}.apk` | During an install | Yes, after the install |

The inspector is a small Java program that Android runs with `app_process` to read app labels, icons and signatures, which the shell can't get otherwise. It runs and exits, so it isn't a background service, and it's used even on a TV where you never ran setup.

## What runs in the background

After setup and at least one mapping, two things keep running:

- The boot script `00-bridgecoil`, as the Termux app. It mostly sleeps, and once a minute it runs `boot.sh` through the agent.
- The listener, `listener.sh`, as the ADB shell user. It waits for key events, and uses almost nothing while you're not pressing buttons.

## Settings changed on your TV

When the agent proves it has access, Bridgecoil makes these changes without a separate review, because mappings can't survive a restart without them:

| Change | Command | Why |
|---|---|---|
| Allow Termux and Termux:Boot to run in the background | `cmd appops set … RUN_IN_BACKGROUND allow`, `RUN_ANY_IN_BACKGROUND allow`, `AUTO_START allow` | So Android doesn't stop them |
| Exempt them from battery saving | `dumpsys deviceidle whitelist +com.termux` | Same reason |
| Open Termux:Boot once, only if it has never run | `am start -n com.termux.boot/.BootActivity`, then Back | Android holds back the boot event from an app that has never been opened |
| Write `boot.sh` and `00-bridgecoil` | | Listed above |

The Termux:Boot page may flash on the TV for about two seconds. A TV without some of these settings skips them.

[What skips the review](../history/reviews.md#what-skips-the-review) lists these along with everything else that runs without one.

## On your phone

| What | Where |
|---|---|
| Bridgecoil's ADB key | `identity/host.pk8` in the app's private data |
| Operation history | `history.sqlite3` in the app's private data |
| Ratings list, if you downloaded it | `ratings/` in the app's private data |
| Saved TVs, pairings, theme, saved commands | The app's own local storage |
