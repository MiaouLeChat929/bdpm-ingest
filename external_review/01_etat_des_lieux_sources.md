# 01 — État des lieux des sources BDPM

## 1.1 Présentation générale

La Base de Données Publique des Médicaments (BDPM) est la référence officielle française pour les données ouvertes sur les médicaments. Sa création découle de l'article L. 161-40-1 du Code de la Sécurité Sociale (loi n° 2011-2012 du 29/12/2011). Les données sont alimentées par trois organismes :

- **ANSM** (Agence nationale de sécurité du médicament) : autorisations de mise sur le marché, ruptures de stock, informations de sécurité
- **HAS** (Haute Autorité de Santé) : avis SMR (Service Médical Rendu) et ASMR (Amélioration du Service Médical Rendu)
- **Assurance Maladie** (AMELI) : prix, taux de remboursement, agréments aux collectivités

La licence Etalab 2.0 autorise librement la reproduction, la distribution et l'exploitation, sous trois conditions : ne pas altérer les données, ne pas en dénaturer le sens, et citer la source avec sa date de mise à jour.

---

## 1.2 Inventaire complet des fichiers

La page de téléchargement officielle met à disposition **11 fichiers de données** au format TXT (tabulation-séparé), complétés par un document PDF de spécification du format (v4, 581 Ko).

| # | Fichier | Contenu | Lignes | Colonnes | Taille | Encodage | Fins de ligne | Date MAJ |
|---|---------|---------|--------|----------|--------|----------|---------------|----------|
| 1 | `CIS_bdpm.txt` | Spécialités (médicaments) | 15 848 | 12 | 3,0 Mo | cp1252 | CRLF | 28/04/2026 |
| 2 | `CIS_CIP_bdpm.txt` | Présentations (conditionnements) | 20 903 | 13 | 4,0 Mo | utf-8 | LF | 25/05/2026 |
| 3 | `CIS_COMPO_bdpm.txt` | Compositions (substances) | 32 389 | 8 | 2,6 Mo | cp1252 | CRLF | 28/04/2026 |
| 4 | `CIS_HAS_SMR_bdpm.txt` | Avis SMR de la HAS | 15 257 | 6 | 4,3 Mo | cp1252 | CRLF | 28/04/2026 |
| 5 | `CIS_HAS_ASMR_bdpm.txt` | Avis ASMR de la HAS | 9 906 | 6 | 4,3 Mo | cp1252 | CRLF | 28/04/2026 |
| 6 | `HAS_LiensPageCT_bdpm.txt` | Liens vers avis CT | 10 342 | 2 | 498 Ko | utf-8 (ASCII) | CRLF | 28/04/2026 |
| 7 | `CIS_GENER_bdpm.txt` | Groupes génériques | 10 704 | 5 | 1,2 Mo | cp1252 | CRLF | 28/04/2026 |
| 8 | `CIS_CPD_bdpm.txt` | Conditions prescription/délivrance | 28 151 | 2 | 1,3 Mo | cp1252 | CRLF* | 28/04/2026 |
| 9 | `CIS_CIP_Dispo_Spec.txt` | Ruptures de stock | 766 | 8 | 165 Ko | latin-1 | CRLF | 19/05/2026 |
| 10 | `CIS_MITM.txt` | Médicaments intérêt thérapeutique majeur | 7 711 | 4 | 1,1 Mo | cp1252 | CRLF | 09/03/2026 |
| 11 | `CIS_InfoImportantes.txt` | Informations de sécurité (dynamique) | 10 189 | 4 | 4,0 Mo | utf-8 | LF | Dynamique |

*\* CIS_CPD contient 9 lignes vides parasites dues à des séquences \r\r\n.*

**Volume total** : ~27,6 Mo de données brutes.

### URLs de téléchargement

```
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_CIP_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_COMPO_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_HAS_SMR_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_HAS_ASMR_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/HAS_LiensPageCT_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_GENER_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_CPD_bdpm.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_CIP_Dispo_Spec.txt
https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_MITM.txt
https://base-donnees-publique.medicaments.gouv.fr/download/CIS_InfoImportantes.txt  # Note: /download/ sans /file/
```

**Note critique** : Le fichier `CIS_InfoImportantes.txt` utilise un chemin d'URL différent (`/download/` au lieu de `/download/file/`) et est généré dynamiquement à chaque requête. L'en-tête HTTP `Content-Disposition` retourne un nom horodaté (ex : `CIS_InfoImportantes_20260526111334_bdpm.txt`).

---

## 1.3 Règles de format générales

Ces règles sont documentées dans le PDF de spécification (v4) et confirmées par l'inspection réelle :

| Règle | Valeur |
|-------|--------|
| Format fichier | `.txt` |
| Séparateur de champs | Tabulation (`\t`) |
| Délimiteur de champs | **Aucun** (pas de guillemets) |
| Ligne d'en-tête | **Aucune** (première ligne = données) |
| Séparateur multi-valeurs | Point-virgule `;` (dans certains champs) |
| BOM | **Aucun fichier** ne contient de BOM UTF-8 |
| Octets nuls | **Zéro** dans tous les fichiers |

---

## 1.4 Structure détaillée par fichier

### CIS_bdpm.txt — Spécialités (table maîtresse)

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | Identifiant 8 chiffres (préfixe 6). Clé primaire. |
| 1 | Dénomination | TEXT | Nom du médicament |
| 2 | Forme pharmaceutique | TEXT | comprimé, gélule, solution, etc. |
| 3 | Voies d'administration | TEXT | Multi-valeurs séparées par `;` |
| 4 | Statut administratif AMM | TEXT/ENUM | |
| 5 | Type de procédure AMM | TEXT/ENUM | |
| 6 | État de commercialisation | TEXT/ENUM | |
| 7 | Date d'AMM | DATE | Format **DD/MM/YYYY** |
| 8 | StatutBdm | ENUM | `""` | `"Alerte"` | `"Warning disponibilité"` |
| 9 | Numéro autorisation européenne | TEXT | |
| 10 | Titulaire(s) | TEXT | Multi-valeurs séparées par `;` |
| 11 | Surveillance renforcée | ENUM | `"Oui"` | `"Non"` |

### CIS_CIP_bdpm.txt — Présentations

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Code CIP7 | TEXT | 7 chiffres |
| 2 | Libellé présentation | TEXT | |
| 3 | Statut administratif | TEXT/ENUM | |
| 4 | État de commercialisation | TEXT/ENUM | |
| 5 | Date déclaration commercialisation | DATE | **DD/MM/YYYY** |
| 6 | Code CIP13 | TEXT | 13 chiffres |
| 7 | Agrément aux collectivités | ENUM | `"oui"` | `"non"` | `"inconnu"` |
| 8 | Taux de remboursement | TEXT | Multi-valeurs `;` possible |
| 9 | Prix HT (€) | DECIMAL | Virgule décimale française |
| 10 | Prix public TTC (€) | DECIMAL | Virgule décimale française |
| 11 | Honoraires dispensation (€) | DECIMAL | Virgule décimale française |
| 12 | Indications ouvrant droit au remboursement | TEXT | Uniquement si taux multiples |

### CIS_COMPO_bdpm.txt — Compositions

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Désignation élément pharmaceutique | TEXT | |
| 2 | Code substance | TEXT | |
| 3 | Dénomination substance | TEXT | |
| 4 | Dosage | TEXT | |
| 5 | Référence dosage | TEXT | ex : `"[pour] un comprimé"` |
| 6 | Nature composant | ENUM | `"SA"` (principe actif) | `"FT"` (fraction thérapeutique) |
| 7 | Numéro liaison SA/FT | INTEGER | Lie substances actives et fractions thérapeutiques |

### CIS_HAS_SMR_bdpm.txt — Avis SMR

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Code dossier HAS | TEXT | FK → HAS_LiensPageCT |
| 2 | Motif d'évaluation | TEXT/ENUM | |
| 3 | Date avis | DATE | ⚠️ Format **YYYYMMDD** |
| 4 | Valeur SMR | ENUM | |
| 5 | Libellé SMR | TEXT | |

### CIS_HAS_ASMR_bdpm.txt — Avis ASMR

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Code dossier HAS | TEXT | FK → HAS_LiensPageCT |
| 2 | Motif d'évaluation | TEXT/ENUM | |
| 3 | Date avis | DATE | ⚠️ Format **YYYYMMDD** |
| 4 | Valeur ASMR | ENUM | |
| 5 | Libellé ASMR | TEXT | |

### HAS_LiensPageCT_bdpm.txt — Liens CT

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code dossier HAS | TEXT | Clé primaire |
| 1 | Lien vers avis CT | URL | |

### CIS_GENER_bdpm.txt — Groupes génériques

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Identifiant groupe générique | TEXT | |
| 1 | Libellé groupe générique | TEXT | |
| 2 | Code CIS | TEXT | FK → CIS_bdpm |
| 3 | Type de générique | ENUM | `0`=princeps, `1`=générique, `2`=complémentarité posologique, `4`=substituable |
| 4 | Numéro de tri | INTEGER | |

### CIS_CPD_bdpm.txt — Conditions de prescription/délivrance

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Condition de prescription ou délivrance | TEXT | |

### CIS_CIP_Dispo_Spec.txt — Disponibilité

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Code CIP13 | TEXT | **Peut être vide** (95,4% de vacuité) |
| 2 | Code statut | ENUM | `1`=Rupture, `2`=Tension, `3`=Arrêt, `4`=Remise à disposition |
| 3 | Libellé statut | TEXT | |
| 4 | Date début | DATE | **DD/MM/YYYY** (avant 06/10/2023 = date de MAJ) |
| 5 | Date mise à jour | DATE | **DD/MM/YYYY** |
| 6 | Date remise à disposition | DATE | **DD/MM/YYYY** |
| 7 | Lien page ANSM | URL | |

### CIS_MITM.txt — Médicaments d'intérêt thérapeutique majeur

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Code ATC | TEXT | |
| 2 | Dénomination | TEXT | |
| 3 | Lien BDPM | URL | |

### CIS_InfoImportantes.txt — Informations de sécurité (dynamique)

| Col | Champ | Type | Notes |
|-----|-------|------|-------|
| 0 | Code CIS | TEXT | FK → CIS_bdpm |
| 1 | Date début info sécurité | DATE | **DD/MM/YYYY** |
| 2 | Date fin info sécurité | DATE | **DD/MM/YYYY** |
| 3 | Texte et lien | TEXT/HTML | Contient des balises `<a>` et des entités HTML |

---

## 1.5 Relations entre fichiers

Le **Code CIS** est la clé de jointure universelle, présent dans 10 des 11 fichiers. Le seul fichier sans Code CIS est `HAS_LiensPageCT_bdpm.txt`, qui utilise le **Code dossier HAS**.

```
CIS_bdpm.txt  (RACINE — Code CIS = clé primaire)
├── CIS_CIP_bdpm.txt           (JOIN ON Code CIS)
├── CIS_COMPO_bdpm.txt         (JOIN ON Code CIS)
├── CIS_HAS_SMR_bdpm.txt       (JOIN ON Code CIS)
│   └── HAS_LiensPageCT_bdpm.txt  (JOIN ON Code dossier HAS)
├── CIS_HAS_ASMR_bdpm.txt      (JOIN ON Code CIS)
│   └── HAS_LiensPageCT_bdpm.txt  (JOIN ON Code dossier HAS)
├── CIS_GENER_bdpm.txt         (JOIN ON Code CIS, colonne 2)
├── CIS_CPD_bdpm.txt           (JOIN ON Code CIS)
├── CIS_InfoImportantes.txt    (JOIN ON Code CIS)
├── CIS_CIP_Dispo_Spec.txt     (JOIN ON Code CIS + CIP13 optionnel)
└── CIS_MITM.txt               (JOIN ON Code CIS)
```

### Chemin de jointure secondaire

- `CIS_CIP_Dispo_Spec.txt` peut aussi se joindre à `CIS_CIP_bdpm.txt` sur **Code CIP13**
- `CIS_COMPO_bdpm.txt` : le numéro de liaison SA/FT (colonne 7) relie les substances actives (SA) et fractions thérapeutiques (FT) au sein d'un même Code CIS

---

## 1.6 Métadonnées HTTP du serveur

L'inspection des en-têtes HTTP révèle des contraintes importantes pour la stratégie de collecte :

| En-tête | Valeur | Impact |
|---------|--------|--------|
| `Content-Type` | `application/octet-stream` (10 fichiers), `application/force-download` (InfoImportantes) | Pas de distinction MIME |
| `Cache-Control` | `private, must-revalidate` | Revalidation obligatoire |
| `Pragma` | `no-cache` | Pas de cache |
| `Expires` | `0` | Expiration immédiate |
| **ETag** | **Non fourni** | ❌ Pas de requête conditionnelle If-None-Match |
| **Last-Modified** | **Non fourni** | ❌ Pas de requête conditionnelle If-Modified-Since |
| **Content-Length** | **Non fourni dans HEAD** | ❌ Pas de vérification de taille avant download |
| `Content-Disposition` | Horodaté pour InfoImportantes | Permet de dater la génération |
| `X-Content-Type-Options` | `nosniff` | Sécurité standard |
| `X-Frame-Options` | `DENY` / `SAMEORIGIN` | Sécurité standard |
| `Content-Security-Policy` | Restrictive avec nonce | Sécurité standard |

**Conclusion** : Le serveur ne fournit aucun mécanisme de cache conditionnel. Chaque vérification de mise à jour nécessite un téléchargement complet du fichier.

---

## 1.7 Fréquence et logique de mise à jour

La base est officiellement actualisée **tous les mois**. Toutefois, les dates affichées sur la page de téléchargement révèlent une cadence variable :

| Fichier | Dernière date observée | Interprétation |
|---------|----------------------|----------------|
| CIS_bdpm.txt | 28/04/2026 | Cycle mensuel standard |
| CIS_CIP_bdpm.txt | 25/05/2026 | Rafraîchissement inter-cycle (prix/remboursement) |
| CIS_COMPO_bdpm.txt | 28/04/2026 | Cycle mensuel standard |
| CIS_HAS_SMR_bdpm.txt | 28/04/2026 | Cycle mensuel standard |
| CIS_HAS_ASMR_bdpm.txt | 28/04/2026 | Cycle mensuel standard |
| HAS_LiensPageCT_bdpm.txt | 28/04/2026 | Cycle mensuel standard |
| CIS_GENER_bdpm.txt | 28/04/2026 | Cycle mensuel standard |
| CIS_CPD_bdpm.txt | 28/04/2026 | Cycle mensuel standard |
| CIS_CIP_Dispo_Spec.txt | 19/05/2026 | Mise à jour plus fréquente (ruptures de stock) |
| CIS_MITM.txt | 09/03/2026 | Mise à jour moins fréquente |
| CIS_InfoImportantes.txt | Dynamique | Généré en temps réel à chaque requête |

Il n'existe **aucun flux RSS, aucune API de notification, aucun changelog officiel**.

---

## 1.8 Écosystème existant

Aucun crate Rust n'existe pour le parsing BDPM. Les projets identifiés utilisent d'autres langages :

| Projet | Langage | Type | URL |
|--------|---------|------|-----|
| api-bdpm-graphql | JavaScript | API GraphQL | github.com/axel-op/api-bdpm-graphql |
| medicaments-api | Docker | API REST | github.com/Giygas/medicaments-api |
| api-medicaments | Node.js | API REST (gouvernement) | github.com/betagouv/api-medicaments |
| infomedicament | TypeScript | Application web | github.com/betagouv/infomedicament |
| fr.gouv.medicaments.rest | .NET/C# | API REST | github.com/Gizmo091/fr.gouv.medicaments.rest |

**Opportunité** : Premier crate Rust pour BDPM + première base SQLite pré-construite publiquement.
