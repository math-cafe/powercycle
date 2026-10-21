#!/bin/bash
# Evidence for the conjecture on the mainland, paper 1 (2026-10-21).
set -u
P=~/Projects/power-sum-graphs/target/release/posum
export POSUM_LKH=~/Projects/power-sum-graphs/lkh/bin/LKH
cd ~/Projects/powercycle/tmp/continent
echo "start $(date -Is)"
$P sweep 4 18194 30000 --run-root k4 --time-limit 10m --nice 19 > k4.log 2>&1; echo "k4 sweep exit $? $(date -Is)"
$P sweep 3 801 10000 --run-root k3 --time-limit 10m --nice 19 > k3.log 2>&1; echo "k3 sweep exit $? $(date -Is)"
$P search 6 16777215 --run-dir k6/k6-n16777215 --time-limit 3h --nice 19 > k6-16777215.log 2>&1; echo "k6 16777215 exit $? $(date -Is)"
$P search 6 30039769 --run-dir k6/k6-n30039769 --time-limit 4h --nice 19 > k6-30039769.log 2>&1; echo "k6 30039769 exit $? $(date -Is)"
echo "done $(date -Is)"
