# Rapport d'Intégrité Référentielle et Qualité des Données

> Analyse de l'intégrité référentielle, de la qualité des données et des anomalies dans les fichiers BDPM.
> Date d'analyse : 26 mai 2026

---

## 1. Résumé exécutif

L'analyse approfondie des 11 fichiers BDPM révèle une base de données globalement cohérente mais présentant plusieurs anomalies notables : des encodages mixtes (CP1252/UTF-8), des enregistrements orphelins significatifs dans les fichiers HAS et les groupes génériques, et des inconsistencies de formatage mineures. Aucune ligne malformée (nombre de champs incorrect) n'a été détectée dans aucun fichier, ce qui est un bon signe de stabilité structurelle.

---

## 2. Intégrité structurelle

### 2.1 Conformité du nombre de champs

| Fichier | Champs attendus | Lignes conformes | Lignes malformées | Taux de conformité |
|---------|----------------|-----------------|-------------------|-------------------|
| CIS_bdpm.txt | 12 | 15 848 | 0 | **100%** |
| CIS_CIP_bdpm.txt | 13 | 20 903 | 0 | **100%** |
| CIS_COMPO_bdpm.txt | 8 | 32 389 | 0 | **100%** |
| CIS_HAS_SMR_bdpm.txt | 6 | 15 257 | 0 | **100%** |
| CIS_HAS_ASMR_bdpm.txt | 6 | 9 906 | 0 | **100%** |
| HAS_LiensPageCT_bdpm.txt | 2 | 10 342 | 0 | **100%** |
| CIS_GENER_bdpm.txt | 5 | 10 704 | 0 | **100%** |
| CIS_CPD_bdpm.txt | 2 | 28 154 | 0 | **100%** |
| CIS_CIP_Dispo_Spec.txt | 8 | 766 | 0 | **100%** |
| CIS_MITM.txt | 4 | 7 711 | 0 | **100%** |

**Conclusion** : La structure TSV des fichiers est parfaitement respectée. Le choix du tabulateur comme séparateur, sans guillemets de délimitation, fonctionne correctement car aucun champ ne contient de tabulation dans les données réelles.

### 2.2 Lignes vides

Chaque fichier se termine par une ligne vide (le dernier `\n` ou `\r\n`), ce qui est normal. Le fichier `CIS_CPD_bdpm.txt` a 4 lignes vides au total (3 supplémentaires), probablement dues à des retours à la ligne en fin de fichier.

---

## 3. Intégrité référentielle

### 3.1 Graphe des relations

```
CIS_bdpm.txt (15 848 CIS codes) — TABLE CENTRALE
│
├─── CIS_COMPO_bdpm.txt (15 846 CIS codes)
│    └─ Couverture : 99,99%
│    └─ Orphelins : 0
│
├─── CIS_CPD_bdpm.txt (12 492 CIS codes)
│    └─ Couverture : 78,82%
│    └─ Orphelins : 0
│
├─── CIS_MITM.txt (7 711 CIS codes)
│    └─ Couverture : 48,65%
│    └─ Orphelins : 0
│
├─── CIS_CIP_bdpm.txt (14 573 CIS codes)
│    └─ Couverture : 91,91%
│    └─ Orphelins : 4 ⚠️
│
├─── CIS_HAS_SMR_bdpm.txt (9 014 CIS codes)
│    └─ Couverture (dans central) : 56,88%
│    └─ Orphelins : 2 806 ⚠️⚠️
│
├─── CIS_HAS_ASMR_bdpm.txt (6 172 CIS codes)
│    └─ Couverture (dans central) : 39,13%
│    └─ Orphelins : 1 567 ⚠️⚠️
│
├─── CIS_GENER_bdpm.txt (10 628 CIS codes)
│    └─ Couverture (dans central) : 67,05%
│    └─ Orphelins : 2 503 ⚠️⚠️
│
└─── CIS_CIP_Dispo_Spec.txt (727 CIS codes)
     └─ Couverture : 4,59%
     └─ Orphelins : 12 ⚠️

HAS_LiensPageCT_bdpm.txt (10 327 codes HAS) — TABLE DE RÉFÉRENCE HAS
│
├─── CIS_HAS_SMR_bdpm.txt (7 437 codes HAS)
│    └─ Couverture : 86,32%
│    └─ Orphelins : 1 017
│
└─── CIS_HAS_ASMR_bdpm.txt (5 895 codes HAS)
     └─ Couverture : 77,14%
     └─ Orphelins : 1 348
```

### 3.2 Analyse des orphelins

#### Orphelins dans CIS_CIP_bdpm.txt (4 CIS codes)

Ces 4 codes de présentation n'ont pas de spécialité correspondante dans le fichier central :

```
64917175, 62969013, 63278664, 69912584
```

**Hypothèse** : Il s'agit probablement de spécialités très récemment autorisées ou de erreurs de synchronisation entre les fichiers lors de la génération mensuelle. Le fichier des présentations a été mis à jour le 25/05/2026 tandis que le fichier central date du 28/04/2026, ce qui suggère un décalage temporel.

#### Orphelins dans les fichiers HAS (SMR : 2 806 / ASMR : 1 567)

C'est l'anomalie la plus significative. Près de 31% des CIS codes dans les avis SMR et 25% dans les avis ASMR n'existent pas dans le fichier central.

**Explication** : Le fichier central `CIS_bdpm.txt` ne contient que les médicaments **actuellement commercialisés ou retirés depuis moins de 5 ans**. Les avis HAS ont une portée historique plus large : ils couvrent également les spécialités retirées du marché depuis plus de 5 ans, dont les avis restent consultables.

**Implication** : La base SQLite doit **accepter ces enregistrements orphelins** et ne pas les rejeter. Ils constituent un historique précieux.

#### Orphelins dans les groupes génériques (2 503)

Même explication que pour les fichiers HAS : les groupes génériques peuvent référencer des spécialités anciennes retirées du fichier central.

#### Orphelins dans les disponibilités (12)

12 spécialités en rupture de stock n'ont pas de fiche dans le fichier central. Cela peut être dû à des spécialités en cours de retrait ou à des erreurs de synchronisation.

### 3.3 Codes HAS sans lien CT

| Fichier | Codes HAS | Avec lien CT | Sans lien CT |
|---------|-----------|-------------|-------------|
| CIS_HAS_SMR_bdpm.txt | 7 437 | 6 420 (86,3%) | 1 017 (13,7%) |
| CIS_HAS_ASMR_bdpm.txt | 5 895 | 4 547 (77,1%) | 1 348 (22,9%) |

**Explication** : Les avis anciens (antérieurs à la numérisation) n'ont pas de page web correspondante. Certains liens peuvent aussi avoir été cassés lors de migrations du site de la HAS.

---

## 4. Qualité des données

### 4.1 Champs vides

Analyse de la proportion de champs vides par fichier et par champ :

**CIS_bdpm.txt :**

| Champ | Champ vide | % vide |
|-------|-----------|--------|
| Code CIS | 0 | 0% |
| Dénomination | 0 | 0% |
| Forme pharma | 0 | 0% |
| Voies admin | 0 | 0% |
| Statut AMM | 0 | 0% |
| Type procédure | 0 | 0% |
| État commercial | 0 | 0% |
| Date AMM | ~quelques uns | <1% |
| Statut BDM | ~15 500 | ~98% |
| Numéro européen | ~15 200 | ~96% |
| Titulaires | ~quelques uns | <1% |
| Surveillance | 0 | 0% |

**Observations** :
- Le champ Statut BDM est vide pour 98% des enregistrements (seuls les médicaments avec alerte ont une valeur)
- Le numéro d'autorisation européenne est vide pour la très grande majorité (procédures nationales)

**CIS_CIP_bdpm.txt :**

| Champ | Champ vide | % vide |
|-------|-----------|--------|
| Taux de remboursement | 7 357 | 35,2% |
| Prix HT | ~8 000 | ~38% |
| Prix TTC | ~8 000 | ~38% |
| Honoraires | ~9 000 | ~43% |
| Indications remboursement | ~20 000 | ~96% |

**Observations** :
- Les champs de prix et de remboursement sont fréquemment vides pour les présentations non remboursées ou non commercialisées
- Le champ indications n'est rempli que pour les présentations avec des conditions spécifiques de remboursement

### 4.2 Doublons

| Fichier | Clé | Doublons |
|---------|-----|----------|
| CIS_bdpm.txt | Code CIS | **0** (chaque CIS est unique) |
| CIS_CIP_bdpm.txt | Code CIS + CIP7 | **0** (chaque combinaison est unique) |
| CIS_COMPO_bdpm.txt | Code CIS + Code substance + Nature | **0** |
| CIS_CPD_bdpm.txt | Code CIS + Condition | Plusieurs conditions par CIS (normal) |
| HAS_LiensPageCT_bdpm.txt | Code dossier HAS | **0** |

**Conclusion** : Aucun doublon sur les clés primaires naturelles. La structure est propre.

### 4.3 Cohérence des valeurs énumérées

| Fichier | Champ | Valeurs attendues | Valeurs observées | Cohérent |
|---------|-------|------------------|------------------|----------|
| CIS_bdpm.txt | Surveillance | Oui, Non | Oui, Non | ✅ |
| CIS_bdpm.txt | Statut BDM | Alerte, Warning disponibilité, vide | Alerte, Warning disponibilité, vide | ✅ |
| CIS_COMPO_bdpm.txt | Nature | SA, FT | SA, FT | ✅ |
| CIS_GENER_bdpm.txt | Type générique | 0, 1, 2, 4 | 0, 1, 2, 4 | ✅ |
| CIS_CIP_bdpm.txt | Agrément collectivités | oui, non, inconnu | oui, non, inconnu | ✅ |
| CIS_CIP_bdpm.txt | Taux remboursement | XX% ou XX % | 65%, 100%, 30%, 15%, 35% + variantes avec espaces | ⚠️ Inconsistance d'espacement |
| CIS_CIP_Dispo_Spec.txt | Libellé statut (code 4) | Remise à disposition | "Remise à disposition" ET "remise à disposition" | ⚠️ Inconsistance de capitalisation |

### 4.4 Validité des dates

| Fichier | Format | Problèmes détectés |
|---------|--------|-------------------|
| CIS_bdpm.txt | DD/MM/YYYY | Aucun — toutes les dates sont valides |
| CIS_HAS_SMR_bdpm.txt | YYYYMMDD | Aucun — toutes les dates sont valides |
| CIS_HAS_ASMR_bdpm.txt | YYYYMMDD | Aucun — toutes les dates sont valides |
| CIS_CIP_Dispo_Spec.txt | DD/MM/YYYY | Aucun — toutes les dates sont valides |

### 4.5 Validité des URLs

| Fichier | Champ URL | Problèmes détectés |
|---------|----------|-------------------|
| HAS_LiensPageCT_bdpm.txt | Lien CT | Toutes commencent par `https://www.has-sante.fr/` — valides |
| CIS_MITM.txt | Lien BDPM | Utilisent l'ancien format `extrait.php?specid=` — fonctionnel mais redirige (301) |
| CIS_CIP_Dispo_Spec.txt | Lien ANSM | URLs vers `ansm.sante.fr` — valides |

---

## 5. Analyse des anomalies par sévérité

### 5.1 Critique (impact sur la justesse des données)

| Anomalie | Impact | Mitigation |
|----------|--------|------------|
| Encodage mixte CP1252/UTF-8 | Caractères corrompus si mauvais décodage | Détection automatique avec fallback CP1252 |
| 2 806+ CIS orphelins dans les avis HAS | Perte de données historiques si rejet | Accepter les orphelins, clés étrangères optionnelles |

### 5.2 Élevée (impact sur la cohérence)

| Anomalie | Impact | Mitigation |
|----------|--------|------------|
| Dates en deux formats | Erreurs de comparaison/tri | Normalisation systématique en ISO 8601 |
| HTML dans les champs texte | Affichage incorrect si rendu brut | Double stockage (raw + clean) |
| Fichier InfoImportantes dynamique | Risque de contenu vide ou périmé | Vérification de taille + hash |

### 5.3 Moyenne (impact sur la qualité)

| Anomalie | Impact | Mitigation |
|----------|--------|------------|
| Trailing tabs dans CIP | Champ fantôme lors du parsing | Trim des champs vides de fin |
| Fins de ligne mixtes | `\r` résiduel dans les données | Normalisation universelle |
| Taux de remboursement inconsistent | Recherche par taux imprécise | Normalisation (suppression espaces) |
| Capitalisation inconsistante des statuts | Regroupement imprécis | Normalisation de casse |

### 5.4 Faible (cosmétique)

| Anomalie | Impact | Mitigation |
|----------|--------|------------|
| Virgule décimale | Incompatibilité avec les parsers numériques | Conversion en point décimal |
| `¿` dans les indications | Affichage inesthétique | Mapping ou nettoyage |
| Anciennes URLs dans MITM | Redirection 301 (fonctionnelle) | Mise à jour optionnelle des URLs |

---

## 6. Recommandations pour le projet Rust

### 6.1 Validation à l'import

Le pipeline d'import doit inclure les validations suivantes :

1. **Validation structurelle** : Nombre de champs conforme au schéma
2. **Validation d'encodage** : Tous les caractères décodables en UTF-8 après conversion
3. **Validation de types** : Codes CIS numériques, dates valides, prix numériques
4. **Validation référentielle** : Log des CIS orphelins (mais ne pas rejeter)
5. **Validation de contenu** : HTML détecté et nettoyé, URLs valides

### 6.2 Rapport de qualité automatisé

Générer un rapport après chaque import :

```rust
struct QualityReport {
    total_rows: usize,
    valid_rows: usize,
    skipped_rows: usize,
    orphan_cis_codes: Vec<(String, i64)>,  // (table, code_cis)
    empty_fields: HashMap<String, usize>,   // (field_name, count)
    encoding_issues: Vec<String>,
    html_fields_count: usize,
    date_format_issues: usize,
}
```

### 6.3 Alertes et seuils

Définir des seuils pour déclencher des alertes :

| Métrique | Seuil d'alerte | Action |
|----------|---------------|--------|
| Lignes malformées | > 0 | Bloquer l'import, investiguer |
| Perte de données vs import précédent | > 10% | Alerte, confirmation manuelle |
| Nouveaux CIS orphelins | Variation > 5% | Alerte info |
| Lignes vides | > 5% | Alerte info |
| Champs HTML | Variation > 20% | Alerte info |
