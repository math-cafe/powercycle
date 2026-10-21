#!/bin/sh
# check-range.sh GSQ A B STEP: run Gerbicz's program GSQ for n = A, A+STEP, ..., B
# and check each output with chk (Hamiltonian cycle of G_2(n), exact).
G=$1; shift
H=$(dirname "$0")
d=$(mktemp -d); bad=0; tot=0
n=$1; while [ "$n" -le "$2" ]; do
  "$G" -n "$n" -out "$d/o.txt" >/dev/null 2>&1
  r=$("$H/chk" "$n" "$d/o.txt"); tot=$((tot+1))
  [ "$r" = OK ] || { bad=$((bad+1)); echo "n=$n: $r"; }
  n=$((n+$3)); done
echo "n = $1..$2 step $3: $tot checked, $bad bad"; rm -rf "$d"
