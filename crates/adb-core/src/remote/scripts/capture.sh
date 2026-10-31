@@KEY_EVENTS@@
since=$(date "+%m-%d %H:%M:%S.000")
case "$(getprop ro.build.version.sdk)" in
  [1-9]|[12][0-9])
    dir=/data/local/tmp/bridgecoil
    umask 077
    mkdir -p "$dir" || exit 0
    chmod 700 "$dir"
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
# Which apps the press opened, leaving out what this shell started itself.
logcat -d -T "$since" -v brief ActivityManager:I "*:S" 2>/dev/null | while read -r line; do
  case "$line" in
    *"from uid 2000"*) ;;
    *"START u0"*cmp=*)
      started=${line#*cmp=}
      echo "started ${started%%/*}"
      ;;
  esac
done
exit 0
