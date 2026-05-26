# 04 — Stratégie de parsing, normalisation et contrôle qualité

---

## 4.1 Pipeline de parsing en 5 étapes

Le parsing des fichiers BDPM suit un pipeline déterministe en 5 étapes, chaque étape validant les sorties de la précédente. Ce pipeline est conçu pour être implémentable en Rust de manière efficace, avec un minimum d'allocation dynamique et un maximum de validation compile-time.

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  1. Fetch +  │───>│  2. Décode + │───>│  3. Split +  │───>│  4. Normalize│───>│  5. Validate │
│  Hash SHA    │    │  Lignes      │    │  Structure   │    │  Champs      │    │  + Insert    │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
```

---

## Étape 1 : Téléchargement et vérification d'intégrité

**Objectif** : Obtenir le fichier brut, vérifier s'il a changé, l'archiver.

**Opérations** :
1. Télécharger le contenu binaire (sans décodage texte à ce stade)
2. Calculer le SHA-256 et le comparer au hash de l'import précédent
3. Si identique → court-circuiter, journaliser `no_change`
4. Si différent → stocker dans le répertoire d'archive horodaté (ex : `raw/2026-05-26/`)
5. Mettre à jour le manifeste de collecte

**Sortie** : Fichier binaire brut + métadonnées (hash, taille, statut HTTP)

```rust
struct RawFile {
    name: String,
    data: Vec<u8>,
    sha256: String,
    size_bytes: u64,
    http_status: u16,
}
```

**Critère de succès** : SHA-256 calculé avec succès, statut HTTP 200.

---

## Étape 2 : Décodage et normalisation des lignes

**Objectif** : Transformer le contenu binaire en lignes de texte UTF-8 valides.

**Opérations** :
1. Décoder le contenu binaire avec l'encodage spécifique à chaque fichier (cp1252, latin-1 ou utf-8). L'encodage est **codé en dur** dans la configuration du parser.
2. Normaliser les fins de ligne : supprimer tous les `\r` résiduels. Cela unifie CRLF et LF en LF, et élimine les séquences `\r\r\n` de CIS_CPD.
3. Filtrer les lignes vides (longueur 0 après trim) résultant des anomalies de fin de ligne.
4. Normaliser Unicode : convertir en NFC si nécessaire (les fichiers UTF-8 sont déjà en NFC, mais cp1252 décodé peut produire des formes décomposées dans certains cas marginaux).

**Configuration d'encodage** :

```rust
enum FileEncoding {
    Windows1252,  // cp1252 — 7 fichiers
    Iso8859_1,    // latin-1 — 1 fichier
    Utf8,         // utf-8 — 3 fichiers
}

struct FileConfig {
    filename: &'static str,
    encoding: FileEncoding,
    expected_columns: usize,
    date_format: DateFormat,
}
```

**Sortie** : `Vec<String>` où chaque String est une ligne de texte UTF-8 valide, sans \r.

**Critère de succès** : Aucune erreur de décodage (les encodeurs cp1252 et latin-1 sont déterministes et sans perte).

---

## Étape 3 : Split tabulation et validation structurelle

**Objectif** : Découper chaque ligne en champs et vérifier la structure.

**Opérations** :
1. Découper chaque ligne sur la tabulation (`\t`)
2. Vérifier le nombre de colonnes attendu par fichier
3. En cas de nombre de colonnes inattendu : journaliser la ligne avec son numéro, mais **ne pas interrompre l'import** (mode de récupération)
4. Détecter les tabulations embarquées dans les champs (aucune constatée dans les données actuelles, mais le risque existe théoriquement)

**Résultats observés** :

| Fichier | Colonnes attendues | Lignes conformes | Lignes anormales |
|---------|--------------------|-----------------|-----------------|
| CIS_bdpm.txt | 12 | 15 848 (100%) | 0 |
| CIS_CIP_bdpm.txt | 13 | 20 903 (100%) | 0 |
| CIS_COMPO_bdpm.txt | 8 | 32 389 (100%) | 0 |
| CIS_HAS_SMR_bdpm.txt | 6 | 15 257 (100%) | 0 |
| CIS_HAS_ASMR_bdpm.txt | 6 | 9 906 (100%) | 0 |
| HAS_LiensPageCT_bdpm.txt | 2 | 10 342 (100%) | 0 |
| CIS_GENER_bdpm.txt | 5 | 10 704 (100%) | 0 |
| CIS_CPD_bdpm.txt | 2 | 28 151 (100%) | 0 |
| CIS_CIP_Dispo_Spec.txt | 8 | 766 (100%) | 0 |
| CIS_MITM.txt | 4 | 7 711 (100%) | 0 |
| CIS_InfoImportantes.txt | 4 | 10 189 (100%) | 0 |

**Sortie** : `Vec<Vec<String>>` (matrice de champs) + rapport de lignes anormales.

**Critère de succès** : 100% des lignes conformes (si anomalie, journaliser et continuer).

---

## Étape 4 : Normalisation par champ

**Objectif** : Transformer chaque champ dans son format canonique pour le stockage en base.

### 4a. Dates

| Format source | Fichiers | Conversion |
|---------------|----------|------------|
| DD/MM/YYYY | CIS_bdpm, CIS_CIP, InfoImportantes, Dispo | → YYYY-MM-DD |
| YYYYMMDD | CIS_HAS_SMR, CIS_HAS_ASMR | → YYYY-MM-DD |
| Vide | Tous (certains champs date sont optionnels) | → NULL |

```rust
fn normalize_date(value: &str, format: DateFormat) -> Option<String> {
    if value.trim().is_empty() { return None; }
    let date = match format {
        DateFormat::DdMmYyyy => NaiveDate::parse_from_str(value.trim(), "%d/%m/%Y").ok()?,
        DateFormat::Yyyymmdd => NaiveDate::parse_from_str(value.trim(), "%Y%m%d").ok()?,
    };
    Some(date.format("%Y-%m-%d").to_string())
}
```

### 4b. Nombres (prix, taux)

Remplacer la virgule décimale par un point. Les valeurs vides restent NULL.

```rust
fn normalize_decimal(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() { return None; }
    let normalized = trimmed.replace(',', ".");
    normalized.parse::<f64>().ok()
}
```

### 4c. Apostrophes (smart quotes)

Normaliser les smart quotes (U+2019) issues de cp1252 et les smart quotes UTF-8 natives vers l'apostrophe droite (U+0027) pour la cohérence en base.

```rust
fn normalize_apostrophes(value: &str) -> String {
    value.replace('\u{2019}', "'")  // Right single quotation mark → apostrophe
         .replace('\u{2018}', "'")  // Left single quotation mark → apostrophe
}
```

Cette normalisation est optionnelle mais recommandée pour éviter les incohérences de recherche textuelle.

### 4d. HTML (CIS_InfoImportantes uniquement)

Extraire le texte d'affichage et l'URL des balises `<a>` :

```rust
struct InfoImportantesContent {
    texte: String,    // Texte d'affichage, entités HTML décodées
    url: Option<String>, // URL extraite du href
}

fn parse_html_content(html: &str) -> InfoImportantesContent {
    // Extraire le texte entre <a>...</a>
    // Extraire l'attribut href
    // Décoder les entités HTML (&ecirc; → ê, &agrave; → à, etc.)
}
```

### 4e. Espaces

Trimmer les espaces en début et fin de champ. Ne pas modifier les espaces internes.

```rust
fn trim_field(value: &str) -> &str {
    value.trim()
}
```

**Sortie** : `Vec<Record>` où chaque Record est une structure Rust typée par fichier.

---

## Étape 5 : Validation sémantique et insertion

**Objectif** : Vérifier la cohérence des données avant insertion en base.

**Opérations** :

1. **Vérifier les énumérations** : chaque valeur doit appartenir au domaine attendu. Journaliser les valeurs inattendues sans bloquer l'import.

   | Champ | Valeurs autorisées |
   |-------|-------------------|
   | StatutBdm | `""`, `"Alerte"`, `"Warning disponibilité"` |
   | Surveillance renforcée | `"Oui"`, `"Non"` |
   | Nature composant | `"SA"`, `"FT"` |
   | Type générique | `"0"`, `"1"`, `"2"`, `"4"` |
   | Code statut dispo | `"1"`, `"2"`, `"3"`, `"4"` |
   | Agrément collectivités | `"oui"`, `"non"`, `"inconnu"` |

2. **Vérifier l'existence du Code CIS** dans la table maîtresse. Les orphelins sont insérés avec `is_orphan = 1` plutôt que rejetés.

3. **Insérer dans SQLite** via une transaction par fichier (autocommit désactivé pour la performance).

```rust
fn validate_and_insert(records: Vec<Record>, db: &Connection) -> Result<ImportStats> {
    let tx = db.unchecked_transaction()?;
    let mut stats = ImportStats::default();
    
    for record in &records {
        // Validation sémantique
        if let Err(e) = record.validate() {
            stats.validation_warnings.push(e);
        }
        
        // Insertion avec upsert
        match record.upsert(&tx)? {
            UpsertResult::Inserted => stats.rows_inserted += 1,
            UpsertResult::Updated => stats.rows_updated += 1,
            UpsertResult::Unchanged => {}
        }
    }
    
    tx.commit()?;
    Ok(stats)
}
```

**Sortie** : Base SQLite mise à jour + statistiques d'import + avertissements de validation.

---

## 4.2 Contrôle qualité : checks automatisés

Le pipeline doit intégrer des checks de qualité exécutables après chaque import. Ces checks sont implémentés comme des requêtes SQL sur la base importée, et leur résultat est journalisé.

### Catégorie 1 : Complétude

| Check | Requête | Seuil |
|-------|---------|-------|
| Nombre de lignes importées = lignes source | `SELECT COUNT(*) FROM table` vs nombre de lignes lues | 100% |
| Aucun fichier manquant | Vérifier que les 11 tables ont été mises à jour dans le même import | 11/11 |

### Catégorie 2 : Cohérence référentielle

| Check | Requête | Seuil |
|-------|---------|-------|
| Code CIS orphelins par table | `SELECT COUNT(*) FROM cis_has_smr WHERE code_cis NOT IN (SELECT code_cis FROM cis_specialites)` | Alerte si variation > 5% |
| Code dossier HAS orphelins | `SELECT COUNT(*) FROM cis_has_smr WHERE code_dossier_has NOT IN (SELECT code_dossier_has FROM has_liens_ct)` | Journaliser |

### Catégorie 3 : Validité des énumérations

| Check | Requête | Seuil |
|-------|---------|-------|
| Valeurs hors domaine | `SELECT DISTINCT nature FROM cis_compositions WHERE nature NOT IN ('SA', 'FT')` | 0 |
| Valeurs inattendues | Même logique pour chaque énumération | 0 |

### Catégorie 4 : Cohérence temporelle

| Check | Requête | Seuil |
|-------|---------|-------|
| Dates d'AMM dans une plage raisonnable | `SELECT COUNT(*) FROM cis_specialites WHERE date_amm < '1950-01-01' OR date_amm > CURRENT_DATE` | 0 |
| Dates de début < dates de fin | `SELECT COUNT(*) FROM cis_info_importantes WHERE date_fin < date_debut` | 0 |

### Catégorie 5 : Détection de régressions

| Check | Requête | Seuil |
|-------|---------|-------|
| Nombre d'enregistrements stable | Comparer COUNT(*) par table avec l'import précédent | Alerte si baisse > 2% |
| Nombre d'orphelins stable | Comparer le nombre d'orphelins par table | Alerte si hausse > 10% |

### Format du rapport de validation

```json
{
  "import_id": 42,
  "timestamp": "2026-05-26T03:15:00Z",
  "checks": [
    {
      "category": "completeness",
      "name": "row_count_cis_specialites",
      "expected": 15848,
      "actual": 15848,
      "status": "pass"
    },
    {
      "category": "referential_integrity",
      "name": "orphan_cis_has_smr",
      "previous_count": 2806,
      "current_count": 2815,
      "variation_pct": 0.32,
      "status": "pass"
    },
    {
      "category": "regression",
      "name": "row_count_cis_specialites_regression",
      "previous_count": 15850,
      "current_count": 15848,
      "variation_pct": -0.01,
      "status": "pass"
    }
  ],
  "summary": {
    "total_checks": 25,
    "passed": 24,
    "failed": 0,
    "warnings": 1
  }
}
```
