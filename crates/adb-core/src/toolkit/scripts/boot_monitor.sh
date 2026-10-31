#!/data/data/com.termux/files/usr/bin/sh
# Written by Bridgecoil, which can remove it. Keeps button mappings running after a restart.
home=/data/data/com.termux/files/home/.bridgecoil
log=$home/monitor.log
mkdir "$home/monitor.lock" 2>/dev/null || {
  kill -0 "$(cat "$home/monitor.pid" 2>/dev/null)" 2>/dev/null && exit 0
}
echo $$ > "$home/monitor.pid"
command -v termux-wake-lock >/dev/null && termux-wake-lock
delay=30
while :; do
  [ "$(wc -c < "$log" 2>/dev/null || echo 0)" -gt 100000 ] && mv -f "$log" "$log.old"
  if timeout 20 "$home/shell" -c "sh /data/local/tmp/bridgecoil/listener.sh --start" >> "$log" 2>&1 </dev/null; then
    delay=30
  else
    echo "$(date): the manager is not reachable yet" >> "$log"
    delay=$((delay * 2))
    [ "$delay" -gt 600 ] && delay=600
  fi
  sleep "$delay"
done
