# Témoin powercycle : k = 6, n = 15748961

Ce répertoire contient le témoin hamiltonien **minimal**, conservé et versionné,
pour le graphe de sommes de puissances `G_6(15748961)`.

## Énoncé mathématique

Le graphe a pour sommets `1, 2, ..., 15748961`. Deux sommets distincts `a` et
`b` sont adjacents exactement lorsque `a + b` est une puissance sixième
parfaite.

`cycle.txt.xz` se décompresse en un fichier texte simple, un sommet par ligne.
Chaque entier de `1` à `15748961` apparaît exactement une fois, et chaque paire
de sommets consécutifs, y compris la paire de fermeture entre la dernière et la
première ligne, somme à une puissance sixième parfaite.

## Minimalité

Ce bundle porte les deux moitiés de

```text
n_min(6) = 15748961
```

**L'existence** est `cycle.txt`.

**La borne inférieure** est `scan.txt` : pour tout `n` dans `[1, 15748960]`, le
graphe `G_6(n)` possède un sommet de degré au plus un, qu'aucun cycle
hamiltonien ne peut emprunter. Le sommet contraignant juste sous le seuil est

```text
v = 8388608 = 16^6 / 2
```

la position de boucle de `16^6` : il se situe dans l'intervalle qu'engendre
cette puissance mais ne peut pas l'utiliser, et reste donc de degré un jusqu'à
ce que `17^6` entre dans sa fenêtre, à

```text
n = 17^6 - 16^6/2 = 24137569 - 8388608 = 15748961
```

La condition de degré est nécessaire et **non** suffisante — voir
[`../k5-n246877/README.fr.md`](../k5-n246877/README.fr.md) pour le cas `k = 2`,
où la borne analogue vaut `31` alors que le minimum réel est `32`. Ici les deux
moitiés se rejoignent, parce qu'un témoin existe au premier `n` que la borne
autorise.

## Checksums

Des artefacts **non compressés** :

```text
cycle.txt          5f2b4192388caf362098311b1363163bfb78b554fb2ae8109608f773f50066c5
scan.txt           eb9f0c13ea120f2e74f709bdb61b4c694e6a41990b472c192325b34cddddf86c
final.pcdump.json  df6402b4d5ebd5a68ba59a2d4d6edfae99efa83c6f46a9fc6da0c97613bf0402
```

Le cycle est exporté sous forme canonique : rotation plaçant le sommet `1` en
premier, orientation plaçant le plus petit voisin de `1` en second. Il commence
par `1, 63, ...`, se termine par `531440`, et se ferme sur
`531440 + 1 = 531441 = 9^6`.

`scan.txt` est la sortie de `pc-scan` pour `n = 1 .. 15748961` : 15 748 960
lignes `IMPOSSIBLE` avec le sommet fautif et son degré, puis le verdict
`CANDIDATE` au seuil. Il se comprime 120 fois parce que tous les rejets de
l'intervalle proviennent du test de degré sans graphe — le rejouer ne construit
aucun graphe et prend 25 secondes.

## Provenance

Le dump final `final.pcdump.json.xz` a pour identifiant sémantique
`34a7b80245ee…`.

La chaîne qui a produit ce témoin, avec ses durées :

- **tour SAT 1**, 3 h 59 (14 332 s), produit un 2-facteur brut de 18 391
  cycles, avec un géant de 9 571 535 sommets et un résiduel de 6 177 426 ;
- **`three-opt-111` glouton**, 710 mouvements en 3 413 s, le ramène à 16 971
  cycles ;
- **`three-opt-21` glouton**, 15 902 mouvements en 6 326 s, à 1 069 cycles ;
- **1 056 réparations alternées à profondeur ≤ 4**, environ 21 heures, à 7
  cycles avec un géant de 15 748 845 sommets et 6 satellites totalisant 116
  sommets. La recherche atteint là un point fixe local : aucune réparation
  n'existe à cette profondeur ;
- **5 réparations à profondeur ≤ 6**, 78 s, à 2 cycles — un seul satellite de
  10 sommets subsiste ;
- **1 réparation à profondeur ≤ 9**, 21 s, qui referme le cycle.

La profondeur a été le paramètre décisif. Trente-quatre heures de patch se sont
enlisées à profondeur 4 ; l'augmenter a refermé le facteur en deux minutes.

Le dernier satellite à résister mérite d'être consigné. Dix sommets, avec une
symétrie d'ordre 2 exacte :

```text
425729 —12⁶— 2560255 —16⁶— 14216961 —17⁶— 9920608 —16⁶— 6856608 —14⁶—
672928 —12⁶— 2313056 —16⁶— 14464160 —17⁶— 9673409 —16⁶— 7103807 —14⁶— 425729
```

La suite de puissances `12⁶, 16⁶, 17⁶, 16⁶, 14⁶` s'y répète deux fois. Deux de
ses sommets, `14216961` et `14464160`, ont degré exactement deux dans le
graphe : leurs deux arêtes sont donc forcées dans tout 2-facteur. C'est cette
rigidité qui a exigé un chemin alternant de longueur neuf pour briser le cycle.

Le dump final est un artefact de provenance. Le certificat mathématique est le
cycle texte lui-même.

## Vérification indépendante

```console
xz -dc witness/k6-n15748961/cycle.txt.xz > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 6 15748961 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py 6 15748961 /tmp/powercycle-cycle.txt
```

La borne inférieure se rejoue avec :

```console
xz -dc witness/k6-n15748961/scan.txt.xz > /tmp/powercycle-scan.txt
cargo run --release --bin pc-scan -- 6 1 15748961 | diff - /tmp/powercycle-scan.txt
```

Les deux vérificateurs sont indépendants des bibliothèques powercycle : ils
n'utilisent que `k`, `n` et le témoin candidat.

## Fichiers

- `cycle.txt.xz` — témoin de cycle hamiltonien canonique, compressé (24 Mo).
- `cycle.txt.sha256` — SHA-256 du `cycle.txt` non compressé.
- `scan.txt.xz` — certificat de borne inférieure `pc-scan`, compressé (4,5 Mo).
- `scan.txt.sha256` — SHA-256 du `scan.txt` non compressé.
- `final.pcdump.json.xz` — dump final compressé, avec provenance (11 Mo).
- `final.pcdump.json.sha256` — SHA-256 du dump final non compressé.
- `README.md` — version anglaise, faisant foi.
- `README.fr.md` — ce document.

Un cycle hamiltonien est quasi incompressible — environ 12 bits par sommet ici,
contre une borne informationnelle proche de 1,7 — si bien que `cycle.txt.xz`
domine ce bundle. Un encodage par indice de voisin serait bien plus petit mais
exigerait le graphe, 922 Mo à cette taille, pour décoder. La liste brute se
vérifie avec vingt lignes et aucune structure de données, ce qui vaut mieux que
la place gagnée.

## Licence

Le contenu de ce répertoire est placé sous CC0 1.0 Universal. Voir
[`../LICENSE`](../LICENSE).
