# Prints key=value facts about the TV. Read-only, and never starts rish: an unauthorized rish
# would put a prompt on the TV just for looking at this screen.
echo "shizukuServer=$(pidof shizuku_server)"
echo "porterServer=$(pidof porter_server)"
if [ -r /data/local/tmp/bridgecoil/listener.pid ]; then
  read -r listener < /data/local/tmp/bridgecoil/listener.pid
  case "$listener" in
    ""|*[!0-9]*) ;;
    *) kill -0 "$listener" 2>/dev/null && echo "listener=$listener" ;;
  esac
fi
# Android turns wireless debugging off on a wired TV, so its manager can't start itself.
case "$(ip -o -4 addr show 2>/dev/null)" in
  *" wlan"*) ;;
  *" eth"*) echo wired=1 ;;
esac
# Wireless debugging arrived in Android 11.
case "$(getprop ro.build.version.sdk)" in
  [1-9]|[12][0-9]) echo noWirelessDebugging=1 ;;
esac
# ADB left listening on the network is enough for a manager to start itself, on any Android.
case "$(getprop service.adb.tcp.port)$(getprop persist.adb.tcp.port)" in
  ""|0|-1|00) ;;
  *) echo networkAdb=1 ;;
esac
# Any Shizuku-type app answers this, whoever built it.
helper=$(cmd package query-activities --brief -a rikka.shizuku.intent.action.REQUEST_BINDER 2>/dev/null |
  sed -n 's/^ *\([A-Za-z0-9_.]*\)\/.*/\1/p' | head -n 1)
[ -n "$helper" ] && echo "helper=$helper"
[ -f /data/local/tmp/bridgecoil-use-helper ] && echo useHelper=1
# A manager only enables this receiver while its start on boot is on.
for manager in moe.shizuku.privileged.api:shizukuBoot eu.darken.porter:porterBoot "$helper:helperBoot"; do
  [ -n "${manager%:*}" ] || continue
  case "$(dumpsys package "${manager%:*}" 2>/dev/null | sed -n '/enabledComponents:/,/disabledComponents:/p')" in
    *BootCompleteReceiver*) echo "${manager#*:}=1" ;;
  esac
done
case "$(am broadcast -n moe.shizuku.privileged.api/moe.shizuku.manager.receiver.WatchdogStatusReceiver \
  -a moe.shizuku.privileged.api.WATCHDOG_STATUS 2>/dev/null)" in
  *'data="1"'*) echo watchdog=1 ;;
esac
run-as com.termux sh 2>/dev/null <<'INNER'
files=/data/data/com.termux/files
echo runAs=1
[ -x "$files/usr/bin/bash" ] && echo terminal=1
[ -x "$files/home/.bridgecoil/shell" ] && [ -s "$files/home/.bridgecoil/shell" ] && echo shell=1
[ -f "$files/home/.bridgecoil/authorized" ] && echo authorized=1
[ -f "$files/home/.termux/boot/00-bridgecoil" ] && echo monitor=1
[ -x "$files/home/.bridgecoil/agent" ] && echo agent=1
[ -f "$files/home/.bridgecoil/agent-ready" ] && echo agentReady=1
INNER
exit 0
