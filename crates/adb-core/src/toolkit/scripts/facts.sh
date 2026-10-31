# Prints key=value facts about the TV. Read-only, and never starts rish: an unauthorized rish
# would put a prompt on the TV just for looking at this screen.
echo "shizukuServer=$(pidof shizuku_server)"
echo "porterServer=$(pidof porter_server)"
if [ -r /data/local/tmp/bridgecoil/listener.pid ]; then
  read -r listener < /data/local/tmp/bridgecoil/listener.pid
  kill -0 "$listener" 2>/dev/null && echo "listener=$listener"
fi
run-as com.termux sh 2>/dev/null <<'INNER'
files=/data/data/com.termux/files
echo runAs=1
[ -x "$files/usr/bin/bash" ] && echo terminal=1
[ -x "$files/home/.bridgecoil/shell" ] && [ -s "$files/home/.bridgecoil/shell" ] && echo shell=1
[ -f "$files/home/.bridgecoil/authorized" ] && echo authorized=1
[ -f "$files/home/.termux/boot/00-bridgecoil" ] && echo monitor=1
INNER
exit 0
