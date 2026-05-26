# Inventaire Complet des Fichiers BDPM

> Document de reference sur l'ensemble des fichiers publiés par la Base de Données Publique des Médicaments (BDPM).
> Source : https://base-donnees-publique.medicaments.gouv.fr/telechargement
> Dernière vérification : 26 mai 2026

---

## 1. Vue d'ensemble

La BDPM publie **11 fichiers de données** au format texte tabulé (TSV), accompagnés d'un document de spécification PDF et d'une licence d'utilisation. Tous les fichiers sont accessibles en téléchargement libre et gratuit, sous licence Licence Ouverte Etalab 2.0.

### URL de base

```
https://base-donnees-publique.medicaments.gouv.fr
```

Les fichiers de données résident sous deux chemins distincts :
- `/download/file/` — 10 fichiers pré-générés (mise à jour mensuelle ou ponctuelle)
- `/download/` — 1 fichier généré dynamiquement (`CIS_InfoImportantes.txt`)

---

## 2. Inventaire détaillé

### 2.1 CIS_bdpm.txt — Fichier des spécialités

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_bdpm.txt` |
| **Description** | Fichier central des spécialités pharmaceutiques |
| **Date de mise à jour** | 28/04/2026 |
| **Taille** | 3 091 Ko (3 164 943 octets) |
| **Lignes de données** | 15 848 |
| **Nombre de champs** | 12 (11 tabulations) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |
| **Clé primaire** | Code CIS (champ 1) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Identifiant unique de la spécialité | `61266250` |
| 2 | Dénomination | Texte | Nom du médicament | `A 313 200 000 UI POUR CENT, pommade` |
| 3 | Forme pharmaceutique | Texte | Forme galénique | `pommade`, `comprimé` |
| 4 | Voies d'administration | Texte (séparateur `;`) | Voies d'administration possibles | `cutanée;orale;sublinguale` |
| 5 | Statut administratif AMM | Texte | Statut de l'autorisation | `Autorisation active` |
| 6 | Type de procédure AMM | Texte | Type de procédure d'autorisation | `Procédure nationale` |
| 7 | État de commercialisation | Texte | Statut commercial | `Commercialisée` |
| 8 | Date d'AMM | Date DD/MM/YYYY | Date d'autorisation de mise sur le marché | `12/03/1998` |
| 9 | Statut BDM | Texte | Alerte ou warning | `Alerte`, `Warning disponibilité`, vide |
| 10 | Numéro autorisation européenne | Texte | Numéro EMA (vide si procédure nationale) | souvent vide |
| 11 | Titulaire(s) | Texte (séparateur `;`) | Titulaire(s) de l'AMM | `PHARMA DEVELOPPEMENT` |
| 12 | Surveillance renforcée | Énuméré | `Oui` ou `Non` | `Non` |

**Valeurs énumérées observées :**
- Statut BDM (champ 9) : `""`, `"Alerte"`, `"Warning disponibilité"`
- Surveillance renforcée (champ 12) : `"Oui"`, `"Non"`
- 2 205 lignes contiennent des points-virgules dans les champs (voies d'administration, titulaires)

---

### 2.2 CIS_CIP_bdpm.txt — Fichier des présentations

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_CIP_bdpm.txt` |
| **Description** | Fichier des présentations (conditionnements) des spécialités |
| **Date de mise à jour** | 25/05/2026 |
| **Taille** | 4 054 Ko (4 151 119 octets) |
| **Lignes de données** | 20 903 |
| **Nombre de champs** | 13 (12 tabulations) |
| **Encodage** | UTF-8 ⚠️ **Seul fichier en UTF-8** |
| **Fins de ligne** | LF (`\n`) ⚠️ **Seul fichier en LF** |
| **Clé étrangère** | Code CIS (champ 1) → CIS_bdpm.txt |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère vers CIS_bdpm | `60002283` |
| 2 | Code CIP7 | Texte 7 chiffres | Code CIP 7 positions | `4949729` |
| 3 | Libellé présentation | Texte | Description du conditionnement | `plaquette(s) PVC PVDC aluminium de 30 comprimé(s)` |
| 4 | Statut administratif | Texte | Statut de la présentation | `Présentation active` |
| 5 | État de commercialisation | Texte | Statut commercial | `Déclaration de commercialisation` |
| 6 | Date de déclaration | Date DD/MM/YYYY | Date de déclaration | `16/03/2011` |
| 7 | Code CIP13 | Texte 13 chiffres | Code CIP 13 positions | `3400949497294` |
| 8 | Agrément collectivités | Énuméré | `oui`, `non`, `inconnu` | `oui` |
| 9 | Taux de remboursement | Texte | Pourcentage(s) de remboursement | `100%`, `65 %` |
| 10 | Prix en euro | Décimal (virgule) | Prix hors taxes | `24,34` |
| 11 | Prix public France | Décimal (virgule) | Prix TTC | `25,36` |
| 12 | Honoraires dispensation | Décimal (virgule) | Honoraires du pharmacien | `1,02` |
| 13 | Indications remboursement | Texte (HTML) | Indications ouvrant droit au remboursement | Peut contenir du HTML `<br>` |

**Quirks détectés :**
- ⚠️ **20 089 lignes sur 20 903** (96%) ont des tabulations de fin (trailing tabs), créant un champ 14 vide fantôme
- Le taux de remboursement (champ 9) est **inconsistant** : parfois avec espace (`65 %`), parfois sans (`65%`)
- Le champ 13 contient du HTML brut (`<br>`, `<a href>`) avec des entités Windows-1252 comme `¿` (0xBF, souvent un apostrophe mal décodé)
- Les prix utilisent la **virgule** comme séparateur décimal français
- 814 lignes contiennent du HTML dans le champ des indications

---

### 2.3 CIS_COMPO_bdpm.txt — Fichier des compositions

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_COMPO_bdpm.txt` |
| **Description** | Composition des spécialités (substances actives et fractions thérapeutiques) |
| **Date de mise à jour** | 28/04/2026 |
| **Taille** | 2 670 Ko (2 733 708 octets) |
| **Lignes de données** | 32 389 |
| **Nombre de champs** | 8 (7 tabulations) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |
| **Clé étrangère** | Code CIS (champ 1) → CIS_bdpm.txt |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère vers CIS_bdpm | `60002283` |
| 2 | Désignation élmt pharmaceutique | Texte | Forme pharmaceutique de la composition | `comprimé` |
| 3 | Code substance | Texte 5 chiffres | Identifiant de la substance | `42215` |
| 4 | Dénomination substance | Texte | Nom de la substance | `ANASTROZOLE` |
| 5 | Dosage | Texte | Dosage de la substance | `1,00 mg` |
| 6 | Référence dosage | Texte | Référence du dosage | `un comprimé` |
| 7 | Nature du composant | Énuméré | `SA` (substance active) ou `FT` (fraction thérapeutique) | `SA` |
| 8 | Numéro de liaison SA/FT | Entier | Lien entre substance active et fraction thérapeutique | `1` |

**Valeurs énumérées observées :**
- Nature du composant (champ 7) : `SA`, `FT` — aucune autre valeur détectée

**Intégrité référentielle :**
- 15 846 CIS codes uniques → couvre quasiment 100% du fichier central (15 848)
- Aucun enregistrement orphelin détecté

---

### 2.4 CIS_HAS_SMR_bdpm.txt — Fichier des avis SMR

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_HAS_SMR_bdpm.txt` |
| **Description** | Avis du Service Médical Rendu (SMR) de la Haute Autorité de Santé |
| **Date de mise à jour** | 28/04/2026 |
| **Taille** | 4 388 Ko (4 493 611 octets) |
| **Lignes de données** | 15 257 |
| **Nombre de champs** | 6 (5 tabulations) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |
| **Clé étrangère** | Code CIS (champ 1), Code dossier HAS (champ 2) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère vers CIS_bdpm | `61175466` |
| 2 | Code dossier HAS | Texte | Identifiant du dossier HAS | `CT-21783` |
| 3 | Motif d'évaluation | Texte | Raison de l'évaluation | `Inscription (CT)` |
| 4 | Date de l'avis | Date YYYYMMDD ⚠️ | Date de l'avis (format différent !) | `20260401` |
| 5 | Valeur du SMR | Texte | Niveau du SMR | `Important`, `Modéré`, `Insuffisant` |
| 6 | Libellé du SMR | Texte (HTML possible) | Justification détaillée | Peut contenir `<br>`, `•` |

**Quirks détectés :**
- ⚠️ Le format de date (champ 4) est **YYYYMMDD**, contrairement au DD/MM/YYYY de CIS_bdpm.txt
- Le champ 6 contient fréquemment du HTML (`<br>`, `•` en CP1252 = `\x95`)
- Bytes CP1252 fréquents : `\x92` (22 253 occurrences = apostrophe française), `\x95` (4 159 = puces), `\x85` (12 = points de suspension)
- 1 975 lignes contiennent du HTML dans le libellé
- **2 806 CIS codes orphelins** (n'existent pas dans CIS_bdpm.txt) — probablement des spécialités retirées du marché depuis plus de 5 ans

---

### 2.5 CIS_HAS_ASMR_bdpm.txt — Fichier des avis ASMR

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_HAS_ASMR_bdpm.txt` |
| **Description** | Avis d'Amélioration du Service Médical Rendu (ASMR) de la HAS |
| **Date de mise à jour** | 28/04/2026 |
| **Taille** | 4 375 Ko (4 480 434 octets) |
| **Lignes de données** | 9 906 |
| **Nombre de champs** | 6 (5 tabulations) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère | `61175466` |
| 2 | Code dossier HAS | Texte | Identifiant du dossier HAS | `CT-21783` |
| 3 | Motif d'évaluation | Texte | Raison de l'évaluation | `Inscription (CT)` |
| 4 | Date de l'avis | Date YYYYMMDD | Même format que SMR | `20260401` |
| 5 | Valeur de l'ASMR | Énuméré (I-V) | Niveau d'amélioration | `V` |
| 6 | Libellé de l'ASMR | Texte (HTML possible) | Justification détaillée | Peut contenir HTML |

**Quirks détectés :**
- Bytes CP1252 : `\x92` (29 704 occurrences !), `\x95` (7 163), `\x96` (35), `\x85` (6), `\x99` (3)
- 2 060 lignes avec contenu HTML
- **1 567 CIS codes orphelins** par rapport à CIS_bdpm.txt
- Le fichier ASMR est le plus riche en bytes CP1252 spécifiques

---

### 2.6 HAS_LiensPageCT_bdpm.txt — Liens vers les avis HAS

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/HAS_LiensPageCT_bdpm.txt` |
| **Description** | Liens vers les pages d'avis de la Commission de la Transparence |
| **Date de mise à jour** | 28/04/2026 |
| **Taille** | 499 Ko (510 490 octets) |
| **Lignes de données** | 10 342 |
| **Nombre de champs** | 2 (1 tabulation) |
| **Encodage** | ASCII pur |
| **Fins de ligne** | CRLF (`\r\n`) |
| **Clé primaire** | Code dossier HAS (champ 1) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code dossier HAS | Texte | Identifiant du dossier | `CT-21584` |
| 2 | Lien page CT | URL | Lien vers la page d'avis | `https://www.has-sante.fr/jcms/p_3961577` |

**Intégrité référentielle :**
- 10 327 codes HAS uniques
- 1 017 codes HAS dans CIS_HAS_SMR n'ont pas de lien correspondant
- 1 348 codes HAS dans CIS_HAS_ASMR n'ont pas de lien correspondant
- Les codes orphelins correspondent probablement à des avis anciens ou non numérisés

---

### 2.7 CIS_GENER_bdpm.txt — Fichier des groupes génériques

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_GENER_bdpm.txt` |
| **Description** | Appartenance des spécialités aux groupes génériques |
| **Date de mise à jour** | 28/04/2026 |
| **Taille** | 1 188 Ko (1 215 963 octets) |
| **Lignes de données** | 10 704 |
| **Nombre de champs** | 5 (4 tabulations) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Identifiant groupe générique | Entier | Numéro du groupe | `1` |
| 2 | Libellé groupe générique | Texte | Nom du groupe | `CIMETIDINE 200 mg - TAGAMET 200 mg, comprimé pelliculé` |
| 3 | Code CIS | Entier 8 chiffres | Spécialité membre du groupe | `65383183` |
| 4 | Type de générique | Énuméré | Rôle dans le groupe | `0`, `1`, `2`, `4` |
| 5 | Numéro de tri | Entier | Ordre d'affichage | `1` |

**Valeurs du type de générique (champ 4) :**

| Valeur | Signification |
|--------|---------------|
| `0` | Princeps (spécialité de référence) |
| `1` | Générique |
| `2` | Complémentarité posologique |
| `4` | Substituable |

**Intégrité référentielle :**
- 10 628 CIS codes uniques
- **2 503 CIS codes orphelins** par rapport à CIS_bdpm.txt — probablement des spécialités anciennes

---

### 2.8 CIS_CPD_bdpm.txt — Conditions de prescription et délivrance

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_CPD_bdpm.txt` |
| **Description** | Conditions de prescription et de délivrance des spécialités |
| **Date de mise à jour** | 28/04/2026 |
| **Taille** | 1 283 Ko (1 313 810 octets) |
| **Lignes de données** | 28 154 |
| **Nombre de champs** | 2 (1 tabulation) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère | `63852237` |
| 2 | Condition de prescription/délivrance | Texte | Description de la condition | `réservé à l'usage professionnel DENTAIRE` |

**Remarques :**
- Un même CIS peut apparaître plusieurs fois (plusieurs conditions possibles)
- 141 occurrences de `\x92` (apostrophe CP1252) dans les conditions
- 12 492 CIS codes uniques — couverture partielle du fichier central
- Aucun enregistrement orphelin par rapport à CIS_bdpm.txt

---

### 2.9 CIS_CIP_Dispo_Spec.txt — Ruptures de stock

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_CIP_Dispo_Spec.txt` |
| **Description** | État de disponibilité des spécialités (ruptures, tensions, remises à disposition) |
| **Date de mise à jour** | 19/05/2026 |
| **Taille** | 165 Ko (168 769 octets) |
| **Lignes de données** | 766 |
| **Nombre de champs** | 8 (7 tabulations) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère | `61436304` |
| 2 | Code CIP13 | Texte 13 chiffres | Code CIP13 (vide si toutes présentations) | peut être vide |
| 3 | Code statut | Énuméré | Code du statut | `1`, `2`, `3`, `4` |
| 4 | Libellé du statut | Texte | Description du statut | `Rupture de stock` |
| 5 | Date de début | Date DD/MM/YYYY | Date de début de l'événement | `14/04/2026` |
| 6 | Date de mise à jour | Date DD/MM/YYYY | Date de dernière mise à jour | `18/05/2026` |
| 7 | Date de remise à disposition | Date DD/MM/YYYY | Date de fin (vide si en cours) | peut être vide |
| 8 | Lien page ANSM | URL | Lien vers la page ANSM | `https://ansm.sante.fr/...` |

**Valeurs du statut (champs 3-4) :**

| Code | Libellé |
|------|---------|
| `1` | Rupture de stock |
| `2` | Tension d'approvisionnement |
| `3` | Arrêt de commercialisation |
| `4` | Remise à disposition / remise à disposition |

**Quirk :** Le libellé pour le code 4 existe en deux variantes : `Remise à disposition` (majuscule) et `remise à disposition` (minuscule) — incohérence de capitalisation.

**Fréquence de mise à jour :** Ce fichier est mis à jour plus fréquemment que le cycle mensuel (19/05/2026 vs 28/04/2026 pour les autres).

---

### 2.10 CIS_MITM.txt — Médicaments d'intérêt thérapeutique majeur

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/file/CIS_MITM.txt` |
| **Description** | Médicaments identifiés comme d'intérêt thérapeutique majeur |
| **Date de mise à jour** | 09/03/2026 |
| **Taille** | 1 110 Ko (1 136 234 octets) |
| **Lignes de données** | 7 711 |
| **Nombre de champs** | 4 (3 tabulations) |
| **Encodage** | Windows-1252 (CP1252) |
| **Fins de ligne** | CRLF (`\r\n`) |

**Structure des champs :**

| # | Nom du champ | Type | Description | Exemple |
|---|-------------|------|-------------|---------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère | `60003620` |
| 2 | Code ATC | Texte | Classification ATC | `R03BA01` |
| 3 | Dénomination | Texte | Nom du médicament | `BECLOSPIN 800 microgrammes/2ml...` |
| 4 | Lien BDPM | URL | Lien vers la fiche BDPM | `http://base-donnees-publique.medicaments.gouv.fr/extrait.php?specid=60003620` |

**Remarques :**
- Les liens (champ 4) utilisent l'ancien format `extrait.php?specid=` qui redirige (301) vers `/medicament/{cis}/extrait`
- 7 711 CIS codes uniques, aucun orphelin par rapport au fichier central

---

### 2.11 CIS_InfoImportantes.txt — Informations de sécurité

| Propriété | Valeur |
|-----------|--------|
| **URL** | `/download/CIS_InfoImportantes.txt` ⚠️ **Chemin différent** |
| **Description** | Informations importantes de sécurité sur les médicaments |
| **Date de mise à jour** | Généré dynamiquement à la demande |
| **Encodage** | Variable |

**Structure des champs :**

| # | Nom du champ | Type | Description |
|---|-------------|------|-------------|
| 1 | Code CIS | Entier 8 chiffres | Clé étrangère |
| 2 | Date de début | Date DD/MM/YYYY | Début de l'information |
| 3 | Date de fin | Date DD/MM/YYYY | Fin de l'information (vide si en cours) |
| 4 | Texte et lien | Texte (HTML) | Contenu de l'information de sécurité |

**Quirks critiques :**
- ⚠️ Le chemin d'accès est **différent** : `/download/CIS_InfoImportantes.txt` (sans `/file/`)
- Le fichier est **généré dynamiquement** — le nom inclut un timestamp : `CIS_InfoImportantes_20260526114141_bdpm.txt`
- Le contenu peut être **vide** (0 octets) si aucune information de sécurité n'est en cours
- Ce fichier a une fréquence de mise à jour **indépendante** du cycle mensuel

---

## 3. Fichiers de documentation

| Fichier | URL | Taille | Description |
|---------|-----|--------|-------------|
| Spécification v4 | `/download/file/Contenu_et_format_des_fichiers_telechargeables_dans_la_BDM_v4.pdf` | 581 Ko | Description officielle des formats et liens entre fichiers |
| Licence | `/docs/telechargement/licence_bdpm.pdf` | 528 Ko | Licence Ouverte Etalab 2.0 |

---

## 4. Synthèse des métadonnées

| Fichier | Lignes | Champs | Encodage | Fins de ligne | Taille |
|---------|--------|--------|----------|---------------|--------|
| CIS_bdpm.txt | 15 848 | 12 | CP1252 | CRLF | 3,1 Mo |
| CIS_CIP_bdpm.txt | 20 903 | 13 | **UTF-8** | **LF** | 4,0 Mo |
| CIS_COMPO_bdpm.txt | 32 389 | 8 | CP1252 | CRLF | 2,7 Mo |
| CIS_HAS_SMR_bdpm.txt | 15 257 | 6 | CP1252 | CRLF | 4,4 Mo |
| CIS_HAS_ASMR_bdpm.txt | 9 906 | 6 | CP1252 | CRLF | 4,4 Mo |
| HAS_LiensPageCT_bdpm.txt | 10 342 | 2 | ASCII | CRLF | 510 Ko |
| CIS_GENER_bdpm.txt | 10 704 | 5 | CP1252 | CRLF | 1,2 Mo |
| CIS_CPD_bdpm.txt | 28 154 | 2 | CP1252 | CRLF | 1,3 Mo |
| CIS_CIP_Dispo_Spec.txt | 766 | 8 | CP1252 | CRLF | 169 Ko |
| CIS_MITM.txt | 7 711 | 4 | CP1252 | CRLF | 1,1 Mo |
| CIS_InfoImportantes.txt | variable | 4 | variable | variable | variable |

**Total estimé : ~145 000 enregistrements, ~22 Mo de données brutes**
