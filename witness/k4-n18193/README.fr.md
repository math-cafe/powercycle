# Témoin powercycle : k = 4, n = 18193

Ce répertoire contient un cycle hamiltonien du graphe des sommes de puissances
`G_4(18193)`.

## Énoncé mathématique

`G_4(n)` a pour sommets `1, 2, ..., n` ; deux sommets distincts sont adjacents
exactement lorsque leur somme est une puissance quatrième parfaite. Pour
`k = 4`, l'ensemble admissible est `[9641, 9999] U [18193, infini)` : une île,
puis le continent. `18193` est la première valeur du continent.

Tout `n` de l'île est hamiltonien (bundle d'île de power-sum-graphs 1.0.0),
aucun `n` de 10000 à 18192 ne l'est (un sommet y est de degré au plus un), et
ce cycle montre que `G_4(18193)` est de nouveau hamiltonien. L'hamiltonicité
de `G_4(n)` n'est donc pas monotone en `n`.

`cycle.txt.xz` se décompresse en une liste des entiers `1..18193`, un par
ligne, dans l'ordre canonique : le sommet `1` d'abord, puis le plus petit de
ses deux voisins. Chaque paire consécutive, y compris la paire de fermeture
entre la dernière et la première ligne, a pour somme une puissance quatrième
parfaite.

## Identité

Le fichier `cycle.txt` non compressé a pour SHA-256 :

`699280154bcdbeec8325c52ef5f06cf546c5ccc57ac7d4466f3b379b86005d9c`

## Provenance

Trouvé le 2026-10-19 par `posum search 4 18193` (power-sum-graphs 1.0.0,
doi:10.5281/zenodo.23403537) avec LKH-3.0.14 : un essai, 0,1 s, après une
propagation et une contraction laissant 13532 nœuds. `provenance.toml` est
l'enregistrement de posum. Vérifié par les deux vérificateurs de
power-sum-graphs 1.0.0 (Rust et Python) et par ceux de ce dépôt.

## Vérification

Depuis la racine du dépôt :

```console
xz -dc witness/k4-n18193/cycle.txt.xz > /tmp/k4-n18193.txt
sha256sum /tmp/k4-n18193.txt          # doit correspondre à cycle.txt.sha256
cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 4 18193 /tmp/k4-n18193.txt
python src/verify/python/verify-cycle.py 4 18193 /tmp/k4-n18193.txt
```

## Fichiers

- `cycle.txt.xz` — le cycle canonique compressé ;
- `cycle.txt.sha256` — SHA-256 du `cycle.txt` non compressé ;
- `provenance.toml` — enregistrement de provenance de posum ;
- `README.md` — version anglaise ;
- `README.fr.md` — ce document.
