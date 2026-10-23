#!/bin/sh
f=$1
top=${f%%/*}; k=${top#k}; k=${k%%-*}
sub=$(dirname "$f"); last=${sub##*/}
case "$last" in n[0-9]*) n=${last#n};; *) n=${top#k*-n}; n=${n%%-*};; esac
out=$(~/Projects/powercycle/scripts/verify-bases.sh "$k" "$n" "$f")
pre=$(basename "$f" .bases.xz)
case "$out" in "VALID k=$k n=$n $pre"*) echo OK;; *) echo "FAIL $f: $out";; esac
