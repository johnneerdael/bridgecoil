# Lists "$1" for the unprivileged shell user. Always exits 0: what could not be read is data.
# Output is NUL-separated: a status, then for readable folders whether new items can be
# created, one record per entry, ":meta", then stat lines.
# Names come from a glob because find drops entries SELinux hides from stat.
# Only shell built-ins run per entry, and [ -r ] also honours SELinux, unlike mode bits.
if cd "$1" 2>/dev/null; then
  if [ -r . ]; then s=ok; else s=denied; fi
elif [ -d "$1" ]; then s=denied
elif [ -e "$1" ]; then s=notDirectory
else s=missing
fi
print -rn -- "$s"; print -n '\0'
[ "$s" = ok ] || exit 0
w=-; [ -w . ] && w=w
print -rn -- "$w"; print -n '\0'
for f in * .[!.]* ..?*; do
  case "$f" in
    '*'|'.[!.]*'|'..?*') [ -e "$f" ] || [ -L "$f" ] || continue ;;
  esac
  k=o; [ -f "$f" ] && k=f; [ -d "$f" ] && k=d
  e=-; [ -e "$f" ] && e=e
  l=-; [ -L "$f" ] && l=l
  r=-; [ -r "$f" ] && r=r
  w=-; [ -w "$f" ] && w=w
  x=-; [ -x "$f" ] && x=x
  print -rn -- "$k$e$l$r$w$x$f"; print -n '\0'
done
print -rn -- ':meta'; print -n '\0'
find . -mindepth 1 -maxdepth 1 -exec stat -L -c 'BC|%s|%Y|%n' {} + 2>/dev/null
exit 0
