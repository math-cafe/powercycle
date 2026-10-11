# Témoin powercycle : k = 4, n = 9641

Ce répertoire contient le témoin hamiltonien **minimal**, conservé et versionné,
pour le graphe de sommes de puissances `G_4(9641)`.

## Énoncé mathématique

Le graphe a pour sommets `1, 2, ..., 9641`. Deux sommets distincts `a` et `b` sont
adjacents exactement lorsque `a + b` est une puissance parfaite d'exposant 4.

`cycle.txt.xz` se décompresse en un fichier texte simple, un sommet par ligne.
Chaque entier de `1` à `9641` apparaît exactement une fois, et chaque paire de
sommets consécutifs, y compris la paire de fermeture entre la dernière et la
première ligne, somme à une puissance parfaite d'exposant 4.

## Minimalité

Ce bundle porte les deux moitiés de

```text
n_min(4) = 9641
```

**L'existence** est `cycle.txt`.

**La borne inférieure** est `bands.txt` : pour tout `n` dans `[1, 9640]`, le
graphe `G_4(n)` possède un sommet de degré au plus un, qu'aucun cycle
hamiltonien ne peut emprunter. Le sommet contraignant juste sous le seuil est

```text
v = 5000 = 10^4 / 2
```

la position de boucle de `10^4`, qui reste de degré un jusqu'à ce que `11^4`
entre dans sa fenêtre, à

```text
n = 11^4 - 10^4/2 = 14641 - 5000 = 9641
```

La condition de degré est nécessaire et **non** suffisante : cet argument seul ne
peut donc pas trancher `n_min` pour tout exposant. Voir
[`../k5-n246877/README.fr.md`](../k5-n246877/README.fr.md) pour la discussion
générale et pour le cas `k = 2`, où la borne analogue vaut `31` alors que le
minimum réel est `32`.

## Travaux antérieurs

`n_min(4) = 9641` n'est pas nouveau : c'est le terme a(4) de l'OEIS [A115418](https://oeis.org/A115418), soumise
par Roberto Tauraso en 2006. Ce bundle apporte un cycle trouvé et vérifié de
façon indépendante, ainsi que le certificat de borne inférieure, dont aucun
équivalent publié n'a été trouvé.

## Checksums

Des artefacts **non compressés** :

```text
cycle.txt          c3ea9c7f188333eaea35dfeeca9320bd63889e9d724f634e1fe984595b9da1e5
bands.txt          de031e95b2cdca66c40880616f2cda572365f254a38365868c57073880a6a27d
final.pcdump.json  de15f073d202395161cee3a27dd20e0ebf6fa0b9303db02cbee0db629f9ec211
```

Le cycle est exporté sous forme canonique : rotation plaçant le sommet `1` en
premier, orientation plaçant le plus petit voisin de `1` en second. Il commence
par `1, 15, ...`, se termine par `80`, et se ferme sur
`80 + 1 = 81 = 3^4`.

`bands.txt` est la sortie de `pc-scan --bands` pour `n = 1 .. 9640` : 10
lignes, chacune une plage de `n` avec le sommet de degré au plus un et son
degré. Le degré d'un sommet ne décroît jamais quand `n` augmente, puisque sa
fenêtre ne fait que s'élargir : une bande vraie à ses deux bouts l'est partout,
et les vérificateurs contrôlent chaque bande à ses deux extrémités. Jusqu'à la
version 0.1.0, le bundle portait à la place `scan.txt`, une ligne par `n`,
toujours disponible au tag `witness-k4-n9641`.

## Provenance

Le dump final `final.pcdump.json.xz` a pour identifiant sémantique `940ef6637add…`.

- le tour SAT 1 produit un 2-facteur brut de 6 cycles, avec un géant de taille 5023 et un résiduel de 4618 sommets ;
- un mouvement glouton `three-opt-21` porte le géant à 9157 ;
- quatre réparations alternées bornées, aux profondeurs 2, 4, 2 et 2, absorbent tour à tour des satellites de 212, 204, 34 et 34 sommets, produisant un cycle unique de longueur 9641.

La durée totale de la recherche est de 0.021 seconde(s).

Le dump final est un artefact de provenance. Le certificat mathématique est le
cycle texte lui-même.

## Vérification indépendante

```console
xz -dc witness/k4-n9641/cycle.txt.xz > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 4 9641 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py 4 9641 /tmp/powercycle-cycle.txt
```

La borne inférieure se vérifie, et se rejoue, avec :

```console
xz -dc witness/k4-n9641/bands.txt.xz > /tmp/powercycle-bands.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  --bin powercycle-verify-bands -- 4 9641 /tmp/powercycle-bands.txt

python src/verify/python/verify-bands.py 4 9641 /tmp/powercycle-bands.txt

cargo run --release --bin pc-scan -- --bands --quick-only 4 1 9640 \
  | diff - /tmp/powercycle-bands.txt
```

Les vérificateurs sont indépendants des bibliothèques powercycle : ils
n'utilisent que `k`, `n` et le fichier qu'ils contrôlent.

## Licence

Le contenu de ce répertoire est placé sous CC0 1.0 Universal. Voir
[`../LICENSE`](../LICENSE).
