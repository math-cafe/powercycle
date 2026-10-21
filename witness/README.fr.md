# Témoins

Ce répertoire conserve des résultats scientifiques versionnés. Il est placé sous
CC0 1.0 Universal ([LICENSE](LICENSE)) ; le reste du dépôt est sous double
licence MIT OR Apache-2.0.

Pour des entiers `k >= 2` et `n >= 1`, le graphe de sommes de puissances
`G_k(n)` a pour sommets `{1, ..., n}` et relie deux sommets distincts dont la
somme est une puissance `k`-ième parfaite. `n_min(k)` désigne le plus petit `n`
pour lequel `G_k(n)` possède un cycle hamiltonien.

## Résultats

| k | n_min | forme close | trouvé par | bundle |
|---|---|---|---|---|
| 2 | 32 | — | classique | non inclus ; voir ci-dessous |
| 3 | **473** | `9^3 - 8^3/2` | G. Resta, vers 2005 | [`k3-n473/`](k3-n473/) |
| 4 | **9641** | `11^4 - 10^4/2` | R. Tauraso, 2006 | [`k4-n9641/`](k4-n9641/) |
| 5 | **246877** | `13^5 - 12^5/2` | ce projet | [`k5-n246877/`](k5-n246877/) |
| 6 | **15748961** | `17^6 - 16^6/2` | ce projet | [`k6-n15748961/`](k6-n15748961/) |
| 7 | `>= 587761723` | `19^7 - 18^7/2` | — | pas encore de témoin ; borne inférieure dans [`k7-n587761723/`](k7-n587761723/) |

[`k5-n490463/`](k5-n490463/) conserve un témoin antérieur, non minimal, pour
`k = 5`. Il reste valide et vérifié de façon indépendante, et documente la
progression de la recherche.

[`k3-n473-800/`](k3-n473-800/) contient un cycle hamiltonien de `G_3(n)` pour
chaque `n` de 473 à 800, à l'appui de la conjecture selon laquelle tout `n`
admissible est hamiltonien pour `k >= 3`.

[`k4-n18193/`](k4-n18193/) contient un cycle hamiltonien de `G_4(18193)`,
première valeur du continent de `k = 4` : avec le bundle d'île de
power-sum-graphs 1.0.0, il montre que l'hamiltonicité n'est pas monotone en
`n`.

[`k6-n30039769/`](k6-n30039769/) contient un cycle hamiltonien de
`G_6(30039769)`, première valeur du continent de `k = 6`, dans le codage par
bases. Voir [`../doc/en/continent.md`](../doc/en/continent.md) pour les autres
calculs sur le continent.

Pour `k = 2`, la réponse relève du square-sum problem classique et est déjà
connue ; voir l'OEIS [A071984](https://oeis.org/A071984) et
[A272259](https://oeis.org/A272259), dont les lignes commencent à `n = 32`.

La suite `n_min(k)` est l'OEIS [A115418](https://oeis.org/A115418), soumise par Roberto Tauraso en 2006 avec
les termes 2, 32, 473, 9641. La boucle de cubes de longueur 473 est due à
Giovanni Resta, qui a ramené la recherche à un problème du voyageur de commerce
(Carlos Rivera, [Puzzle 311](http://www.primepuzzles.net/puzzles/puzz_311.htm) : Sum to a cube). Aucune preuve publiée de la minimalité de
473 et 9641 n'a été trouvée ; les bundles `k3` et `k4` en fournissent une.
`246877` et `15748961` prolongent la suite et sont, à notre connaissance,
nouveaux.

## Contenu d'un bundle

Un bundle revendiquant un `n` minimal porte les **deux moitiés** de l'énoncé.

**L'existence** est `cycle.txt`, la simple liste des entiers `1..n` en ordre
cyclique. Il est exporté sous forme canonique : rotation plaçant le sommet `1`
en premier, orientation plaçant le plus petit voisin de `1` en second.

**La borne inférieure** est `bands.txt`, la sortie de `pc-scan --bands` sur
`1..n-1`. Elle découpe toutes les valeurs inférieures en bandes, chacune nommant
un sommet de degré au plus un — qu'aucun cycle hamiltonien ne peut emprunter —
sur toute sa plage. Une douzaine de lignes couvrent des millions de valeurs, car
le degré d'un sommet ne décroît jamais quand `n` augmente : égal aux deux bouts
d'une bande, il l'est partout entre les deux.

Jusqu'à la version 0.1.0, la borne inférieure était `scan.txt`, une ligne par
`n`. Sa taille croissait linéairement avec `n` — 33 Go à `k = 7` — et il reste
disponible aux tags `witness-k*`.

`final.pcdump.json` n'est que de la provenance. Le certificat mathématique est
le cycle brut.

Chaque artefact est stocké compressé à côté du SHA-256 de sa forme **non
compressée** : le checksum identifie ainsi le contenu mathématique, et non une
compression particulière.

## La borne inférieure

Le sommet `v` est adjacent à `p - v` exactement lorsque `p` est une puissance
`k`-ième parfaite vérifiant `v + 1 <= p <= v + n` et `p != 2v`, le cas exclu
étant une boucle. Compter ces puissances donne directement le degré de `v`, sans
construire aucun graphe.

Les sommets contraignants sont les positions de boucle elles-mêmes. Pour `m`
pair, le sommet `v = m^k/2` se situe dans l'intervalle engendré par `m^k` mais ne
peut pas l'utiliser : il lui faut donc la puissance suivante `(m+1)^k` dans sa
fenêtre.

| k | sommet contraignant | attend |
|---|---|---|
| 3 | `256 = 8^3/2` | `9^3 = 729` |
| 4 | `5000 = 10^4/2` | `11^4 = 14641` |
| 5 | `124416 = 12^5/2` | `13^5 = 371293` |
| 6 | `8388608 = 16^6/2` | `17^6 = 24137569` |
| 7 | `306110016 = 18^7/2` | `19^7 = 893871739` |

Cette condition est **nécessaire, pas suffisante**. `k = 2` est le contre-exemple
à garder en tête : la borne analogue vaut `31`, et `G_2(31)` passe toutes les
conditions nécessaires implémentées dans `pc-scan`, alors que le minimum réel est
`32`.

Pour `k = 3, 4, 5, 6`, les deux moitiés se rejoignent, car un témoin existe au
tout premier `n` que la borne autorise. C'est ce qui fait de ces valeurs des
théorèmes et non de simples bornes. Pour `k >= 7`, seule la borne inférieure est
établie.

Voir [`k5-n246877/README.fr.md`](k5-n246877/README.fr.md) pour la discussion
complète.

## Vérifier un bundle

Depuis la racine du dépôt, avec `K` et `N` fixés aux paramètres du bundle :

```console
xz -dc witness/k$K-n$N/cycle.txt.xz > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt          # doit correspondre à cycle.txt.sha256

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- $K $N /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py $K $N /tmp/powercycle-cycle.txt
```

Les deux vérificateurs vivent hors du graphe de dépendances de powercycle. Ils
ne prennent que `k`, `n` et une liste candidate, et vérifient que c'est une
permutation de `1..n` dont les sommes consécutives, fermeture comprise, sont des
puissances `k`-ièmes parfaites. Aucun ne lit de dump, d'identifiant d'arête,
d'historique de patch ni quoi que ce soit du solveur.

Pour un bundle qui revendique la minimalité, la borne inférieure :

```console
xz -dc witness/k$K-n$N/bands.txt.xz > /tmp/powercycle-bands.txt
sha256sum /tmp/powercycle-bands.txt          # doit correspondre à bands.txt.sha256

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  --bin powercycle-verify-bands -- $K $N /tmp/powercycle-bands.txt

python src/verify/python/verify-bands.py $K $N /tmp/powercycle-bands.txt
```

Ces deux-là vérifient que les bandes couvrent `1..n-1` sans trou et, aux deux
bouts de chaque bande, recomptent les puissances dans la fenêtre de son sommet
pour confirmer le degré annoncé. Comme les vérificateurs de cycle, ils
n'utilisent que `k`, `n` et le fichier.
