#!/system/bin/sh
# Written by Bridgecoil, which can replace it at any time. Termux's boot monitor runs this as the
# shell user after every restart and then once a minute, so every job has to be safe to repeat.
dir=/data/local/tmp/bridgecoil
[ ! -f "$dir/listener.sh" ] || sh "$dir/listener.sh" --start
