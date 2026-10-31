# Copies a manager shell into Termux and writes one wrapper, so everything else can stay the
# same for both managers. "$1" package, "$2" script asset, "$3" dex asset, "$4" app ID variable.
apk=$(pm path "$1" | head -n 1)
run-as com.termux sh -s "${apk#package:}" "$2" "$3" "$4" "$1" <<'INNER'
home=/data/data/com.termux/files/home/.bridgecoil
mkdir -p "$home" || exit 1
rm -f "$home"/rish "$home"/rish_shizuku.dex "$home"/porsh "$home"/porsh.dex "$home"/authorized
/data/data/com.termux/files/usr/bin/unzip -o -j -q "$1" "$2" "$3" -d "$home" || exit 1
chmod 700 "$home/${2##*/}"
chmod 400 "$home/${3##*/}"
printf '#!/system/bin/sh\nexport %s=com.termux MANAGER_APPLICATION_ID=%s\nexec sh %s "$@"\n' \
  "$4" "$5" "$home/${2##*/}" > "$home/shell"
chmod 700 "$home/shell"
INNER
