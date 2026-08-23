# Témoin powercycle : k = 5, n = 490463

Ce répertoire contient un témoin hamiltonien conservé et versionné pour le graphe
de sommes de puissances `G_5(490463)`.

## Énoncé mathématique

Le graphe a pour sommets `1, 2, ..., 490463`. Deux sommets distincts `a` et `b`
sont adjacents exactement lorsque `a + b` est une puissance cinquième parfaite.

Le fichier `cycle.txt.xz` contient un cycle hamiltonien : après décompression,
c'est un fichier texte simple comportant un sommet par ligne. Chaque entier de
`1` à `490463` apparaît exactement une fois et la somme de chaque paire de
sommets consécutifs, y compris la paire de fermeture entre la dernière et la
première ligne, est une puissance cinquième parfaite.

Ce témoin prouve donc l'existence d'un cycle hamiltonien pour `n = 490463`. À lui
seul, il ne prouve **pas** que `490463` est minimal.

> **Ce n'est pas le témoin minimal pour `k = 5`.** Le résultat de référence est
> `n_min(5) = 246877`, porté par `witness/k5-n246877/`. Le présent bundle est
> conservé parce qu'il reste valide et vérifié de façon indépendante, et parce
> qu'il documente la progression de la recherche.

## Identité canonique du témoin

Le fichier `cycle.txt` non compressé a pour SHA-256 :

`9c9119e5946db4cf3c433e0c90bb6b098afb19f89a2a3fe6f48bfed669cf1efe`

Le cycle est exporté sous forme canonique : rotation plaçant le sommet `1` en
première position, puis choix de l'orientation pour laquelle le plus petit des
deux voisins de `1` est placé en deuxième position.

`cycle.txt.sha256` contient le checksum du témoin **non compressé**.

## Provenance

Le dump final versionné de powercycle est conservé sous la forme
`final.pcdump.json.xz`. Son identifiant sémantique est :

`fa62a76a36f8…`

La chaîne ayant produit ce témoin est la suivante :

- le tour SAT 1 produit un 2-facteur brut de 91 cycles, avec un cycle géant de
  taille 383558 et un résiduel de 106905 sommets ;
- les opérateurs gloutons `three-opt-111` et `three-opt-21`, puis des réparations
  alternées bornées à la profondeur 3, ramènent le facteur à deux cycles : un
  géant de taille 490438 et un satellite de 25 sommets ;
- une réparation alternée de profondeur 4 absorbe ce dernier satellite et
  produit un cycle unique de longueur 490463.

Le dump final est un artefact de provenance. Le certificat mathématique est le
cycle texte lui-même.

## Vérification indépendante

Le dépôt contient deux vérificateurs indépendants du témoin :

- un vérificateur Rust autonome dans `src/verify/rust/` ;
- un vérificateur Python n'utilisant que la bibliothèque standard dans
  `src/verify/python/`.

Ils ne vérifient que la définition mathématique du témoin : permutation de
`1..n` et condition de puissance cinquième pour chaque somme consécutive,
fermeture comprise. Ils n'utilisent ni le solveur SAT, ni le format des dumps, ni
les identifiants d'arêtes, ni l'historique de patching, ni l'implémentation de la
recherche.

Vérifications typiques depuis la racine du dépôt :

```console
xz -dc witness/k5-n490463/cycle.txt.xz > /tmp/powercycle-cycle.txt

sha256sum /tmp/powercycle-cycle.txt

cargo run --release \
  --manifest-path src/verify/rust/Cargo.toml \
  -- 5 490463 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py \
  5 490463 /tmp/powercycle-cycle.txt
```

Le SHA-256 obtenu avec `sha256sum` et celui affiché par le vérificateur Python
doivent être identiques à l'identité canonique indiquée ci-dessus.

## Fichiers

- `cycle.txt.xz` — témoin hamiltonien canonique compressé ;
- `cycle.txt.sha256` — SHA-256 du `cycle.txt` non compressé ;
- `final.pcdump.json.xz` — dump final powercycle compressé avec provenance ;
- `final.pcdump.json.sha256` — SHA-256 du dump final non compressé ;
- `README.md` — version anglaise ;
- `README.fr.md` — ce document.
