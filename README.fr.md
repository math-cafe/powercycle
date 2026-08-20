# powercycle

Recherche, transformation, export et vérification indépendante de cycles
hamiltoniens de sommes de puissances.

Pour des entiers `k >= 2` et `n >= 1`, le **graphe de sommes de puissances**
`G_k(n)` a pour sommets `{1, ..., n}` et relie deux sommets distincts `x != y`
dès que `x + y` est une puissance `k`-ième parfaite. Un cycle hamiltonien de
`G_k(n)` est un arrangement des entiers de `1` à `n` en cercle dans lequel
**chaque paire voisine somme à une puissance `k`-ième parfaite**. Pour `k = 2`,
c'est le square-sum problem classique.

`n_min(k)` désigne le plus petit `n` pour lequel un tel arrangement existe.

## Résultats

| k | n_min | forme close | témoin |
|---|---|---|---|
| 2 | 32 | — | résultat connu ([A272259](https://oeis.org/A272259)) |
| 3 | **473** | `9^3 - 8^3/2` | [`witness/k3-n473/`](witness/k3-n473/) |
| 4 | **9641** | `11^4 - 10^4/2` | [`witness/k4-n9641/`](witness/k4-n9641/) |
| 5 | **246877** | `13^5 - 12^5/2` | [`witness/k5-n246877/`](witness/k5-n246877/) |
| 6 | `>= 15748961` | `17^6 - 16^6/2` | ouvert |
| 7 | `>= 587761723` | `19^7 - 18^7/2` | ouvert |

Chaque valeur en gras est un théorème, pas une borne : le bundle porte un cycle
hamiltonien **et** un certificat écartant tout `n` plus petit. Voir
[`witness/README.fr.md`](witness/README.fr.md) pour l'argument et ses limites —
en particulier pourquoi `k = 2` ne suit pas le même motif.

## Vérifier un résultat

Rien ici ne demande de faire confiance à la recherche. Depuis un clone neuf :

```console
zstd -dc witness/k5-n246877/cycle.txt.zst > /tmp/cycle.txt

sha256sum /tmp/cycle.txt
cat witness/k5-n246877/cycle.txt.sha256

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 5 246877 /tmp/cycle.txt

python src/verify/python/verify-cycle.py 5 246877 /tmp/cycle.txt
```

Les deux vérificateurs sont délibérément hors du graphe de dépendances de
powercycle. L'un est un crate Rust autonome **sans aucune dépendance** ; l'autre
n'utilise que la bibliothèque standard de Python. Chacun reçoit `k`, `n` et une
liste candidate, et vérifie que c'est une permutation de `1..n` dont les sommes
consécutives, fermeture comprise, sont des puissances `k`-ièmes parfaites. Aucun
ne lit de dump, d'identifiant d'arête, d'historique de patch ni quoi que ce soit
produit par le solveur.

Pour rejouer la borne inférieure :

```console
zstd -dc witness/k5-n246877/scan.txt.zst > /tmp/scan.txt
cargo run --release --bin pc-scan -- 5 1 246877 | diff - /tmp/scan.txt
```

## Outils

| binaire | rôle |
|---|---|
| `pc-scan` | décide, sur un intervalle de `n`, quelles valeurs une condition nécessaire écarte |
| `pc-search` | recherche SAT de 2-facteurs, avec élimination de sous-tours et réparation en place |
| `pc-patch` | pousse plus loin un dump stocké, hors du run qui l'a produit |
| `pc-export` | extrait un témoin canonique d'un dump à cycle unique |

Usage typique :

```console
cargo build --release

./target/release/pc-scan --compact 5 1 250000     # où chercher
./target/release/pc-search 5 246877 10            # chercher là
./target/release/pc-export <dump>.pcdump.json     # certifier
```

Un verdict `CANDIDATE` de `pc-scan` signifie qu'aucune obstruction implémentée
n'a été trouvée. Il ne prouve pas qu'un cycle hamiltonien existe.

## Approche

Le cycle hamiltonien est relâché en **2-facteur** : une variable SAT par arête du
graphe, contrainte pour que chaque sommet ait un degré sélectionné exactement
égal à deux. Un modèle est alors une union disjointe de cycles couvrant tous les
sommets, et il est hamiltonien exactement lorsqu'il n'y a qu'un cycle. Les
coupes d'élimination de sous-tours sont ajoutées paresseusement et sous budget
par tour, et chaque facteur brut est poussé vers le cycle unique par des
opérateurs de réparation combinatoire — `three-opt-111`, `three-opt-21` et une
réparation alternée bornée en profondeur.

Voir [`doc/en/algorithms.md`](doc/en/algorithms.md) pour les algorithmes,
[`doc/en/architecture.md`](doc/en/architecture.md) pour la structure des crates
et [`doc/en/dump-format.md`](doc/en/dump-format.md) pour le format d'échange.

## Développement

```console
cargo test --workspace              # suite rapide
cargo test --release -- --ignored   # contrôles croisés longs, environ une minute
./scripts/check.fish                # tout, plus la re-vérification de
                                    # chaque témoin versionné
```

`scripts/check.fish` est le garde-fou avant publication : formatage, lints,
tests, build release, puis pour chaque bundle de `witness/` il recalcule les
checksums, lance les deux vérificateurs indépendants et rejoue le certificat de
borne inférieure.

## Licence

Le code source est sous double licence [MIT](LICENSE-MIT) OU
[Apache-2.0](LICENSE-APACHE), au choix.

`witness/` est de la donnée, pas du logiciel, et est placé sous
[CC0 1.0](witness/LICENSE). Voir [LICENSE](LICENSE).
