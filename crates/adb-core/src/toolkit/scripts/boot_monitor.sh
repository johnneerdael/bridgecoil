#!/data/data/com.termux/files/usr/bin/sh
# Written by Bridgecoil, which can remove it. Brings button mappings back after a restart.
home=/data/data/com.termux/files/home/.bridgecoil
log=$home/monitor.log
if [ -r "$home/monitor.pid" ]; then
  running=$(cat "$home/monitor.pid")
  case "$running" in
    ""|*[!0-9]*) ;;
    *)
      case "$(tr "\000" " " < "/proc/$running/cmdline" 2>/dev/null)" in
        *00-bridgecoil*) exit 0 ;;
      esac
      ;;
  esac
fi
echo $$ > "$home/monitor.pid"
listener=/data/local/tmp/bridgecoil/listener.sh
# Wireless ADB can take a couple of minutes after a restart, so keep asking until it answers.
# After that this is the watchdog: starting a listener that's already running does nothing.
while :; do
  [ "$(wc -c < "$log" 2>/dev/null || echo 0)" -gt 100000 ] && : > "$log"
  if "$home/agent" run "[ ! -f $listener ] || sh $listener --start" >> "$log" 2>&1; then
    sleep 60
  else
    sleep 3
  fi
done
