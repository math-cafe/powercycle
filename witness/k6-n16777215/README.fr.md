# Témoin powercycle : k = 6, n = 16777215

Un cycle hamiltonien du graphe des sommes de puissances `G_6(16777215)`,
dernière valeur de l'île de `k = 6`.

## Énoncé

`G_6(n)` a pour sommets `1, ..., n` ; deux sommets distincts sont adjacents
lorsque leur somme est une puissance sixième parfaite. Pour `k = 6`,
l'ensemble admissible est `[15748961, 16777215] U [30039769, infini)`.
`G_6(15748961)`, première valeur de l'île, est hamiltonien (power-sum-graphs
1.0.0), de même que `G_6(30039769)`, première valeur du continent
([`../k6-n30039769/`](../k6-n30039769/)). Ce cycle règle l'autre extrémité de
l'île, `n = 16^6 - 1`, le graphe le plus difficile rencontré jusqu'ici : voir
[`../../doc/en/end-of-island.md`](../../doc/en/end-of-island.md).

## Codage et identité

Le cycle est stocké dans le codage par bases de power-sum-graphs
(`doc/formats.md`, section 3) : `4caf6992.bases.xz` contient les `n - 1`
racines sixièmes `b_i` telles que `v_{i+1} = b_i^6 - v_i`, à partir de
`v_1 = 1`. La liste décodée, un sommet par ligne, dans l'ordre canonique, a
pour SHA-256

`4caf699294398fb56bbb91d8d1bf8447892d62937f7de8911c4f28e6110cea73`

## Provenance

Trouvé le 2026-10-22 avec LKH-3.0.14 sur un AMD Ryzen 9 7940HS, alors que
`posum search` et toutes les autres recherches plafonnaient à 6 ou 7 arêtes
manquantes. Le cycle suit un **motif** : pour tout `x` de 1 à 1028254, les
sommets `x + 7360353` et `2056510 - x + 7360353`, extrémités de deux chaînes
forcées dont la somme vaut `16^6`, sont reliés (l'une de ces arêtes est déjà
forcée ; 1028253 sont donc imposées). `provenance.toml` consigne chaque étape
avec le SHA-256 de son résultat :

1. le problème contracté écrit par `posum search 6 16777215`
   (power-sum-graphs 1.0.0), 11390619 nœuds ;
2. le motif imposé en arêtes fixées par `motif`, dans
   [`../../scripts/end-of-island/`](../../scripts/end-of-island/) ;
3. LKH depuis zéro avec [`lkh/motif-a.par`](lkh/motif-a.par), graine 21 : un
   tour à 2 arêtes près au quatrième essai, 1120 s ;
4. LKH depuis ce tour avec [`lkh/motif-r25.par`](lkh/motif-r25.par), graine 25 :
   un cycle hamiltonien au premier essai, 155 s ;
5. le tour développé en cycle de `G_6(16777215)` par `expand`, dans le même
   répertoire.

Les fichiers de paramètres sont ceux des calculs, avec les noms de fichiers de
cet enregistrement. Un second calcul depuis le même tour, avec
[`lkh/motif-r26.par`](lkh/motif-r26.par) (graine 26, premier essai, 170 s), a
donné un autre cycle, rangé dans `ae108229.bases.xz`, d'identité
`ae108229ea29ec360f47f4d1bbcc9a8194c1ed8b8020a2fc9976bc24ba6fcd63` ; il est
accepté par le vérificateur Rust de power-sum-graphs 1.1.0 (liste en clair et
bases) et par `scripts/verify-bases.sh`. Aucun de ces deux
cycles n'a été produit par posum seul ; ce ne sont donc pas des témoins posum.

Le premier cycle est accepté par les deux vérificateurs de power-sum-graphs 1.1.0 (Rust, sur la
liste en clair et sur les bases ; Python, sur la liste en clair) et par
[`scripts/verify-bases.sh`](../../scripts/verify-bases.sh), qui ne partage aucun
code avec eux.

## Vérification

```console
scripts/verify-bases.sh 6 16777215 witness/k6-n16777215/4caf6992.bases.xz
```

affiche `VALID` et l'empreinte ci-dessus.
