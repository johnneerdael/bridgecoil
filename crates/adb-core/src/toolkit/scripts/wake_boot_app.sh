# Android keeps the boot event from an app that has never run. Only opening it clears that from a
# shell, because its receiver is private, so it opens once and closes as soon as it's on screen.
if dumpsys package com.termux.boot 2>/dev/null | grep -m1 'User 0:' | grep -q 'stopped=true'; then
  am start -n com.termux.boot/.BootActivity >/dev/null 2>&1
  for wait in 1 2 3 4 5 6 7 8 9 10; do
    if dumpsys activity activities 2>/dev/null | grep -m1 topResumedActivity | grep -q com.termux.boot; then
      input keyevent KEYCODE_BACK
      break
    fi
    sleep 0.5
  done
fi
