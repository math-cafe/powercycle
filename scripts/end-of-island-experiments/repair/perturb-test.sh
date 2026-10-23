#!/bin/bash
R=~/Projects/powercycle/tmp/runs/k4-n18193
for pair in "100 5000" "101 5001" "102 5002" "103 5003" "200 7000" "301 9001" "402 11002" "503 8003"; do
  read -r a b <<<"$pair"
  python3 - "$a" "$b" "$R/final.tour" <<'PY'
import sys
a,b,src=int(sys.argv[1]),int(sys.argv[2]),sys.argv[3]
lines=open(src).read().split("\n")
i=lines.index("TOUR_SECTION")+1; j=lines.index("-1")
body=lines[i:j]; body[a],body[b]=body[b],body[a]
open("swap.tour","w").write("\n".join(lines[:i]+body+lines[j:]))
PY
  out=$(./target/release/repair 4 18193 swap.tour 2 swap.pcdump.json 2>&1 | grep -E "defects|zone|2-factor|no degree|panicked" | tr '\n' ' ')
  echo "== $pair: ${out:0:220}"
done
