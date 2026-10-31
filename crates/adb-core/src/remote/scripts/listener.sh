#!/system/bin/sh
# Written by Bridgecoil, which can pause or remove it. Generated for this TV's own buttons.
dir=/data/local/tmp/bridgecoil
pid_file=$dir/listener.pid
running() {
  [ -r "$pid_file" ] || return 1
  read -r pid < "$pid_file"
  case "$pid" in ""|*[!0-9]*) return 1 ;; esac
  kill -0 "$pid" 2>/dev/null || return 1
  case "$(tr "\000" " " < "/proc/$pid/cmdline")" in
    *listener.sh*--run*) return 0 ;;
  esac
  return 1
}
case "$1" in
  --status) running; exit $? ;;
  --start)
    running && exit 0
    [ "$(wc -c < "$dir/listener.log" 2>/dev/null || echo 0)" -gt 100000 ] && mv -f "$dir/listener.log" "$dir/listener.log.old"
    nohup sh "$0" --run >> "$dir/listener.log" 2>&1 < /dev/null &
    exit 0
    ;;
  --stop)
    if running; then
      for child in $(pgrep -P "$pid"); do kill "$child" 2>/dev/null; done
      kill "$pid"
    fi
    rm -f "$pid_file"
    exit 0
    ;;
  --run) ;;
  *) exit 2 ;;
esac
echo $$ > "$pid_file"
echo "$(date +%s) started"
set -f
paths=" "
path=
scan=
scan_device=
getevent -l 2>/dev/null | while read -r line; do
  set -- $line
  case "$line" in
    "add device"*) path=$4; continue ;;
    name:*)
      label=${line#*\"}
      label=${label%\"*}
      case "$label" in
@@SOURCES@@
      esac
      continue
      ;;
    /dev/input/*) ;;
    *) continue ;;
  esac
  device=${1%:}
  case "$2" in
    EV_MSC)
      if [ "$3" = MSC_SCAN ]; then
        scan=$4
        scan_device=$device
      fi
      ;;
    EV_KEY)
      [ "$4" = UP ] || continue
      [ "$device" = "$scan_device" ] || scan=
      source=
      case "$paths" in
@@PATHS@@
      esac
      target=
      case "$source:$scan:$3" in
@@KEYS@@
      esac
      scan=
      if [ -n "$target" ] && [ ! -f "$dir/paused" ]; then
        echo "$(date +%s) $source:$3 $target"
        (sleep 0.2; am start -n "$target" >/dev/null 2>&1) &
      fi
      ;;
  esac
done
echo "$(date +%s) stopped"
