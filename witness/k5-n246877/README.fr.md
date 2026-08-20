# Témoin powercycle : k = 5, n = 246877

Ce répertoire contient le témoin hamiltonien **minimal**, conservé et versionné,
pour le graphe de sommes de puissances `G_5(246877)`.

## Énoncé mathématique

Le graphe a pour sommets `1, 2, ..., 246877`. Deux sommets distincts `a` et `b`
sont adjacents exactement lorsque `a + b` est une puissance cinquième parfaite.

Le fichier `cycle.txt.zst` contient un cycle hamiltonien : après décompression,
c'est un fichier texte simple comportant un sommet par ligne. Chaque entier de
`1` à `246877` apparaît exactement une fois et la somme de chaque paire de
sommets consécutifs, y compris la paire de fermeture entre la dernière et la
première ligne, est une puissance cinquième parfaite.

## Minimalité

Ce bundle porte les deux moitiés de l'énoncé

```text
n_min(5) = 246877
```

c'est-à-dire : `246877` est le plus petit `n` pour lequel `G_5(n)` possède un
cycle hamiltonien.

**Existence.** `cycle.txt` est un cycle hamiltonien de `G_5(246877)`.

**Borne inférieure.** Pour tout `n` dans `[1, 246876]`, le graphe `G_5(n)`
contient un sommet de degré au plus un. Un cycle hamiltonien emprunte exactement
deux arêtes en chaque sommet, donc aucun de ces `n` n'en admet. `scan.txt`
consigne ce sommet pour chaque `n`.

L'obstruction est élémentaire et ne dépend d'aucune recherche. Le sommet `v` est
adjacent à `p - v` exactement lorsque `p` est une puissance cinquième parfaite
vérifiant `v + 1 <= p <= v + n` et `p != 2v`, le cas exclu étant la boucle.
Compter ces puissances donne directement le degré de `v`.

Le sommet contraignant juste sous le seuil est

```text
v = 124416 = 12^5 / 2
```

Il se situe dans l'intervalle engendré par `12^5 = 248832` mais ne peut pas
utiliser cette puissance, car `124416 + 124416` est une boucle et non une arête.
Sa seule autre puissance est `11^5 = 161051`, il reste donc de degré un jusqu'à
ce que `13^5 = 371293` entre dans sa fenêtre, ce qui se produit exactement à

```text
n = 13^5 - 12^5 / 2 = 371293 - 124416 = 246877
```

### Ce que la borne inférieure ne dit pas

La condition de degré est **nécessaire, pas suffisante**. La satisfaire
n'implique pas qu'un cycle hamiltonien existe, et le même argument ne peut donc
pas à lui seul déterminer `n_min` pour d'autres exposants.

Le cas des sommes de carrés est le contre-exemple à garder en tête : pour
`k = 2`, la borne de degré analogue vaut `31`, et `G_2(31)` passe toutes les
conditions nécessaires implémentées dans `pc-scan`, alors que le plus petit `n`
admettant un cycle hamiltonien est `32`.

Ici les deux moitiés se rejoignent, parce qu'un témoin existe au premier `n` que
la borne de degré autorise. C'est cette coïncidence qui fait de
`n_min(5) = 246877` un théorème et non une simple borne.

## Identité canonique du témoin

Le fichier `cycle.txt` non compressé a pour SHA-256 :

`8988c848256b7227d95e7501c517700a4970fc67fb79151e178ebc5bd630fc3b`

Le cycle est exporté sous forme canonique : rotation plaçant le sommet `1` en
première position, puis choix de l'orientation pour laquelle le plus petit des
deux voisins de `1` est placé en deuxième position. Il commence par
`1, 31, 212, ...` et se termine par `242`, avec la fermeture
`242 + 1 = 243 = 3^5`.

`cycle.txt.sha256` contient le checksum du témoin **non compressé**.

## Certificat de scan

Le fichier `scan.txt` non compressé a pour SHA-256 :

`a1d8f02c74c32e08887fa921bfeeae15b4b59ca923f8c408b4545ad79a8e23ce`

C'est la sortie de `pc-scan` pour `n = 1 .. 246877`, un enregistrement séparé par
tabulations par ligne. Les `246876` premières lignes portent le verdict
`IMPOSSIBLE` accompagné du sommet fautif et de son degré ; la dernière ligne
porte le verdict `CANDIDATE` à `246877`. Le fichier contient donc tout l'argument
de borne inférieure et le seuil dans un seul artefact.

Tous les rejets de l'intervalle proviennent du test de degré sans graphe : sa
reproduction ne construit aucun graphe.

## Provenance

Le dump final versionné de powercycle est conservé sous la forme
`final.pcdump.json.zst`. Son identifiant sémantique est :

`110da2ad0fd9…`

La chaîne ayant produit ce témoin est la suivante :

- le tour SAT 1 produit un 2-facteur brut de 8 cycles, avec un cycle géant de
  taille 182200 et un résiduel de 64677 sommets ;
- le tour SAT 2, après un tour de coupes d'élimination de sous-tours de bord
  sous budget, produit un 2-facteur brut de 5 cycles, avec un géant de taille
  106589 et un résiduel de 140288 sommets ;
- deux mouvements gloutons `three-opt-111` referment le tout en un cycle unique
  de longueur 246877. Aucun mouvement `2+1` ni aucune réparation alternée n'a
  été nécessaire.

La durée totale de la recherche est de 4,4 secondes.

Le dump final est un artefact de provenance. Le certificat mathématique est le
cycle texte lui-même.

## Vérification indépendante

Le dépôt contient deux vérificateurs de témoin indépendants :

- un vérificateur Rust autonome dans `src/verify/rust/` ;
- un vérificateur Python n'utilisant que la bibliothèque standard, dans
  `src/verify/python/`.

Ils ne vérifient que la définition du témoin : permutation de `1..n` et condition
de puissance cinquième sur chaque somme consécutive, fermeture comprise. Ils
n'utilisent ni le solveur SAT, ni le format de dump, ni les identifiants d'arêtes
du graphe, ni l'historique de patch, ni l'implémentation de la recherche.

Les vérifications habituelles, depuis la racine du dépôt, sont :

```console
zstd -dc witness/k5-n246877/cycle.txt.zst > /tmp/powercycle-cycle.txt

sha256sum /tmp/powercycle-cycle.txt

cargo run --release \
  --manifest-path src/verify/rust/Cargo.toml \
  -- 5 246877 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py \
  5 246877 /tmp/powercycle-cycle.txt
```

La borne inférieure se rejoue avec :

```console
zstd -dc witness/k5-n246877/scan.txt.zst > /tmp/powercycle-scan.txt

sha256sum /tmp/powercycle-scan.txt

cargo run --release --bin pc-scan -- 5 1 246877 | diff - /tmp/powercycle-scan.txt
```

Les SHA-256 rapportés par les commandes de checksum doivent correspondre à ceux
donnés ci-dessus.

## Fichiers

- `cycle.txt.zst` — témoin de cycle hamiltonien canonique, compressé.
- `cycle.txt.sha256` — SHA-256 du `cycle.txt` non compressé.
- `scan.txt.zst` — certificat de borne inférieure `pc-scan`, compressé.
- `scan.txt.sha256` — SHA-256 du `scan.txt` non compressé.
- `final.pcdump.json.zst` — dump powercycle final compressé, avec provenance.
- `final.pcdump.json.sha256` — SHA-256 du dump final non compressé.
- `README.md` — version anglaise, faisant foi.
- `README.fr.md` — ce document.

## Voir aussi

`witness/k5-n490463/` conserve un témoin antérieur, non minimal, pour le même
exposant. Il reste valide et vérifié de façon indépendante, mais le résultat de
référence pour `k = 5` est celui de ce répertoire.
