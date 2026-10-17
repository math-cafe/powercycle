# The least admissible n, in closed form

[`islands.md`](islands.md) observes that every first admissible `n` is
`(q+1)^k - q^k/2` for an even `q`, and section 5 explains the chain of blocking
vertices that leads there. This note turns the observation into a theorem, for
every `k >= 2`.

---

## 1. Statement

For `k >= 2` and `m >= 1`, `G_k(m)` has vertices `1..m`, two of them adjacent
when their sum is a `k`-th power. Call `m` **admissible** when every vertex of
`G_k(m)` has degree at least two.

> **Theorem.** Let `k >= 2` and let `Q` be the least **even** integer with
> `(1 + 1/Q)^k < 3/2`. The least admissible `m` is
>
> ```text
> A(k) = (Q + 1)^k - Q^k / 2.
> ```

`Q = 6, 8, 10, 12, 16, 18, 20` for `k = 2..8`, giving
`31, 473, 9641, 246877, 15748961, 587761723, 25022859361`. Since a Hamiltonian
cycle needs minimum degree two, `n_min(k) >= A(k)` for every `k`; equality is
proved for `k = 3..6` and conjectured for every `k >= 3`
([`islands.md`](islands.md), section 6). For `k = 1` the graph is complete and
the formula also gives the right value, `5 - 2 = 3`, but the proof below
assumes `k >= 2`.

---

## 2. Notation

- A power `p = b^k` is **usable** for `v` when `p > v` and `p != 2v`. The degree
  of `v <= m` in `G_k(m)` is the number of usable powers in `(v, v + m]`: each
  gives the neighbour `p - v`, which lies in `1..m` and differs from `v`.
- `p_2(v)` is the second usable power above `v`, and `g(v) = p_2(v) - v`.
- `phi(t) = (1 + 1/t)^k`, decreasing in `t`.
- `Delta(t) = (t + 1)^k - t^k`, increasing in `t`.
- A **loop position** is `L_c = c^k / 2` for an even `c`: the vertex that cannot
  use the power `c^k`.

---

## 3. Reduction to the blocking vertices

**Lemma 1.** A vertex `v <= m` has degree at least two in `G_k(m)` if and only
if `g(v) <= m`. So `m` is admissible if and only if `g(v) <= m` for every
`v <= m`. Say that `v` **blocks** `m` when `v <= m < g(v)`.

*Proof.* The degree counts the usable powers in `(v, v + m]`, and the second
one is `p_2(v)`. ∎

**Lemma 2.** Let `a^k <= v < (a + 1)^k`.

- If `v` is not a loop position, `p_2(v) = (a + 2)^k`, so
  `g(v) <= g(a^k) = (a + 2)^k - a^k`, with `a^k <= v`.
- If `v = L_c` with `c = a + 1`, `g(v) = (c + 2)^k - c^k/2`.
- If `v = L_c` with `c = a + 2`, `g(v) = (c + 1)^k - c^k/2`.
- If `v = L_c` with `c >= a + 3`, `p_2(v) = (a + 2)^k` and again
  `g(v) < g(a^k)`.

So every blocked `m` is blocked by a power `a^k` (with `a = 1` covering the
vertex 1) or by a loop position in one of the two middle cases.

*Proof.* The usable powers above `v` are `(a+1)^k, (a+2)^k, ...` minus `2v`,
which removes `(a+1)^k` or `(a+2)^k` only in the two middle cases. `2 a^k` is
never a `k`-th power, so `a^k` is not a loop position. ∎

---

## 4. Four inequalities

Since `phi(2) = (3/2)^k >= 3/2` and `phi(4) = (5/4)^k >= 25/16` for `k >= 2`,
`Q >= 6`.

**(i)** `phi(c) >= 3/2` for every even `c < Q`. This is the definition of `Q`.

**(ii)** `(1 + 2/(Q - 2))^k >= 2`.

*Proof.* Let `lambda = log 2 / log(3/2)`. On `[0, 1/2]`,
`h(x) = log(1 + 2x) - lambda log(1 + x)` vanishes at both ends and is concave:
`h''(x) < 0` amounts to `2(1 + x) > sqrt(lambda) (1 + 2x)`, true for
`x < 1.12`. So `h >= 0` there, that is `1 + 2x >= (1 + x)^lambda`. With
`x = 1/(Q - 2) <= 1/4` and (i) at `c = Q - 2`:
`(1 + 2/(Q-2))^k >= phi(Q - 2)^lambda >= (3/2)^lambda = 2`. ∎

**(iii)** `phi(Q - 1) < 2`.

*Proof.* `phi(Q - 1) = phi(Q) (1 + 1/(Q^2 - 1))^k`. From `phi(Q) < 3/2` and
`log(1 + 1/Q) >= 1/(Q + 1)`, `k < (Q + 1) log(3/2)`, so
`(1 + 1/(Q^2 - 1))^k <= exp(k/(Q^2 - 1)) < exp(log(3/2)/(Q - 1)) <= (3/2)^(1/5)`
for `Q >= 6`. Hence `phi(Q - 1) < (3/2)^(6/5) < 1.63`. ∎

**(iv)** `A < Q^k`, which is `phi(Q) < 3/2`.

---

## 5. Every `m < A` is blocked

The blocking intervals `[x, g(x) - 1]` chain without gap from 1 to `A - 1`,
climbing through the even `c`.

- **Start.** The vertex 1 blocks `[1, 3^k - 2]`. `L_2 = 2^(k-1)` lies in
  `[1, 2^k)`, so it is in the case `c = a + 1` and blocks
  `[2^(k-1), 4^k - 2^(k-1) - 1]`, which overlaps.
- **Step, for even `c` with `2 <= c <= Q - 2`.** Suppose `L_c` blocks from
  `c^k/2` on, in one of the two middle cases. In the case `c = a + 2` it blocks
  up to `(c + 1)^k - c^k/2 - 1 >= c^k - 1` by (i); in the case `c = a + 1`, even
  further. Then `c^k` blocks `[c^k, (c + 2)^k - c^k - 1]`, and
  `(c + 2)^k - c^k >= (c + 2)^k / 2` by (ii) and the monotonicity of
  `(1 + 2/c)^k`. So `L_{c+2}` starts within reach. It is not in the case
  `c + 2 >= a + 3`, since `L_{c+2} >= c^k` by the same inequality, so it again
  blocks from `(c + 2)^k/2` on, in a middle case.
- **End.** By (ii) and (iii), `(Q - 2)^k <= L_Q < (Q - 1)^k`: `L_Q` is in the
  case `c = a + 2` and blocks exactly `[Q^k/2, (Q + 1)^k - Q^k/2 - 1] = [Q^k/2, A - 1]`.

---

## 6. Nothing blocks `A`

By (iv) and Lemma 2, the only vertices to examine are the powers `a^k` with
`a <= Q - 1` and the loop positions `L_c` with `c <= Q`; `L_{Q+2}` lies above
`A`, since `(Q + 2)^k + Q^k > 2 (Q + 1)^k` by strict convexity.

- **Powers.** `g(a^k) = (a + 2)^k - a^k <= (Q + 1)^k - (Q - 1)^k <= A`, the
  last step because `(Q - 1)^k >= Q^k / 2` by (iii).
- **`L_Q`.** `g(L_Q) = A` exactly, which does not block `A`.
- **`L_c` with `c <= Q - 2`.** In either middle case
  `g(L_c) <= (c + 2)^k - c^k/2`, increasing in `c`, so
  `g(L_c) <= Q^k - (Q - 2)^k/2`. And
  `Q^k - (Q-2)^k/2 <= A` amounts to
  `(Q^k - (Q - 2)^k)/2 = (Delta(Q - 2) + Delta(Q - 1))/2 <= Delta(Q)`, true since
  `Delta` increases.

So `A` is admissible and every smaller `m` is not. ∎

---

## 7. Checks

[`scripts/least-admissible.py`](../../scripts/least-admissible.py) checks, in
exact integer arithmetic:

- the theorem against a direct computation of the least admissible `m` — the
  chain of blocking vertices over the `O(k)` candidates, independent of the
  formula — for `k = 1..100` (it also agrees with `posum islands` of
  power-sum-graphs 1.0.0 for `k = 2..18`, checked separately);
- every inequality of sections 4 to 6 (`Q >= 6`, (i) to (iv), the position of
  `L_Q` and of `L_{Q+2}`, and the two bounds of section 6) for `k = 2..600`.

```console
scripts/least-admissible.py terms 100     # the b-file: k, A(k)
scripts/least-admissible.py check 600     # the formula and every inequality
```
