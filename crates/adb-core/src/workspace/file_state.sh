# Prints three NUL-terminated records per path in "$@": details, the path, its canonical form.
# Details are flags|type|size|modified|items. Flags are e (exists) and w (parent writable).
# The final link is not followed, because rm and mv act on the link itself.
for p in "$@"; do
  d=$(dirname "$p")
  w=-; [ -w "$d" ] && w=w
  if [ -e "$p" ] || [ -L "$p" ]; then
    n=0
    if [ -d "$p" ] && [ ! -L "$p" ]; then n=$(find "$p" -mindepth 1 2>/dev/null | wc -l); fi
    print -rn -- "e$w|$(stat -c '%F|%s|%Y' "$p" 2>/dev/null)|$n"
  else
    print -rn -- "-$w|||0|0"
  fi
  print -n '\0'; print -rn -- "$p"; print -n '\0'
  print -rn -- "$(readlink -f "$d")/$(basename "$p")"; print -n '\0'
done
exit 0
