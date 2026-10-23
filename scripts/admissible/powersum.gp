\\ The admissible set of the power-sum graphs G_k(n) (Theorems A and B).
\\ Exact integer arithmetic; c is always even.
Q(k) = my(c = 2); while (2*(c+1)^k >= 3*c^k, c += 2); c;
qstar(k) = my(c = Q(k)); while ((c+2)^k >= 2*c^k, c += 2); c;
S(k, c) = (c+1)^k - c^k/2;
A(k) = S(k, Q(k));                \\ least admissible n
mainland(k) = S(k, qstar(k));     \\ start of the mainland
N(k) = (qstar(k) - Q(k))/2;       \\ number of islands
islands(k) = my(q = Q(k)); vector(N(k), i, my(c = q + 2*i - 2); [S(k, c), c^k - 1]);
