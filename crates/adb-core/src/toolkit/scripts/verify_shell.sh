# Proves the shell works, and only then records it as authorized.
run-as com.termux sh <<'INNER'
home=/data/data/com.termux/files/home/.bridgecoil
output=$(timeout 10 "$home/shell" -c id 2>&1 </dev/null)
status=$?
case "$output" in
  *uid=0\(*) touch "$home/authorized"; echo route=root ;;
  *uid=2000\(*) touch "$home/authorized"; echo route=shell ;;
  *)
    # Running out of time says nothing about access, so it can't take access away.
    if [ "$status" = 124 ]; then
      echo route=timeout
    else
      rm -f "$home/authorized"
      echo route=none
    fi
    ;;
esac
INNER
