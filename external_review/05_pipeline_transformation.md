# Pipeline de Transformation des Données BDPM

> Description complète du pipeline de transformation : du téléchargement des fichiers bruts à la base SQLite.
> Date : 26 mai 2026

---

## 1. Vue d'ensemble du pipeline

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   FETCH      │────▶│   DECODE     │────▶│   PARSE      │────▶│  TRANSFORM   │────▶│   LOAD       │
│              │     │              │     │              │     │              │     │              │
│ Télécharger  │     │ Détecter et  │     │ Découper les │     │ Normaliser   │     │ Insérer en   │
│ les fichiers │     │ convertir    │     │ lignes en    │     │ les données  │     │ base SQLite  │
│ sources      │     │ l'encodage   │     │ champs       │     │ (dates, etc) │     │              │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

---

## 2. Étape 1 : FETCH — Téléchargement

### 2.1 Configuration

```rust
const BASE_URL: &str = "https://base-donnees-publique.medicaments.gouv.fr";
const USER_AGENT: &str = "BDPM-Importer/1.0";
const REQUEST_DELAY: Duration = Duration::from_secs(5);
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(30);

struct FileSource {
    name: &'static str,
    url: &'static str,
    expected_fields: usize,
    frequency: CheckFrequency,
}

enum CheckFrequency {
    Daily,       // CIS_CIP_bdpm, CIS_CIP_Dispo_Spec
    Weekly,      // Fichiers standard
    Monthly,     // CIS_MITM
    OnDemand,    // CIS_InfoImportantes
}
```

### 2.2 Inventaire des sources

```rust
const FILE_SOURCES: &[FileSource] = &[
    FileSource { name: "CIS_bdpm.txt",            url: "/download/file/CIS_bdpm.txt",            expected_fields: 12, frequency: CheckFrequency::Weekly },
    FileSource { name: "CIS_CIP_bdpm.txt",        url: "/download/file/CIS_CIP_bdpm.txt",        expected_fields: 13, frequency: CheckFrequency::Daily },
    FileSource { name: "CIS_COMPO_bdpm.txt",      url: "/download/file/CIS_COMPO_bdpm.txt",      expected_fields: 8,  frequency: CheckFrequency::Weekly },
    FileSource { name: "CIS_HAS_SMR_bdpm.txt",    url: "/download/file/CIS_HAS_SMR_bdpm.txt",    expected_fields: 6,  frequency: CheckFrequency::Weekly },
    FileSource { name: "CIS_HAS_ASMR_bdpm.txt",   url: "/download/file/CIS_HAS_ASMR_bdpm.txt",   expected_fields: 6,  frequency: CheckFrequency::Weekly },
    FileSource { name: "HAS_LiensPageCT_bdpm.txt",url: "/download/file/HAS_LiensPageCT_bdpm.txt", expected_fields: 2,  frequency: CheckFrequency::Weekly },
    FileSource { name: "CIS_GENER_bdpm.txt",      url: "/download/file/CIS_GENER_bdpm.txt",      expected_fields: 5,  frequency: CheckFrequency::Weekly },
    FileSource { name: "CIS_CPD_bdpm.txt",        url: "/download/file/CIS_CPD_bdpm.txt",        expected_fields: 2,  frequency: CheckFrequency::Weekly },
    FileSource { name: "CIS_CIP_Dispo_Spec.txt",  url: "/download/file/CIS_CIP_Dispo_Spec.txt",  expected_fields: 8,  frequency: CheckFrequency::Daily },
    FileSource { name: "CIS_MITM.txt",            url: "/download/file/CIS_MITM.txt",            expected_fields: 4,  frequency: CheckFrequency::Monthly },
    FileSource { name: "CIS_InfoImportantes.txt", url: "/download/CIS_InfoImportantes.txt",       expected_fields: 4,  frequency: CheckFrequency::OnDemand },
];
```

### 2.3 Processus de téléchargement

```rust
async fn fetch_file(client: &Client, source: &FileSource) -> Result<Vec<u8>> {
    let url = format!("{}{}", BASE_URL, source.url);
    let mut attempts = 0;

    loop {
        attempts += 1;
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let data = resp.bytes().await?.to_vec();
                info!("Downloaded {} ({} bytes, attempt {})", source.name, data.len(), attempts);
                return Ok(data);
            }
            Ok(resp) => {
                warn!("HTTP {} for {} (attempt {})", resp.status(), source.name, attempts);
            }
            Err(e) => {
                warn!("Network error for {} (attempt {}): {}", source.name, attempts, e);
            }
        }

        if attempts >= MAX_RETRIES {
            bail!("Failed to download {} after {} attempts", source.name, MAX_RETRIES);
        }
        tokio::time::sleep(RETRY_DELAY).await;
    }
}
```

---

## 3. Étape 2 : DECODE — Détection et conversion d'encodage

### 3.1 Algorithme

```rust
fn decode_bdpm_file(raw: &[u8]) -> Result<String> {
    // Étape 1 : Tentative UTF-8
    match std::str::from_utf8(raw) {
        Ok(s) => {
            info!("Decoded as UTF-8");
            return Ok(s.to_string());
        }
        Err(_) => {
            info!("UTF-8 decoding failed, falling back to Windows-1252");
        }
    }

    // Étape 2 : Fallback Windows-1252 (CP1252)
    let (decoded, _encoding_used, _had_errors) = encoding_rs::WINDOWS_1252.decode(raw);

    if _had_errors {
        warn!("Windows-1252 decoding had errors for some bytes");
    }

    Ok(decoded.into_owned())
}
```

### 3.2 Normalisation des fins de ligne

```rust
fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
```

---

## 4. Étape 3 : PARSE — Découpage en champs

### 4.1 Parseur de ligne TSV

```rust
fn parse_line(line: &str, expected_fields: usize) -> Result<Vec<String>> {
    let mut fields: Vec<String> = line.split('\t').map(String::from).collect();

    // Supprimer les champs vides de fin (trailing tabs)
    while fields.len() > expected_fields {
        if fields.last().map_or(false, |f| f.is_empty()) {
            fields.pop();
        } else {
            break;
        }
    }

    // Compléter les champs manquants
    while fields.len() < expected_fields {
        fields.push(String::new());
    }

    // Valider le nombre de champs
    if fields.len() != expected_fields {
        bail!(
            "Expected {} fields, got {} in line: {}",
            expected_fields,
            fields.len(),
            &line[..line.len().min(100)]
        );
    }

    // Nettoyer les espaces en début et fin de chaque champ
    for field in &mut fields {
        let trimmed = field.trim();
        *field = trimmed.to_string();
    }

    Ok(fields)
}
```

### 4.2 Parseur de fichier complet

```rust
struct ParsedFile {
    rows: Vec<Vec<String>>,
    total_lines: usize,
    skipped_lines: usize,
    encoding: String,
}

fn parse_bdpm_file(raw: &[u8], expected_fields: usize) -> Result<ParsedFile> {
    let text = decode_bdpm_file(raw)?;
    let text = normalize_line_endings(&text);

    let mut rows = Vec::new();
    let mut total_lines = 0;
    let mut skipped_lines = 0;

    for line in text.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        total_lines += 1;

        match parse_line(line, expected_fields) {
            Ok(fields) => rows.push(fields),
            Err(e) => {
                warn!("Skipping malformed line {}: {}", total_lines, e);
                skipped_lines += 1;
            }
        }
    }

    // Détection d'encodage
    let encoding = if std::str::from_utf8(raw).is_ok() {
        "utf-8".to_string()
    } else {
        "windows-1252".to_string()
    };

    Ok(ParsedFile {
        rows,
        total_lines,
        skipped_lines,
        encoding,
    })
}
```

---

## 5. Étape 4 : TRANSFORM — Normalisation des données

### 5.1 Transformation des dates

```rust
fn normalize_date(input: &str) -> Option<NaiveDate> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Format YYYYMMDD (fichiers HAS)
    if input.len() == 8 && input.chars().all(|c| c.is_ascii_digit()) {
        return NaiveDate::parse_from_str(input, "%Y%m%d").ok();
    }

    // Format DD/MM/YYYY
    if input.contains('/') {
        return NaiveDate::parse_from_str(input, "%d/%m/%Y").ok();
    }

    // Format YYYY-MM-DD (déjà normalisé)
    NaiveDate::parse_from_str(input, "%Y-%m-%d").ok()
}
```

### 5.2 Transformation des prix

```rust
fn parse_price(input: &str) -> Option<f64> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Remplacer la virgule décimale française par un point
    let normalized = input.replace(',', ".");

    normalized.parse::<f64>().ok()
}
```

### 5.3 Transformation du taux de remboursement

```rust
fn parse_taux_remboursement(input: &str) -> Option<u8> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Supprimer les espaces et le symbole %
    let cleaned: String = input
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();

    cleaned.parse::<u8>().ok()
}
```

### 5.4 Nettoyage du HTML

```rust
fn clean_html(input: &str) -> String {
    let mut result = input.to_string();

    // Remplacer les balises <br> par des sauts de ligne
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");

    // Supprimer toutes les autres balises HTML
    let re = Regex::new(r"<[^>]+>").unwrap();
    result = re.replace_all(&result, "").to_string();

    // Décoder les entités HTML communes
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&#39;", "'");

    // Remplacer le ¿ par une apostrophe (artifact du système source)
    result = result.replace('\u{00BF}', "'");

    // Nettoyer les espaces multiples
    let ws_re = Regex::new(r"  +").unwrap();
    result = ws_re.replace_all(&result, " ").to_string();

    // Nettoyer les sauts de ligne multiples
    let nl_re = Regex::new(r"\n{3,}").unwrap();
    result = nl_re.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}
```

### 5.5 Transformation des champs multi-valués

```rust
/// Sépare un champ contenant des valeurs séparées par des points-virgules
fn parse_semicolon_list(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }
    input
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
```

---

## 6. Étape 5 : LOAD — Insertion en base SQLite

### 6.1 Stratégie d'insertion

Pour garantir la cohérence des données lors d'une mise à jour :

1. **Transaction** : Toujours insérer dans une transaction SQLite
2. **Delete + Insert** : Supprimer les anciennes données du fichier, puis insérer les nouvelles
3. **Batch** : Utiliser des insertions par lot pour la performance

```rust
fn import_specialites(conn: &Connection, rows: &[Vec<String>]) -> Result<()> {
    let tx = conn.transaction()?;

    // Supprimer les anciennes données
    tx.execute("DELETE FROM specialites", [])?;

    // Préparer l'insertion
    let mut stmt = tx.prepare(
        "INSERT INTO specialites (code_cis, denomination, forme_pharma, voies_admin,
         statut_amm, type_procedure, etat_commercial, date_amm, statut_bdm,
         num_europe, titulaires, surveillance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
    )?;

    for row in rows {
        let date_amm = normalize_date(&row[7]);
        let surveillance = match row[11].as_str() {
            "Oui" => 1,
            _ => 0,
        };

        stmt.execute(rusqlite::params![
            row[0].parse::<i64>()?,
            row[1],
            row[2],
            row[3],      // Voies d'administration (brut, séparateur ;)
            row[4],
            row[5],
            row[6],
            date_amm,
            if row[8].is_empty() { None } else { Some(&row[8]) },
            if row[9].is_empty() { None } else { Some(&row[9]) },
            row[10],
            surveillance,
        ])?;
    }

    // Enregistrer l'import
    tx.execute(
        "INSERT INTO import_history (file_name, rows_count, encoding, status)
         VALUES ('CIS_bdpm.txt', ?, 'cp1252', 'success')",
        [rows.len() as i64],
    )?;

    tx.commit()?;
    Ok(())
}
```

### 6.2 Performance d'insertion

Pour insérer efficacement des dizaines de milliers de lignes :

```rust
fn optimize_for_bulk_insert(conn: &Connection) {
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = OFF;
        PRAGMA cache_size = -64000;  -- 64 Mo de cache
        PRAGMA temp_store = MEMORY;
    ").unwrap();
}

fn restore_normal_settings(conn: &Connection) {
    conn.execute_batch("
        PRAGMA synchronous = NORMAL;
        PRAGMA cache_size = -2000;
    ").unwrap();
}
```

---

## 7. Pipeline complet

### 7.1 Orchestration

```rust
async fn run_full_import(client: &Client, db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)?;
    optimize_for_bulk_import(&conn);

    for source in FILE_SOURCES {
        info!("Processing {}...", source.name);

        // 1. FETCH
        let raw = fetch_file(client, source).await?;

        // 2. DECODE + PARSE
        let parsed = parse_bdpm_file(&raw, source.expected_fields)?;
        info!("  Parsed {} lines, {} skipped", parsed.total_lines, parsed.skipped_lines);

        // 3. TRANSFORM + LOAD
        let tx = conn.transaction()?;
        match source.name {
            "CIS_bdpm.txt" => import_specialites(&tx, &parsed.rows)?,
            "CIS_CIP_bdpm.txt" => import_presentations(&tx, &parsed.rows)?,
            "CIS_COMPO_bdpm.txt" => import_compositions(&tx, &parsed.rows)?,
            "CIS_HAS_SMR_bdpm.txt" => import_avis_smr(&tx, &parsed.rows)?,
            "CIS_HAS_ASMR_bdpm.txt" => import_avis_asmr(&tx, &parsed.rows)?,
            "HAS_LiensPageCT_bdpm.txt" => import_has_liens(&tx, &parsed.rows)?,
            "CIS_GENER_bdpm.txt" => import_generiques(&tx, &parsed.rows)?,
            "CIS_CPD_bdpm.txt" => import_conditions(&tx, &parsed.rows)?,
            "CIS_CIP_Dispo_Spec.txt" => import_disponibilites(&tx, &parsed.rows)?,
            "CIS_MITM.txt" => import_mitm(&tx, &parsed.rows)?,
            "CIS_InfoImportantes.txt" => import_infos_importantes(&tx, &parsed.rows)?,
            _ => bail!("Unknown file: {}", source.name),
        }

        // Enregistrer l'import
        let hash = compute_file_hash(&raw);
        tx.execute(
            "INSERT INTO import_history (file_name, rows_count, sha256, file_size, encoding, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'success')",
            rusqlite::params![source.name, parsed.rows.len() as i64, hash, raw.len() as i64, parsed.encoding],
        )?;

        tx.commit()?;
        info!("  Imported successfully");

        // Délai entre les fichiers
        tokio::time::sleep(REQUEST_DELAY).await;
    }

    restore_normal_settings(&conn);
    Ok(())
}
```

### 7.2 Ordre d'import

L'ordre d'import est important pour respecter les contraintes de clés étrangères :

1. **`specialites`** (CIS_bdpm.txt) — Table centrale, doit être importée en premier
2. **`presentations`** (CIS_CIP_bdpm.txt) — Dépend de specialites
3. **`compositions`** (CIS_COMPO_bdpm.txt) — Dépend de specialites
4. **`has_liens_ct`** (HAS_LiensPageCT_bdpm.txt) — Table de référence pour les codes HAS
5. **`avis_smr`** (CIS_HAS_SMR_bdpm.txt) — Dépend de specialites et has_liens_ct
6. **`avis_asmr`** (CIS_HAS_ASMR_bdpm.txt) — Dépend de specialites et has_liens_ct
7. **`groupes_generiques`** (CIS_GENER_bdpm.txt) — Dépend de specialites
8. **`conditions_prescription`** (CIS_CPD_bdpm.txt) — Dépend de specialites
9. **`disponibilites`** (CIS_CIP_Dispo_Spec.txt) — Dépend de specialites
10. **`mitm`** (CIS_MITM.txt) — Dépend de specialites
11. **`infos_importantes`** (CIS_InfoImportantes.txt) — Dépend de specialites

---

## 8. Gestion des erreurs

### 8.1 Types d'erreurs

| Type | Exemple | Action |
|------|---------|--------|
| Réseau | Timeout, DNS, 5xx | Retry avec backoff exponentiel |
| Encodage | Octet non décodable | Fallback vers CP1252, logging |
| Parsing | Nombre de champs incorrect | Skip la ligne, logging, continuer |
| Transformation | Date invalide | Mettre à NULL, logging |
| Base de données | Contrainte violée | Transaction, rollback, logging |

### 8.2 Rapport d'erreurs

Chaque import génère un rapport avec :
- Nombre total de lignes traitées
- Nombre de lignes ignorées (malformées)
- Nombre d'avertissements (champs vides, dates invalides, etc.)
- Détail des lignes problématiques (numéro de ligne + contenu tronqué)

---

## 9. Validation post-import

### 9.1 Contrôles de cohérence

```sql
-- 1. Vérifier le nombre total de spécialités
SELECT COUNT(*) FROM specialites;
-- Attendu : ~15 848

-- 2. Vérifier l'absence de doublons sur la clé primaire
SELECT code_cis, COUNT(*) FROM specialites GROUP BY code_cis HAVING COUNT(*) > 1;

-- 3. Vérifier la cohérence des clés étrangères
SELECT COUNT(*) FROM presentations WHERE code_cis NOT IN (SELECT code_cis FROM specialites);

-- 4. Vérifier les champs obligatoires vides
SELECT COUNT(*) FROM specialites WHERE denomination = '' OR forme_pharma = '';

-- 5. Vérifier les dates invalides
SELECT code_cis, date_amm FROM specialites WHERE date_amm IS NULL AND statut_amm = 'Autorisation active';

-- 6. Vérifier les orphelins
SELECT * FROM cis_orphelins LIMIT 20;
```

### 9.2 Statistiques de l'import

```sql
SELECT file_name, rows_count, encoding, import_date, status
FROM import_history
ORDER BY import_date DESC;
```
