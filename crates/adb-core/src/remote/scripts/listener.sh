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
      while read -r events; do
        case "$events" in ""|*[!0-9]*) ;; *) kill "$events" 2>/dev/null ;; esac
      done < "$events_file"
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
# A listener that was stopped hard leaves its pipes behind.
rm -f "$dir"/events.[0-9]*
set -f
paths=" "
path=
scan=
scan_device=
held=
# Sets $pressed to when this key went down and forgets it, so a chord times each key by itself.
take() {
  pressed=
  case "$held" in
    *" $1="*)
      rest=${held#*" $1="}
      pressed=${rest%% *}
      held="${held%%" $1="*} ${rest#* }"
      ;;
  esac
}
@@KEY_EVENTS@@
{
  case "$(getprop ro.build.version.sdk)" in
    [1-9]|[12][0-9]) key_events ;;
    *)
      getevent -l 2>/dev/null &
      echo $! > "$events_file"
      wait
      ;;
  esac
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
      read -r now _ < /proc/uptime
      now=${now%.*}${now#*.}
      take "$device:$3"
      if [ "$4" = DOWN ]; then
        held="$held $device:$3=$now "
        continue
      fi
      [ "$4" = UP ] || continue
      gesture=tap
      [ -n "$pressed" ] && [ $((now - pressed)) -ge 60 ] && gesture=hold
      [ "$device" = "$scan_device" ] || scan=
      source=
      case "$paths" in
@@PATHS@@
      esac
      target=
      send=
      case "$gesture:$source:$scan:$3" in
@@KEYS@@
      esac
      scan=
      if [ -n "$target$send" ] && [ ! -f "$dir/paused" ]; then
        [ "$(wc -c < "$log" 2>/dev/null || echo 0)" -gt 100000 ] && : > "$log"
        (
          sleep 0.2
          if [ -n "$send" ]; then
            if input keyevent "$send" 2>/dev/null; then
              echo "$(date +%s) sent $send"
            else
              echo "$(date +%s) failed $send"
            fi
          else
            case "$(am start -n "$target" 2>&1)" in
              *Error*|*Exception*) echo "$(date +%s) failed $target" ;;
              *) echo "$(date +%s) opened $target" ;;
            esac
          fi
        ) &
      fi
      ;;
  esac
done
rmdir "$dir/run.lock" 2>/dev/null
echo "$(date +%s) stopped"
