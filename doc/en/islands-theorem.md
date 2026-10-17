# The admissible islands, in closed form

[`islands.md`](islands.md) describes the admissible islands of `G_k(n)` and
states, from measurements, where they begin, when they exist and when they
end. [`least-admissible.md`](least-admissible.md) proves where the first one
begins. This note proves the whole description, for every `k >= 2`.

---

## 1. Statement

`I_k` is the set of `n` for which every vertex of `G_k(n)` has degree at least
two; its maximal intervals are the **admissible islands**. Let

```text
t_1 = 1 / ((3/2)^(1/k) - 1)        t_2 = 2 / (2^(1/k) - 1)
```

so that for real `q > 0`

```text
(1 + 1/q)^k < 3/2   <=>   q > t_1          (1 + 2/q)^k < 2   <=>   q > t_2
```

and, for an even `q`, let `S_q = (q + 1)^k - q^k/2`.

> **Theorem.** Let `k >= 2`, let `Q` be the least even integer above `t_1` and
> `q*` the least even integer above `t_2`. Then `Q <= q*` and
>
> ```text
> I_k  =  [S_Q, Q^k - 1]  U  [S_{Q+2}, (Q+2)^k - 1]  U ...  U  [S_{q*-2}, (q*-2)^k - 1]  U  [S_{q*}, infinity)
> ```
>
> the union running over the even `q` with `Q <= q < q*`, all intervals
> non-empty and pairwise separated. In particular:
>
> - `G_k(n)` has **`(q* - Q)/2` finite islands**, one for each even `q` with
>   `t_1 < q < t_2`, the island of `q` being `[(q+1)^k - q^k/2, q^k - 1]`;
> - the **last island** is unbounded and starts at `(q* + 1)^k - q*^k/2`;
> - the **first island** starts at `(Q + 1)^k - Q^k/2`, the theorem of
>   [`least-admissible.md`](least-admissible.md).

This is the table of `islands.md` section 3, and its two conditions of
section 4: the island of an even `q` exists when `(1 + 1/q)^k < 3/2`, and is
bounded above, at `q^k - 1`, when `(1 + 2/q)^k > 2`. Neither condition is ever
an equality, since `2(q+1)^k = 3q^k` and `(q+2)^k = 2q^k` have no solution in
positive integers for `k >= 2`.

**The number of finite islands.** Expanding `1/(e^x - 1) = 1/x - 1/2 + x/12 - ...`,

```text
t_1 = k / ln(3/2) - 1/2 + O(1/k) = 2.4663 k - 0.5 + O(1/k)
t_2 = 2k / ln 2   - 1   + O(1/k) = 2.8854 k - 1   + O(1/k)
```

so there are `(1/ln 2 - 1/(2 ln(3/2))) k + O(1) = 0.2095 k + O(1)` finite
islands, one per even integer strictly between `t_1` and `t_2`: none for
`k = 2, 3`, one for `k = 4..7`, two for `k = 8, 9`.

---

## 2. Notation and earlier results

As in [`least-admissible.md`](least-admissible.md): a power is **usable** for
`v` when it exceeds `v` and differs from `2v`; `g(v) = p_2(v) - v`, `p_2(v)`
being the second usable power above `v`; `v` **blocks** `m` when
`v <= m < g(v)`, and `m` is admissible exactly when nothing blocks it
(Lemma 1). By Lemma 2, every blocked `m` is blocked by a power `a^k`, with
`g(a^k) = (a + 2)^k - a^k`, or by a loop position `L_c = c^k/2`, `c` even, in
one of two cases:

- `(c - 1)^k <= L_c < c^k`, case **near**: `g(L_c) = (c + 2)^k - c^k/2`;
- `(c - 2)^k <= L_c < (c - 1)^k`, case **far**: `g(L_c) = (c + 1)^k - c^k/2 = S_c`.

A loop position below `(c - 2)^k` blocks nothing that the power below it does
not already block. Write `phi(t) = (1 + 1/t)^k` and `psi(t) = (1 + 2/t)^k`, both
decreasing. Used again from `least-admissible.md`, for every `k >= 2`:

- `Q >= 6`;
- **(ii)** `psi(Q - 2) >= 2`;
- **(iii)** `phi(Q - 1) < 2`;
- every `m < S_Q` is blocked, by powers `a^k` and loop positions `L_c` with
  `c <= Q`, and none of these blocks any `m >= S_Q` (sections 5 and 6 there:
  their blocking intervals all end at `S_Q - 1` or before).

---

## 3. Three lemmas

**Lemma 3.** `t_1 < t_2` for `k >= 2`; hence `Q <= q*`.

*Proof.* With `x = 1/k`, `t_2 > t_1` amounts to `F(x) = 2 (3/2)^x - 2^x > 1`.
`F(0) = F(1) = 1`, and `F''(x) = 2 (3/2)^x ln^2(3/2) - 2^x ln^2 2 < 0` on
`[0, 1]`, since `2 ln^2(3/2) / ln^2 2 = 0.68 < 1 <= (4/3)^x`. A concave function
equal to 1 at both ends exceeds 1 inside, and `x <= 1/2 < 1`. ∎

**Lemma 4 (loop positions from `Q` on).** For even `c` with `Q <= c <= q*`,
`L_c` is in the case far, so it blocks exactly `[c^k/2, S_c - 1]`. For even
`c > q*`, `L_c` lies below `(c - 2)^k` and blocks nothing new.

*Proof.* `L_c < (c - 1)^k` is `phi(c - 1) < 2`, true since
`phi(c - 1) <= phi(Q - 1) < 2` by (iii). `L_c >= (c - 2)^k` is
`psi(c - 2) >= 2`: for `c = Q` this is (ii); for `Q < c <= q*`, `c - 2` is even,
at least `Q` and below `q*`, so `c - 2 <= t_2` and `psi(c - 2) > 2`. For
`c > q*`, `c - 2 >= q* > t_2`, so `psi(c - 2) < 2`: `L_c < (c - 2)^k`. ∎

**Lemma 5 (powers from `Q` on).** Let `q` be even with `q >= Q`.

- If `q < q*`, `q^k` blocks `[q^k, (q + 2)^k - q^k - 1]`, an interval that
  reaches `L_{q+2} - 1` at least; `(q + 1)^k` blocks nothing outside
  `[q^k, S_{q+2} - 1]`.
- If `q >= q*`, neither `q^k` nor `(q + 1)^k` blocks anything.

*Proof.* `q^k` blocks a non-empty interval exactly when `(q + 2)^k > 2 q^k`,
that is `psi(q) > 2`, that is `q < t_2`, that is `q < q*` for an even `q`. Its
end `(q + 2)^k - q^k - 1` is at least `(q + 2)^k/2 - 1 = L_{q+2} - 1` for the
same reason. For `(q + 1)^k`: its interval, when not empty, starts at
`(q + 1)^k >= q^k` and ends at `(q + 3)^k - (q + 1)^k - 1`, which is at most
`S_{q+2} - 1 = (q + 3)^k - (q + 2)^k/2 - 1` because `phi(q + 1) < 2`, as in
Lemma 4. If `q >= q*`, `psi(q + 1) < psi(q) < 2`, so `(q + 1)^k` blocks
nothing, and neither does `q^k`. ∎

---

## 4. Proof of the theorem

Below `S_Q` every `m` is blocked, and the powers `a^k` with `a < Q` and the
loop positions `L_c` with `c < Q` block nothing from `S_Q` on (section 2). So
from `S_Q` on, the only blockers left are the powers `a^k` with `a >= Q` and
the loop positions `L_c` with `c >= Q`.

By Lemmas 4 and 5, for each even `q` with `Q <= q < q*`, the power `q^k` and the
loop position `L_{q+2}` together block the interval `[q^k, S_{q+2} - 1]`
without gap: `q^k` blocks up to at least `L_{q+2} - 1`, and `L_{q+2}`, in the
case far, blocks `[L_{q+2}, S_{q+2} - 1]`. The odd power `(q + 1)^k` adds
nothing outside it, and nothing at or above `q*` blocks anything. So the
blocked `m >= S_Q` are exactly

```text
[Q^k, S_{Q+2} - 1]  U  [(Q+2)^k, S_{Q+4} - 1]  U ...  U  [(q*-2)^k, S_{q*} - 1]
```

and `I_k` is the complement, as stated. The intervals of `I_k` are non-empty
and separated: `S_q <= q^k - 1` because `phi(q) < 3/2` for `q >= Q > t_1`, and
`S_{q+2} - 1 >= q^k` because `S_{q+2} > (q + 2)^k/2 > q^k` for `q < q*`. ∎

---

## 5. Checks

[`scripts/islands-theorem.py`](../../scripts/islands-theorem.py) compares the
islands given by the theorem with those computed by
[`scripts/islands-chain.py`](../../scripts/islands-chain.py), which follows the
smallest blocking vertex and does not use the theorem. They agree, interval for
interval, for `k = 2..40`; and with `posum islands` of power-sum-graphs 1.0.0
for `k = 2..18`, checked separately.

```console
scripts/islands-theorem.py 40
```
