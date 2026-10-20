# Certificat powercycle : k = 7, borne inférieure 587761723

Ce répertoire contient la moitié « borne inférieure » d'un résultat pour
`k = 7`, sans témoin : aucun cycle hamiltonien de `G_7(587761723)` n'est
encore connu.

## Énoncé

`G_7(n)` a pour sommets `1, ..., n` ; deux sommets distincts sont adjacents
lorsque leur somme est une puissance septième parfaite. `bands.txt` est un
certificat de degré pour tout `n` de 1 à 587761722 : chacune de ses 18 lignes
donne un intervalle de `n` et un sommet de degré au plus un sur tout cet
intervalle, donc aucun `G_7(n)` avec `3 <= n < 587761723` n'est hamiltonien.
D'où `h(7) >= 587761723 = 19^7 - 18^7/2`, le plus petit ordre admissible pour
`k = 7`.

Le format est celui de power-sum-graphs 1.0.0 (`doc/formats.md`, section 4).
Cette version publie les certificats de `k = 3..6` dans ses bundles, mais pas
celui-ci, puisqu'un bundle exige un témoin.

## Provenance

Produit le 2026-10-20 par `posum bands 7 587761723` (power-sum-graphs 1.0.0,
doi:10.5281/zenodo.23403537), et accepté par ses deux vérificateurs :

```console
python3 verify/python/posum_verify.py bands 7 587761723 bands.txt
verify/rust/target/release/posum-verify bands 7 587761723 bands.txt
```

Un audit indépendant de l'article 1 a reconstruit les mêmes 18 lignes avec son
propre code.

## Fichiers

- `bands.txt` — le certificat de degré ;
- `bands.txt.sha256` — son SHA-256 ;
- `README.md` — version anglaise ;
- `README.fr.md` — ce document.
