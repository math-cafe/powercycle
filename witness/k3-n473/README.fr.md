# Témoin powercycle : k = 3, n = 473

Ce répertoire contient le témoin hamiltonien **minimal**, conservé et versionné,
pour le graphe de sommes de puissances `G_3(473)`.

## Énoncé mathématique

Le graphe a pour sommets `1, 2, ..., 473`. Deux sommets distincts `a` et `b` sont
adjacents exactement lorsque `a + b` est une puissance parfaite d'exposant 3.

`cycle.txt.xz` se décompresse en un fichier texte simple, un sommet par ligne.
Chaque entier de `1` à `473` apparaît exactement une fois, et chaque paire de
sommets consécutifs, y compris la paire de fermeture entre la dernière et la
première ligne, somme à une puissance parfaite d'exposant 3.

## Minimalité

Ce bundle porte les deux moitiés de

```text
n_min(3) = 473
```

**L'existence** est `cycle.txt`.

**La borne inférieure** est `bands.txt` : pour tout `n` dans `[1, 472]`, le
graphe `G_3(n)` possède un sommet de degré au plus un, qu'aucun cycle
hamiltonien ne peut emprunter. Le sommet contraignant juste sous le seuil est

```text
v = 256 = 8^3 / 2
```

la position de boucle de `8^3`, qui reste de degré un jusqu'à ce que `9^3`
entre dans sa fenêtre, à

```text
n = 9^3 - 8^3/2 = 729 - 256 = 473
```

La condition de degré est nécessaire et **non** suffisante : cet argument seul ne
peut donc pas trancher `n_min` pour tout exposant. Voir
[`../k5-n246877/README.fr.md`](../k5-n246877/README.fr.md) pour la discussion
générale et pour le cas `k = 2`, où la borne analogue vaut `31` alors que le
minimum réel est `32`.

## Checksums

Des artefacts **non compressés** :

```text
cycle.txt          2638ac29ca8b0463836d96fac7c1eadda2638ceda07f20200b49db9b341d824f
bands.txt          e9717e68480df786ee177f7ff4b50263953457f6beb8c6c48fc3afcc7f5ae729
final.pcdump.json  366314d4cc03ce77601a76fd4011ef63c79c679316992dafcb205cadb76c27d1
```

Le cycle est exporté sous forme canonique : rotation plaçant le sommet `1` en
premier, orientation plaçant le plus petit voisin de `1` en second. Il commence
par `1, 63, ...`, se termine par `342`, et se ferme sur
`342 + 1 = 343 = 7^3`.

`bands.txt` est la sortie de `pc-scan --bands` pour `n = 1 .. 472` : 8
lignes, chacune une plage de `n` avec le sommet de degré au plus un et son
degré. Le degré d'un sommet ne décroît jamais quand `n` augmente, puisque sa
fenêtre ne fait que s'élargir : une bande vraie à ses deux bouts l'est partout,
et les vérificateurs contrôlent chaque bande à ses deux extrémités. Jusqu'à la
version 0.1.0, le bundle portait à la place `scan.txt`, une ligne par `n`,
toujours disponible au tag `witness-k3-n473`.

## Provenance

Le dump final `final.pcdump.json.xz` a pour identifiant sémantique `bdb9fcc35ded…`.

- le tour SAT 1 produit un 2-facteur brut de 7 cycles, avec un géant de taille 204 et un résiduel de 269 sommets ;
- deux mouvements gloutons `three-opt-111` le ramènent à 3 cycles, avec un géant de taille 441 ;
- deux mouvements gloutons `three-opt-21` absorbent les satellites restants en un cycle unique de longueur 473.

La durée totale de la recherche est de 0.001 seconde(s).

Le dump final est un artefact de provenance. Le certificat mathématique est le
cycle texte lui-même.

## Vérification indépendante

```console
xz -dc witness/k3-n473/cycle.txt.xz > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 3 473 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py 3 473 /tmp/powercycle-cycle.txt
```

La borne inférieure se vérifie, et se rejoue, avec :

```console
xz -dc witness/k3-n473/bands.txt.xz > /tmp/powercycle-bands.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  --bin powercycle-verify-bands -- 3 473 /tmp/powercycle-bands.txt

python src/verify/python/verify-bands.py 3 473 /tmp/powercycle-bands.txt

cargo run --release --bin pc-scan -- --bands --quick-only 3 1 472 \
  | diff - /tmp/powercycle-bands.txt
```

Les vérificateurs sont indépendants des bibliothèques powercycle : ils
n'utilisent que `k`, `n` et le fichier qu'ils contrôlent.

## Licence

Le contenu de ce répertoire est placé sous CC0 1.0 Universal. Voir
[`../LICENSE`](../LICENSE).
