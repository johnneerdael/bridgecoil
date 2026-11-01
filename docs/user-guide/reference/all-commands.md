# All commands

<p class="lead">Every command in Tools → Commands, with what it runs. `{app}` is the app you pick and `{profile}` is the selected profile.</p>

## Apps

| Command | Badge | Options | Runs |
|---|---|---|---|
| **List apps** | Only reads | All, Installed by you, Preinstalled, Disabled | `pm list packages` with `-3`, `-s` or `-d` |
| **Show an app's version** | Only reads | App | `dumpsys package {app} \| grep -m2 -E 'versionName\|versionCode'` |
| **Show an app's permissions** | Only reads | App | `dumpsys package {app} \| grep -E 'permission\.'` |
| **Find an app's files** | Only reads | App | `pm path {app}` |
| **Force stop an app** | Changes the TV | App | `am force-stop {app}` |
| **Switch an app off** | Be careful | App | `pm disable-user --user {profile} {app}` |
| **Switch an app back on** | Changes the TV | App | `pm enable --user {profile} {app}` |
| **Delete an app's data** | Be careful | App | `pm clear --user {profile} {app}` |
| **Remove an app** | Be careful | App | `pm uninstall --user {profile} {app}` |

Switching off Settings or your launcher can leave you unable to get back in without this app. Removing a system app can break the TV.

## Display

| Command | Badge | Options | Runs |
|---|---|---|---|
| **Show the screen size** | Only reads | | `wm size` |
| **Show the text and icon size** | Only reads | | `wm density` |
| **Change text and icon size** | Changes the TV | 160, 213, 240, 320 | `wm density {density}` |
| **Reset text and icon size** | Changes the TV | | `wm density reset` |
| **Change animation speed** | Changes the TV | Off, Fast, Normal | `settings put global` for the three animation scales |
| **Turn the screensaver on or off** | Changes the TV | On, Off | `settings put secure screensaver_enabled {state}` |

Higher density numbers look bigger. Some apps may look wrong at 320.

## Input

| Command | Badge | Options | Runs |
|---|---|---|---|
| **Press a key** | Changes the TV | Home, Back, Menu, Settings, Change input, Play or pause, Mute | `input keyevent {KEYCODE}` |
| **List input devices** | Only reads | | `getevent -il \| grep -E 'add device\|name:'` |
| **List keyboards** | Only reads | | `ime list -s` |

## Network

| Command | Badge | Options | Runs |
|---|---|---|---|
| **Show network addresses** | Only reads | | `ip -o addr show` |
| **Show Wi-Fi status** | Only reads | | `dumpsys wifi \| grep -m1 mWifiInfo` |
| **Show private DNS** | Only reads | | `settings get global private_dns_mode`, then the specifier |
| **Test the internet connection** | Only reads | Cloudflare, Google, A website name | `ping -c 3 {host}` |
| **Block ads with private DNS** | Changes the TV | AdGuard, Cloudflare, Off | `settings put global private_dns_mode hostname`, then the specifier, or `off` |
| **Turn Wi-Fi on or off** | Be careful | On, Off | `svc wifi {state}` |

**Block ads with private DNS** sends the TV's name lookups to the service you pick: AdGuard blocks ads and trackers, and the Cloudflare option blocks malware. Turning Wi-Fi off can disconnect your phone, and you may need the remote to turn it back on.

## Power

| Command | Badge | Options | Runs |
|---|---|---|---|
| **Put the TV to sleep** | Changes the TV | | `input keyevent KEYCODE_SLEEP` |
| **Keep the TV awake** | Changes the TV | On, Off | `svc power stayon {state}` |
| **Restart the TV** | Be careful | | `reboot` |
| **Shut down the TV** | Be careful | | `reboot -p` |

After a shutdown you'll need the remote or the TV's button to turn it back on.

## Settings

| Command | Badge | Options | Runs |
|---|---|---|---|
| **List settings** | Only reads | System, Secure, Global | `settings list {group}` |
| **Stop automatic system updates** | Changes the TV | Stop them, Allow them | `settings put global ota_disable_automatic_update {state}` |
| **Show touches for debugging** | Changes the TV | On, Off | `settings put system show_touches {state}` |

Some TVs ignore the setting for automatic updates.

## System

| Command | Badge | Runs |
|---|---|---|
| **How long the TV has been on** | Only reads | `uptime` |
| **Show the Android version** | Only reads | `getprop ro.build.version.release` |
| **Show the model** | Only reads | `getprop ro.product.model` |
| **Show the processor type** | Only reads | `getprop ro.product.cpu.abilist` |
| **Show free storage** | Only reads | `df -h /data` |
| **Show memory use** | Only reads | `head -n 5 /proc/meminfo` |
| **Show what's using the processor** | Only reads | `top -b -n 1 -m 12` |
| **Show temperature** | Only reads | `dumpsys thermalservice \| head -n 30` |

## Logs

| Command | Badge | Options | Runs |
|---|---|---|---|
| **Read recent logs** | Only reads | Errors only, Warnings and errors, Everything useful. 100 or 500 lines | `logcat -d -t {count} '{level}'` |
| **Read crash reports** | Only reads | | `logcat -d -b crash -t 100` |
| **Why the TV last restarted** | Only reads | | `getprop sys.boot.reason` |
