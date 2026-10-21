#!/bin/sh
# check-lee.sh GSQ: every N = 49m + 24 with m even, 2000 < N <= 10^6, the step
# n -> 49n + 24 (n even) whose table entries were questioned in 2024.
G=$1; H=$(dirname "$0")
d=$(mktemp -d); bad=0; tot=0
m=42; while [ $((49*m+24)) -le 1000000 ]; do
  N=$((49*m+24))
  "$G" -n "$N" -out "$d/o.txt" >/dev/null 2>&1
  r=$("$H/chk" "$N" "$d/o.txt"); tot=$((tot+1))
  [ "$r" = OK ] || { bad=$((bad+1)); echo "N=$N: $r"; }
  m=$((m+2)); done
echo "N = 49m + 24, m even, up to 10^6: $tot checked, $bad bad"; rm -rf "$d"
