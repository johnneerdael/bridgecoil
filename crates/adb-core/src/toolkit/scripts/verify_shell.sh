# Proves the shell works, and only then records it as authorized.
run-as com.termux sh <<'INNER'
home=/data/data/com.termux/files/home/.bridgecoil
output=$(timeout 10 "$home/shell" -c id 2>&1 </dev/null)
case "$output" in
  *uid=0\(*) touch "$home/authorized"; echo route=root ;;
  *uid=2000\(*) touch "$home/authorized"; echo route=shell ;;
  *) rm -f "$home/authorized"; echo route=none ;;
esac
INNER
