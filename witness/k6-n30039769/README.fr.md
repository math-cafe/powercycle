# Témoin powercycle : k = 6, n = 30039769

Un cycle hamiltonien du graphe des sommes de puissances `G_6(30039769)`,
première valeur du continent de `k = 6`.

## Énoncé

`G_6(n)` a pour sommets `1, ..., n` ; deux sommets distincts sont adjacents
lorsque leur somme est une puissance sixième parfaite. Pour `k = 6`,
l'ensemble admissible est `[15748961, 16777215] U [30039769, infini)`.
`G_6(15748961)` est hamiltonien (power-sum-graphs 1.0.0), aucun `n` de
16777216 à 30039768 n'est admissible, et ce cycle montre que `G_6(30039769)`
est de nouveau hamiltonien.

## Codage et identité

Le cycle a 30 millions de sommets ; il est donc stocké dans le codage par
bases de power-sum-graphs (`doc/formats.md`, section 3) : `a7aa7317.bases.xz`
contient les `n - 1` racines sixièmes `b_i` telles que `v_{i+1} = b_i^6 - v_i`,
à partir de `v_1 = 1`. La liste décodée, un sommet par ligne, dans l'ordre
canonique, a pour SHA-256

`a7aa7317ddc6a3d38164b0d0372c0ab862eab57b3132f7a514a0e262491d4eb6`

## Provenance

Trouvé le 2026-10-21 par `posum search 6 30039769` (power-sum-graphs 1.0.0)
avec LKH-3.0.14 sur un AMD Ryzen 9 7940HS : 23450876 nœuds après propagation
et contraction, 3 essais, 4820 s, environ 15 Go. `provenance.toml` est
l'enregistrement de posum. Accepté par les deux vérificateurs de
power-sum-graphs 1.0.0 (Rust et Python, sur la liste en clair) et par
[`scripts/verify-bases.sh`](../../scripts/verify-bases.sh), qui ne partage aucun
code avec eux.

## Vérification

```console
scripts/verify-bases.sh 6 30039769 witness/k6-n30039769/a7aa7317.bases.xz
```

affiche `VALID` et l'empreinte ci-dessus.
