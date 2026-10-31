@@KEY_EVENTS@@
case "$(getprop ro.build.version.sdk)" in
  [1-9]|[12][0-9])
    dir=/data/local/tmp/bridgecoil
    mkdir -p "$dir" || exit 0
    events_file=$dir/capture.$$
    key_events &
    sleep @@SECONDS@@
    while read -r events; do
      kill "$events" 2>/dev/null
    done < "$events_file"
    wait
    rm -f "$events_file"
    ;;
  *) timeout @@SECONDS@@ getevent -l 2>/dev/null ;;
esac
exit 0
