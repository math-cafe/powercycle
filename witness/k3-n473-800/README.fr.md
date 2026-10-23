# Témoins powercycle : k = 3, chaque n de 473 à 800

Ce répertoire contient un cycle hamiltonien du graphe des sommes de puissances
`G_3(n)` pour chacune des 328 valeurs `n = 473, ..., 800`.

## Énoncé mathématique

`G_3(n)` a pour sommets `1, 2, ..., n` ; deux sommets distincts sont adjacents
exactement lorsque leur somme est un cube parfait. Pour `k = 3`, l'ensemble
admissible est `[473, infini)` : à partir de `n = 473`, tout sommet est de
degré au moins deux. Ces cycles montrent que les 328 premières valeurs
admissibles sont toutes hamiltoniennes, comme le prédit la conjecture de
[`doc/en/islands.md`](../../doc/en/islands.md), section 6.

Chaque fichier `cycles/k3-n<n>.txt.xz` se décompresse en une liste des entiers
`1..n`, un par ligne, dans l'ordre canonique : le sommet `1` d'abord, puis le
plus petit de ses deux voisins. Chaque paire consécutive, y compris la paire
de fermeture entre la dernière et la première ligne, a pour somme un cube
parfait.

Le cycle pour `n = 473` n'est pas le témoin de référence de
[`../k3-n473/`](../k3-n473/) : c'est un autre cycle hamiltonien du même
graphe, trouvé par le balayage ci-dessous.

## Identité

`SHA256SUMS` donne le SHA-256 de chaque fichier **non compressé**, qui
identifie le cycle quelle que soit la compression.

## Provenance

Trouvés le 2026-10-13 par [`generator/cycles_473_800.py`](generator/cycles_473_800.py)
(copié le 2026-10-23 depuis `tmp/gerbicz/`, non versionné, avec sa sortie
`generator/cycles-473-800.csv` ; la procédure est `scripts/nice-pairs.py cycles 3 473 800`, voir
[`doc/en/cube-necklaces.md`](../../doc/en/cube-necklaces.md)) : `pc-tsplib
export`, LKH-3, `pc-tsplib tour` pour la vérification et la forme canonique,
puis le vérificateur Rust indépendant de ce dépôt. Vérifiés de nouveau le
2026-10-19 par les deux vérificateurs de power-sum-graphs 1.0.0 (Rust et
Python, doi:10.5281/zenodo.23403537) : 328 sur 328 valides.

## Vérification

Depuis la racine du dépôt :

```console
D=witness/k3-n473-800
for n in $(seq 473 800); do
  xz -dc $D/cycles/k3-n$n.txt.xz > /tmp/k3-n$n.txt
  python src/verify/python/verify-cycle.py 3 $n /tmp/k3-n$n.txt > /dev/null || echo "n = $n FAILED"
done
(cd /tmp && sha256sum -c --quiet $OLDPWD/$D/SHA256SUMS)
```

## Fichiers

- `cycles/k3-n<n>.txt.xz` — les 328 cycles canoniques compressés ;
- `SHA256SUMS` — SHA-256 des cycles non compressés ;
- `README.md` — version anglaise ;
- `README.fr.md` — ce document.
