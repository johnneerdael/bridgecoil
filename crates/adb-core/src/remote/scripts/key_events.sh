# Android 10 and older hold back getevent's output until about 8 KB has built up, so a press
# wouldn't arrive in time. od passes each event on at once. This prints what getevent -l does,
# with the key as a number, since nothing here can name it. Needs $dir and $events_file.
key_events() {
  set +f
  case "$(getprop ro.product.cpu.abi)" in
    *64*) width=24 clock="a b c d e f g h i j k l m n o p" ;;
    *) width=16 clock="a b c d e f g h" ;;
  esac
  : > "$events_file"
  number=0
  for device in /dev/input/event*; do
    number=$((number + 1))
    node=/sys/class/input/${device##*/}/device
    echo "add device $number: $device"
    echo "  name:     \"$(cat "$node/name")\""
    # A touch surface has no buttons and is far busier than everything else.
    [ "$(cat "$node/capabilities/key")" = 0 ] && continue
    pipe=$dir/events.$$.$number
    mkfifo "$pipe" || continue
    od -A n -t x1 -w"$width" -v "$device" > "$pipe" &
    echo $! >> "$events_file"
    while read -r $clock type0 type1 code0 code1 value rest; do
      [ "$type1$type0" = 0001 ] || continue
      case "$value" in
        01) echo "$device: EV_KEY $code1$code0 DOWN" ;;
        00) echo "$device: EV_KEY $code1$code0 UP" ;;
      esac
    done < "$pipe" &
  done
  wait
  rm -f "$dir"/events.$$.*
}
