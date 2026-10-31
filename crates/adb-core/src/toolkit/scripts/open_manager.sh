# Opens a manager on the TV. "$1" is its package. TVs and phones list apps differently.
for category in LEANBACK_LAUNCHER LAUNCHER; do
  activity=$(cmd package resolve-activity --brief -a android.intent.action.MAIN \
    -c "android.intent.category.$category" "$1" | tail -n 1)
  case "$activity" in
    */*) exec am start -n "$activity" ;;
  esac
done
echo "This manager has no screen to open." >&2
exit 1
