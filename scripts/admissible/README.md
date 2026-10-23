# The admissible set of the power-sum graphs

Programs for Theorems A and B of the paper *Hamiltonian cycles in power-sum
graphs* (Appendix C): `G_k(n)` has minimum degree at least two exactly on
the islands `[S_c, c^k - 1]`, for the even `c` with `Q <= c < q*`, and on the
mainland `[S_{q*}, infinity)`, where `S_c = (c+1)^k - c^k/2`.

- `powersum.gp` (PARI/GP) and `powersum.py` (Python): `Q(k)`, `qstar(k)`,
  `S(k, c)`, `A(k)` (the least admissible `n`), `mainland(k)`, `N(k)` (the
  number of islands) and `islands(k)`, in exact integer arithmetic.
- `check.py`: checks `powersum.py` against a direct computation of the
  minimum degree for every `n` up to twice the start of the mainland, for
  `k <= 6` (needs numpy; under 2 s), then against `powersum.gp` for
  `k <= 200`.

```console
python3 scripts/admissible/check.py
GP=/usr/bin/gp-2.19 python3 scripts/admissible/check.py
echo 'islands(8)' | gp -q -f scripts/admissible/powersum.gp
```
