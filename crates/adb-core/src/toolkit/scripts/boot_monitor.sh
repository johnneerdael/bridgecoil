#!/data/data/com.termux/files/usr/bin/sh
# Written by Bridgecoil, which can remove it. Keeps button mappings running after a restart.
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
delay=30
refused=0
while :; do
  [ "$(wc -c < "$log" 2>/dev/null || echo 0)" -gt 100000 ] && : > "$log"
  output=$(timeout 20 "$home/shell" -c "sh /data/local/tmp/bridgecoil/listener.sh --start" 2>&1 </dev/null)
  status=$?
  case "$status:$output" in
    0:*)
      delay=30
      refused=0
      ;;
    *"not running"*)
      # The manager hasn't started yet. Waiting for it shows nothing on the TV.
      delay=$((delay * 2))
      [ "$delay" -gt 600 ] && delay=600
      ;;
    *)
      # Anything else may be a prompt on the TV, so give up until the next restart.
      refused=$((refused + 1))
      echo "$(date): the manager refused this ($refused of 3): $output" >> "$log"
      [ "$refused" -ge 3 ] && exit 0
      delay=$((delay * 2))
      ;;
  esac
  sleep "$delay"
done
