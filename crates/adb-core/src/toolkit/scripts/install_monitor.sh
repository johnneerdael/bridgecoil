# Installs the boot script through run-as. The monitor text follows this script as stdin.
run-as com.termux sh -c 'mkdir -p /data/data/com.termux/files/home/.termux/boot && cat > /data/data/com.termux/files/home/.termux/boot/00-bridgecoil && chmod 700 /data/data/com.termux/files/home/.termux/boot/00-bridgecoil' <<'MONITOR'
