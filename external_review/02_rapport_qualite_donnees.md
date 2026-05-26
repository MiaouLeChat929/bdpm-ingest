# 02 — Rapport de qualité des données BDPM

Ce document présente l'ensemble des problèmes de qualité, quirks et anomalies identifiés lors de l'inspection exhaustive des 11 fichiers BDPM. Chaque problème est documenté avec son impact concret sur le parsing et des recommandations de traitement.

---

## 2.1 Encodage : trois normes cohabitent

### Constat

C'est la découverte la plus critique de l'étude. Les 11 fichiers utilisent **trois encodages différents**, sans aucun indicateur dans les fichiers eux-mêmes (pas de BOM, pas de déclaration, pas d'en-tête).

| Encodage | Fichiers | Nombre |
|----------|----------|--------|
| **cp1252** (Windows-1252) | CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_ASMR, CIS_HAS_SMR, CIS_MITM | 7 |
| **latin-1** (ISO-8859-1) | CIS_CIP_Dispo_Spec | 1 |
| **utf-8** | CIS_CIP_bdpm, CIS_InfoImportantes, HAS_LiensPageCT | 3 |

### Pourquoi la distinction cp1252 vs latin-1 est critique

cp1252 définit des caractères dans la plage 0x80-0x9F que latin-1 laisse non définis (réservé aux caractères de contrôle C1). En pratique :

| Octet | latin-1 | cp1252 | Occurrences dans BDPM |
|-------|---------|--------|----------------------|
| 0x92 | Contrôle C1 | Apostrophe courbe `'` (U+2019) | 52 168 au total |
| 0x85 | Contrôle C1 | Points de suspension `…` (U+2026) | 18 |

**Décoder un fichier cp1252 en latin-1 produit des caractères de contrôle invisibles au lieu des apostrophes attendues.** C'est le bug le plus probable si l'encodage est mal choisi.

### Comptage des smart quotes (0x92) par fichier

| Fichier | Occurrences de 0x92 |
|---------|---------------------|
| CIS_HAS_ASMR_bdpm.txt | 29 704 |
| CIS_HAS_SMR_bdpm.txt | 22 253 |
| CIS_CPD_bdpm.txt | 141 |
| CIS_GENER_bdpm.txt | 26 |
| CIS_bdpm.txt | 28 |
| CIS_COMPO_bdpm.txt | 11 |
| CIS_MITM.txt | 5 |

### Recommandation

**Coder en dur l'encodage attendu pour chaque fichier** dans la configuration du parser. Ne pas utiliser de détection dynamique (les détecteurs statistiques confondent cp1252 et latin-1). La configuration recommandée :

```rust
const FILE_ENCODING: &[(&str, &str)] = &[
    ("CIS_bdpm.txt", "windows-1252"),
    ("CIS_CIP_bdpm.txt", "utf-8"),
    ("CIS_COMPO_bdpm.txt", "windows-1252"),
    ("CIS_HAS_SMR_bdpm.txt", "windows-1252"),
    ("CIS_HAS_ASMR_bdpm.txt", "windows-1252"),
    ("HAS_LiensPageCT_bdpm.txt", "utf-8"),
    ("CIS_GENER_bdpm.txt", "windows-1252"),
    ("CIS_CPD_bdpm.txt", "windows-1252"),
    ("CIS_CIP_Dispo_Spec.txt", "iso-8859-1"),
    ("CIS_MITM.txt", "windows-1252"),
    ("CIS_InfoImportantes.txt", "utf-8"),
];
```

### Normalisation Unicode

Après décodage cp1252 → UTF-8, les smart quotes (U+2019) devraient être normalisées en apostrophes droites (U+0027) pour la cohérence en base. Les fichiers UTF-8 contiennent également des smart quotes en UTF-8 natif (`\xE2\x80\x99`).

Les fichiers UTF-8 sont déjà en forme NFC (pas de normalisation supplémentaire nécessaire).

---

## 2.2 Fins de ligne incohérentes

### Constat

| Type | Fichiers |
|------|----------|
| **CRLF** (`\r\n`) | CIS_bdpm, CIS_CIP_Dispo_Spec, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_ASMR, CIS_HAS_SMR, CIS_MITM, HAS_LiensPageCT |
| **LF** (`\n`) | CIS_CIP_bdpm, CIS_InfoImportantes |

9 fichiers utilisent CRLF, 2 utilisent LF. Cette incohérence est classique dans les fichiers produits par des équipes ou systèmes différents.

### Anomalie CIS_CPD_bdpm.txt

Le fichier `CIS_CPD_bdpm.txt` contient **6 positions** avec des séquences `\r\r\n` (double CR avant LF), générant **9 lignes vides parasites**. Exemple hex :

```
...hospitalier\r\r\n\r\r\n66446220\t...
```

### Recommandation

Le parser doit :
1. Normaliser systématiquement les fins de ligne en supprimant tous les `\r` résiduels
2. Filtrer les lignes vides (longueur 0 après trim) avant le split tabulation
3. Ne pas dépendre du type de fin de ligne pour la logique de parsing

---

## 2.3 Formats de date incohérents

### Constat

Deux formats de date coexistent, sans que cela soit clairement indiqué dans la documentation :

| Format | Fichiers concernés | Champ |
|--------|--------------------|-------|
| **DD/MM/YYYY** | CIS_bdpm, CIS_CIP_bdpm, CIS_InfoImportantes, CIS_CIP_Dispo_Spec | Date AMM, dates déclaration, dates info sécurité, dates disponibilité |
| **YYYYMMDD** | CIS_HAS_SMR_bdpm, CIS_HAS_ASMR_bdpm | Date avis Commission transparence |

### Validation exhaustive

- **CIS_bdpm.txt** (colonne 7) : 15 848 dates au format DD/MM/YYYY — **100% conforme**, zéro exception
- **CIS_HAS_SMR_bdpm.txt** (colonne 3) : 15 257 dates au format YYYYMMDD — **100% conforme**, zéro exception

Les deux formats sont strictement respectés dans leur champ respectif.

### Recommandation

Convertir systématiquement vers le format ISO 8601 (YYYY-MM-DD) lors de l'import :

```rust
fn parse_date_ddmmyyyy(s: &str) -> Option<NaiveDate> {
    if s.is_empty() { return None; }
    NaiveDate::parse_from_str(s, "%d/%m/%Y").ok()
}

fn parse_date_yyyymmdd(s: &str) -> Option<NaiveDate> {
    if s.is_empty() { return None; }
    NaiveDate::parse_from_str(s, "%Y%m%d").ok()
}
```

La fonction de parsing doit être sélectionnée par configuration du fichier, pas par détection dynamique.

---

## 2.4 Champs à forte vacuité

Plusieurs colonnes ont des taux de remplissage très faibles. Ce n'est pas un bug mais une caractéristique du modèle de données, qui influence le design du schéma SQLite et les stratégies d'indexation.

| Fichier | Colonne | Taux de vacuité | Signification |
|---------|---------|-----------------|---------------|
| CIS_bdpm | StatutBdm (col 8) | 85,8% | La plupart des médicaments n'ont pas de statut spécial |
| CIS_bdpm | Numéro autorisation européenne (col 9) | 85,4% | AMM nationales sans composante européenne |
| CIS_CIP_bdpm | Agrément aux collectivités (col 7) | 96,1% | Rarement applicable |
| CIS_CIP_bdpm | Taux de remboursement (col 8) | 35,2% | Non applicable pour les présentations non remboursées |
| CIS_CIP_bdpm | Prix HT/TTC (cols 9-11) | 35,2% | Non applicable pour les présentations non commercialisées |
| CIS_COMPO_bdpm | Dosage (cols 4-5) | 8,7% | Certains composants n'ont pas de dosage |
| CIS_CIP_Dispo_Spec | CIP13 (col 1) | 95,4% | Fiche au niveau spécialité, pas présentation |
| CIS_CIP_Dispo_Spec | Date déclaration (col 5) | 65,5% | Souvent non déclarée |

**Impact** : Éviter les index sur les colonnes à plus de 80% de vacuité (coût d'indexation supérieur au bénéfice). Privilégier les index partiels (WHERE colonne IS NOT NULL).

---

## 2.5 Virgule comme séparateur décimal

### Constat

Les champs numériques (prix, taux, honoraires) dans CIS_CIP_bdpm.txt utilisent la virgule comme séparateur décimal (convention française).

| Pattern | Nombre de lignes |
|---------|-----------------|
| Virgule décimale (ex : `24,34`) | 13 546 |
| Point décimal | 0 |
| Vide | 7 357 |

### Recommandation

Convertir systématiquement la virgule en point lors de l'import dans les colonnes REAL de SQLite :

```rust
fn parse_french_decimal(s: &str) -> Option<f64> {
    if s.is_empty() { return None; }
    let normalized = s.replace(',', ".");
    normalized.parse::<f64>().ok()
}
```

Pour les calculs financiers précis, préférer `rust_decimal::Decimal` plutôt que `f64`.

---

## 2.6 Contenu HTML dans les données

### Constat

Le fichier `CIS_InfoImportantes.txt` contient du HTML brut dans sa quatrième colonne (champ texte). L'analyse révèle :

- **421 balises HTML distinctes**
- Principalement des liens `<a>` avec `target='_blank'` pointant vers les pages de l'ANSM (`ansm.sante.fr/...`)
- Des entités HTML comme `&ecirc;`, `&agrave;`, `&eacute;`, etc.
- Le contenu peut être long et multi-lien

### Exemple de contenu réel (colonne 3)

```html
<a target='_blank' href='https://ansm.sante.fr/...'>Information de sécurité du JJ/MM/AAAA : ...</a>
```

### Recommandation

Pour une base SQLite destinée à être requêtée, séparer le texte d'affichage et l'URL cible en deux colonnes :

```
texte: "Information de sécurité du 07/05/2026 : ..."
url: "https://ansm.sante.fr/..."
```

L'extraction peut être réalisée avec le crate `scraper` ou `select` en Rust, ou simplement avec des expressions régulières si le HTML est assez régulier.

---

## 2.7 Intégrité référentielle : orphelins constatés

### Constat

L'analyse croisée des Codes CIS révèle des références orphelines significatives — des Codes CIS présents dans des fichiers secondaires mais absents de la table maîtresse CIS_bdpm.txt.

| Fichier | CIS uniques | Orphelins | % orphelins | Interprétation |
|---------|-------------|-----------|-------------|----------------|
| CIS_CIP_bdpm | 14 573 | 4 | 0,03% | Négligeable, probablement transitoire |
| CIS_COMPO_bdpm | 15 846 | 0 | 0% | Parfaitement cohérent |
| CIS_HAS_SMR | 9 014 | 2 806 | 18,4% | Évaluations de médicaments retirés du répertoire |
| CIS_HAS_ASMR | 6 172 | 1 567 | 15,8% | Idem SMR |
| CIS_GENER | 10 628 | 2 503 | 23,5% | Groupes génériques incluant des principes retirés |
| CIS_InfoImportantes | 4 208 | 965 | 9,5% | Alertes de sécurité sur médicaments retirés |
| CIS_CPD | 12 492 | 0 | 0% | Parfaitement cohérent |
| CIS_CIP_Dispo_Spec | 727 | 12 | 1,6% | Mineur |
| CIS_MITM | 7 711 | 0 | 0% | Parfaitement cohérent |

### Cause probable

CIS_bdpm.txt ne conserve que les spécialités commercialisées ou arrêtées depuis **moins de 2 ans**. Les évaluations HAS, les groupes génériques et les informations de sécurité référencent des médicaments plus anciens qui ont été retirés du répertoire.

### Liaison HAS (Code dossier HAS)

| Métrique | Valeur |
|----------|--------|
| Codes HAS uniques dans SMR | 7 437 |
| Codes HAS uniques dans ASMR | 5 895 |
| Codes HAS uniques dans LiensPageCT | 10 327 |
| SMR non dans LiensPageCT | 1 017 |
| ASMR non dans LiensPageCT | 1 348 |
| LiensPageCT non dans SMR ni ASMR | 3 126 |

LiensPageCT est le sur-ensemble (10 327 codes) — il couvre des évaluations au-delà de SMR/ASMR.

### Recommandation

- Insérer les enregistrements orphelins avec un flag `is_orphan = 1`
- Ne pas utiliser de contrainte `FOREIGN KEY` stricte (ou la désactiver via `PRAGMA foreign_keys = OFF`)
- Journaliser le nombre d'orphelins à chaque import pour détecter les variations anormales

---

## 2.8 Intégrité structurelle

### Résultat global : excellent

| Métrique | Valeur |
|----------|--------|
| Lignes mal formées (nombre de colonnes inattendu) | **0** sur 163 451 lignes totales |
| Doublons de Code CIS dans CIS_bdpm.txt | **0** |
| Octets nuls | **0** |
| BOM UTF-8 | **0** |
| Tabulations embarquées dans les champs | **0** (aucune détectée) |
| Nouvelles lignes embarquées dans les champs | **0** |

Tous les fichiers ont un nombre de colonnes strictement constant par ligne (après filtrage des lignes vides de CIS_CPD).

### Vérification des énumérations

Toutes les valeurs catégorielles sont conformes aux spécifications :

| Champ | Valeurs observées | Conforme ? |
|-------|-------------------|------------|
| StatutBdm | `""` (13 594), `"Warning disponibilité"` (2 238), `"Alerte"` (16) | ✅ |
| Surveillance renforcée | `"Non"` (15 366), `"Oui"` (482) | ✅ |
| Nature composant | `"SA"` (26 892), `"FT"` (5 497) | ✅ |
| Type générique | `"0"` (1 781), `"1"` (8 826), `"2"` (36), `"4"` (61) | ✅ |
| Code statut dispo | `"1"` (66), `"2"` (421), `"3"` (15), `"4"` (264) | ✅ |
| Agrément collectivités | `"oui"` (15 012), `"non"` (5 891) | ✅ |

Aucune valeur inattendue n'a été détectée dans les champs énumérés.

---

## 2.9 Synthèse des problèmes par sévérité

| Sévérité | Problème | Fichiers impactés | Action requise |
|----------|----------|-------------------|----------------|
| 🔴 Critique | 3 encodages cohabitent | Tous | Hardcoder l'encodage par fichier |
| 🔴 Critique | Pas de ETag/Last-Modified | Tous (côté serveur) | Hash SHA-256 pour détection de changement |
| 🟡 Important | 2 formats de date | SMR, ASMR | Normalisation ISO 8601 |
| 🟡 Important | Virgule décimale | CIS_CIP | Conversion `,` → `.` |
| 🟡 Important | HTML brut | InfoImportantes | Extraction URL/texte |
| 🟡 Important | Orphelins CIS (18-23%) | SMR, ASMR, GENER, InfoImportantes | Flag is_orphan, FK lâches |
| 🟢 Mineur | CRLF vs LF incohérent | Tous | Strip \r systématique |
| 🟢 Mineur | \r\r\n dans CIS_CPD | CIS_CPD | Filtrer lignes vides |
| 🟢 Mineur | Forte vacuité certaines colonnes | CIP, Dispo | Pas d'index sur colonnes vides |
