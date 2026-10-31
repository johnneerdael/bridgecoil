# Starts a manager server over ADB. "$1" is its package and "$2" its starter library. "$3" is the
# first part of the server process name. The whole name must stay out of this script: starters
# kill any process whose command line contains it, and that would include this one.
# The CPU folder is discovered, because it differs between TVs.
apk=$(pm path "$1" | head -n 1)
apk=${apk#package:}
for folder in "${apk%/*}"/lib/*; do
  if [ -x "$folder/$2" ]; then
    "$folder/$2" || exit $?
    i=0
    while [ $i -lt 20 ]; do
      pidof "$3"_server >/dev/null && exit 0
      i=$((i + 1))
      sleep 0.5
    done
    exit 1
  fi
done
echo "This manager's starter was not found." >&2
exit 1
