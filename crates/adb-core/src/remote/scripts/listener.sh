#!/system/bin/sh
# Written by Bridgecoil, which can pause or remove it. Generated for this TV's own buttons.
dir=/data/local/tmp/bridgecoil
pid_file=$dir/listener.pid
events_file=$dir/events.pid
log=$dir/listener.log
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
    nohup sh "$0" --run >> "$log" 2>&1 < /dev/null &
    exit 0
    ;;
  --stop)
    # getevent goes first: ending it ends every part of the pipeline, on any Android version.
    if [ -r "$events_file" ]; then
      read -r events < "$events_file"
      case "$events" in ""|*[!0-9]*) ;; *) kill "$events" 2>/dev/null ;; esac
    fi
    running && kill "$pid"
    rm -f "$pid_file" "$events_file"
    rmdir "$dir/run.lock" 2>/dev/null
    exit 0
    ;;
  --run) ;;
  *) exit 2 ;;
esac
# The app and the boot monitor can both ask for a start, and two listeners would open apps twice.
if ! mkdir "$dir/run.lock" 2>/dev/null; then
  running && exit 0
fi
echo $$ > "$pid_file"
echo "$(date +%s) started"
set -f
paths=" "
path=
scan=
scan_device=
{
  getevent -l 2>/dev/null &
  echo $! > "$events_file"
  wait
} | while read -r line; do
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
        [ "$(wc -c < "$log" 2>/dev/null || echo 0)" -gt 100000 ] && : > "$log"
        (
          sleep 0.2
          case "$(am start -n "$target" 2>&1)" in
            *Error*|*Exception*) echo "$(date +%s) failed $target" ;;
            *) echo "$(date +%s) opened $target" ;;
          esac
        ) &
      fi
      ;;
  esac
done
rmdir "$dir/run.lock" 2>/dev/null
echo "$(date +%s) stopped"
