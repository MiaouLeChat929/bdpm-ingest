# Architecture du Projet Rust — BDPM Importer

> Conception architecturale du projet Rust pour l'import, le stockage et l'accès aux données BDPM.
> Date : 26 mai 2026

---

## 1. Objectifs du projet

- **Import fiable** des 11 fichiers BDPM vers une base SQLite
- **Détection proactive** des mises à jour sans spammer le serveur
- **API locale** (CLI + bibliothèque) pour interroger les données
- **Extensibilité** pour ajouter des sources de données futures (ANSM, HAS, etc.)
- **Robustesse** face aux quirks d'encodage et de formatage

---

## 2. Stack technique

| Composant | Crate Rust | Rôle |
|-----------|-----------|------|
| HTTP Client | `reqwest` (async) | Téléchargement des fichiers |
| HTML Parsing | `scraper` | Extraction de la date de mise à jour |
| Encodage | `encoding_rs` | Décodeage CP1252 / UTF-8 |
| CSV/TSV | Manuel (split) | Parsing TSV (pas de quoting dans BDPM) |
| SQLite | `rusqlite` | Base de données locale |
| Hash | `sha2` | Calcul SHA-256 pour détection de changements |
| Date/Time | `chrono` | Manipulation des dates |
| Regex | `regex` | Nettoyage HTML, extraction de patterns |
| CLI | `clap` | Interface en ligne de commande |
| Logging | `tracing` | Journalisation structurée |
| Config | `config` | Fichier de configuration |
| Async Runtime | `tokio` | Runtime asynchrone |
| Sérialisation | `serde` + `serde_json` | Export JSON, API |

---

## 3. Structure du projet

```
bdpm-importer/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .env.example
├── config.toml.example
│
├── src/
│   ├── main.rs              # Point d'entrée CLI
│   ├── lib.rs                # Déclarations publiques du crate
│   │
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs       # Configuration (fichier + env)
│   │
│   ├── fetch/
│   │   ├── mod.rs
│   │   ├── downloader.rs     # Téléchargement HTTP avec retry
│   │   └── update_checker.rs # Vérification de la date de mise à jour
│   │
│   ├── decode/
│   │   ├── mod.rs
│   │   └── encoder.rs        # Détection et conversion d'encodage
│   │
│   ├── parse/
│   │   ├── mod.rs
│   │   ├── tsv_parser.rs     # Parseur TSV générique
│   │   ├── schemas.rs        # Définitions des schémas de fichiers
│   │   └── validators.rs     # Validation des champs
│   │
│   ├── transform/
│   │   ├── mod.rs
│   │   ├── normalizer.rs     # Normalisation (dates, prix, taux)
│   │   └── html_cleaner.rs   # Nettoyage du HTML
│   │
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs         # DDL SQLite (CREATE TABLE, INDEX)
│   │   ├── migrator.rs       # Migrations de schéma
│   │   ├── importer.rs       # Import par fichier
│   │   └── queries.rs        # Requêtes utilitaires
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   ├── specialite.rs     # Struct Specialite
│   │   ├── presentation.rs   # Struct Presentation
│   │   ├── composition.rs    # Struct Composition
│   │   ├── avis.rs           # Struct AvisSmr, AvisAsmr
│   │   ├── generique.rs      # Struct GroupeGenerique
│   │   ├── condition.rs      # Struct ConditionPrescription
│   │   ├── disponibilite.rs  # Struct Disponibilite
│   │   ├── mitm.rs           # Struct Mitm
│   │   └── info_importante.rs # Struct InfoImportante
│   │
│   └── pipeline/
│       ├── mod.rs
│       └── orchestrator.rs   # Orchestration du pipeline complet
│
├── migrations/
│   ├── 001_initial.sql       # Schéma initial
│   └── 002_add_indexes.sql   # Index secondaires
│
├── tests/
│   ├── integration/
│   │   ├── test_fetch.rs
│   │   ├── test_parse.rs
│   │   ├── test_transform.rs
│   │   └── test_full_pipeline.rs
│   └── fixtures/
│       ├── CIS_bdpm_sample.txt
│       ├── CIS_CIP_bdpm_sample.txt
│       └── ...
│
└── scripts/
    ├── schedule_cron.sh      # Configuration cron pour les mises à jour
    └── validate_db.py        # Script de validation Python
```

---

## 4. Conception détaillée

### 4.1 Module `config`

```rust
// src/config/settings.rs

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub fetch: FetchConfig,
    pub schedule: ScheduleConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,                          // Chemin vers le fichier SQLite
    pub wal_mode: bool,                        // Activer le mode WAL
    pub busy_timeout_ms: u32,                  // Timeout pour les locks
}

#[derive(Debug, Deserialize, Clone)]
pub struct FetchConfig {
    pub base_url: String,                      // URL de base BDPM
    pub user_agent: String,                    // User-Agent HTTP
    pub request_delay_secs: u64,               // Délai entre les requêtes
    pub max_retries: u32,                      // Tentatives max
    pub retry_delay_secs: u64,                 // Délai entre les tentatives
    pub respect_robots_txt: bool,              // Respecter robots.txt
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleConfig {
    pub daily_check_hour: u32,                 // Heure de vérification quotidienne
    pub weekly_check_day: String,              // Jour de vérification hebdomadaire
    pub monthly_check_day: u32,                // Jour du mois pour vérification mensuelle
}
```

### 4.2 Module `fetch`

```rust
// src/fetch/downloader.rs

pub struct BdpmDownloader {
    client: reqwest::Client,
    config: FetchConfig,
}

impl BdpmDownloader {
    pub fn new(config: FetchConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { client, config })
    }

    /// Télécharge un fichier BDPM avec retry et respect du délai
    pub async fn download(&self, file_source: &FileSource) -> Result<RawFile> {
        let url = format!("{}{}", self.config.base_url, file_source.url);
        let data = self.download_with_retry(&url).await?;
        let hash = compute_sha256(&data);

        Ok(RawFile {
            source: file_source.clone(),
            data,
            hash,
            downloaded_at: Utc::now(),
        })
    }

    async fn download_with_retry(&self, url: &str) -> Result<Vec<u8>> {
        let mut attempts = 0;
        loop {
            attempts += 1;
            match self.client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let data = resp.bytes().await?.to_vec();
                    tokio::time::sleep(Duration::from_secs(self.config.request_delay_secs)).await;
                    return Ok(data);
                }
                Ok(resp) if resp.status() == reqwest::StatusCode::NOT_MODIFIED => {
                    return Err(anyhow!("Not modified"));
                }
                Ok(resp) => {
                    warn!("HTTP {} for {} (attempt {}/{})",
                        resp.status(), url, attempts, self.config.max_retries);
                }
                Err(e) => {
                    warn!("Error for {} (attempt {}/{}): {}",
                        url, attempts, self.config.max_retries, e);
                }
            }
            if attempts >= self.config.max_retries {
                bail!("Failed after {} attempts: {}", self.config.max_retries, url);
            }
            tokio::time::sleep(Duration::from_secs(self.config.retry_delay_secs)).await;
        }
    }
}
```

### 4.3 Module `decode`

```rust
// src/decode/encoder.rs

pub struct BdpmDecoder;

impl BdpmDecoder {
    /// Décode les octets bruts en String UTF-8
    /// Stratégie : UTF-8 d'abord, puis CP1252 en fallback
    pub fn decode(raw: &[u8]) -> DecodedFile {
        // Tentative UTF-8
        if let Ok(text) = std::str::from_utf8(raw) {
            return DecodedFile {
                text: text.to_string(),
                detected_encoding: Encoding::Utf8,
                had_errors: false,
            };
        }

        // Fallback Windows-1252
        let (cow, _encoding, had_errors) = encoding_rs::WINDOWS_1252.decode(raw);
        DecodedFile {
            text: cow.into_owned(),
            detected_encoding: Encoding::Windows1252,
            had_errors,
        }
    }

    /// Normalise les fins de ligne
    pub fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    Utf8,
    Windows1252,
    Ascii,
}

pub struct DecodedFile {
    pub text: String,
    pub detected_encoding: Encoding,
    pub had_errors: bool,
}
```

### 4.4 Module `parse`

```rust
// src/parse/schemas.rs

pub struct FileSchema {
    pub name: &'static str,
    pub url: &'static str,
    pub expected_fields: usize,
    pub frequency: CheckFrequency,
    pub fields: &'static [FieldSchema],
}

pub struct FieldSchema {
    pub name: &'static str,
    pub field_type: FieldType,
    pub required: bool,
}

pub enum FieldType {
    Integer,
    Text,
    DateDdMmYyyy,
    DateYyyymmdd,
    Decimal,          // Virgule comme séparateur
    Percentage,       // Taux de remboursement
    SemicolonList,    // Valeurs séparées par ;
    HtmlText,         // Peut contenir du HTML
    Url,
    Enum(&'static [&'static str]),
}

pub const SCHEMAS: &[FileSchema] = &[
    FileSchema {
        name: "CIS_bdpm.txt",
        url: "/download/file/CIS_bdpm.txt",
        expected_fields: 12,
        frequency: CheckFrequency::Weekly,
        fields: &[
            FieldSchema { name: "code_cis",           field_type: FieldType::Integer,      required: true },
            FieldSchema { name: "denomination",        field_type: FieldType::Text,         required: true },
            FieldSchema { name: "forme_pharma",        field_type: FieldType::Text,         required: true },
            FieldSchema { name: "voies_admin",         field_type: FieldType::SemicolonList,required: true },
            FieldSchema { name: "statut_amm",          field_type: FieldType::Text,         required: true },
            FieldSchema { name: "type_procedure",      field_type: FieldType::Text,         required: true },
            FieldSchema { name: "etat_commercial",     field_type: FieldType::Text,         required: true },
            FieldSchema { name: "date_amm",            field_type: FieldType::DateDdMmYyyy, required: false },
            FieldSchema { name: "statut_bdm",          field_type: FieldType::Enum(&["Alerte", "Warning disponibilité"]), required: false },
            FieldSchema { name: "num_europe",          field_type: FieldType::Text,         required: false },
            FieldSchema { name: "titulaires",          field_type: FieldType::SemicolonList,required: false },
            FieldSchema { name: "surveillance",        field_type: FieldType::Enum(&["Oui", "Non"]), required: true },
        ],
    },
    // ... autres schémas
];
```

### 4.5 Module `transform`

```rust
// src/transform/normalizer.rs

pub struct DataNormalizer;

impl DataNormalizer {
    /// Normalise une date BDPM en NaiveDate
    pub fn normalize_date(input: &str, format_hint: DateHint) -> Option<NaiveDate> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        match format_hint {
            DateHint::DdMmYyyy => NaiveDate::parse_from_str(input, "%d/%m/%Y").ok(),
            DateHint::Yyyymmdd => {
                if input.len() == 8 && input.chars().all(|c| c.is_ascii_digit()) {
                    NaiveDate::parse_from_str(input, "%Y%m%d").ok()
                } else {
                    None
                }
            }
            DateHint::Auto => {
                // Essayer les deux formats
                Self::normalize_date(input, DateHint::DdMmYyyy)
                    .or_else(|| Self::normalize_date(input, DateHint::Yyyymmdd))
            }
        }
    }

    /// Convertit un prix français (virgule) en f64
    pub fn parse_price(input: &str) -> Option<f64> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        input.replace(',', ".").parse().ok()
    }

    /// Extrait le pourcentage d'un taux de remboursement
    pub fn parse_taux(input: &str) -> Option<u8> {
        let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse().ok()
    }

    /// Normalise les champs multi-valués (séparateur ;)
    pub fn parse_semicolon_list(input: &str) -> Vec<String> {
        input.split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

pub enum DateHint {
    DdMmYyyy,
    Yyyymmdd,
    Auto,
}
```

### 4.6 Module `db`

```rust
// src/db/importer.rs

pub struct BdpmImporter<'a> {
    conn: &'a Connection,
}

impl<'a> BdpmImporter<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Importe un fichier complet dans une transaction
    pub fn import_file(&self, file_name: &str, rows: &[Vec<String>]) -> Result<ImportStats> {
        let tx = self.conn.transaction()?;

        // Supprimer les anciennes données du fichier
        self.clear_table_for_file(&tx, file_name)?;

        let stats = match file_name {
            "CIS_bdpm.txt" => self.import_specialites(&tx, rows)?,
            "CIS_CIP_bdpm.txt" => self.import_presentations(&tx, rows)?,
            "CIS_COMPO_bdpm.txt" => self.import_compositions(&tx, rows)?,
            "CIS_HAS_SMR_bdpm.txt" => self.import_avis_smr(&tx, rows)?,
            "CIS_HAS_ASMR_bdpm.txt" => self.import_avis_asmr(&tx, rows)?,
            "HAS_LiensPageCT_bdpm.txt" => self.import_has_liens(&tx, rows)?,
            "CIS_GENER_bdpm.txt" => self.import_generiques(&tx, rows)?,
            "CIS_CPD_bdpm.txt" => self.import_conditions(&tx, rows)?,
            "CIS_CIP_Dispo_Spec.txt" => self.import_disponibilites(&tx, rows)?,
            "CIS_MITM.txt" => self.import_mitm(&tx, rows)?,
            "CIS_InfoImportantes.txt" => self.import_infos_importantes(&tx, rows)?,
            _ => bail!("Unknown file: {}", file_name),
        };

        tx.commit()?;
        Ok(stats)
    }

    fn clear_table_for_file(&self, tx: &Transaction, file_name: &str) -> Result<()> {
        match file_name {
            "CIS_bdpm.txt" => tx.execute("DELETE FROM specialites", [])?,
            "CIS_CIP_bdpm.txt" => tx.execute("DELETE FROM presentations", [])?,
            // ... etc.
            _ => bail!("Unknown file for clear: {}", file_name),
        };
        Ok(())
    }
}

pub struct ImportStats {
    pub rows_imported: usize,
    pub rows_skipped: usize,
    pub warnings: Vec<String>,
}
```

---

## 5. Interface CLI

### 5.1 Commandes

```bash
# Import complet (télécharger + parser + insérer)
bdpm-importer import --full

# Import incrémental (vérifier les mises à jour uniquement)
bdpm-importer import --incremental

# Import d'un seul fichier
bdpm-importer import --file CIS_CIP_bdpm.txt

# Vérifier si des mises à jour sont disponibles
bdpm-importer check-updates

# Valider la base de données
bdpm-importer validate

# Exporter en JSON
bdpm-importer export --format json --output bdpm.json

# Statistiques de la base
bdpm-importer stats

# Lancer le scheduler (vérifications automatiques)
bdpm-importer serve --port 8080
```

### 5.2 Configuration clap

```rust
// src/main.rs

#[derive(Parser)]
#[command(name = "bdpm-importer")]
#[command(about = "Import and manage BDPM medication database")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import BDPM data into SQLite
    Import {
        /// Full import (re-download everything)
        #[arg(long)]
        full: bool,

        /// Incremental import (check for updates only)
        #[arg(long)]
        incremental: bool,

        /// Import a specific file
        #[arg(long)]
        file: Option<String>,

        /// Database path
        #[arg(long, default_value = "bdpm.db")]
        db: String,
    },

    /// Check if updates are available
    CheckUpdates {
        #[arg(long, default_value = "bdpm.db")]
        db: String,
    },

    /// Validate database integrity
    Validate {
        #[arg(long, default_value = "bdpm.db")]
        db: String,
    },

    /// Export database to various formats
    Export {
        #[arg(long, default_value = "json")]
        format: String,

        #[arg(long)]
        output: Option<String>,

        #[arg(long, default_value = "bdpm.db")]
        db: String,
    },

    /// Show database statistics
    Stats {
        #[arg(long, default_value = "bdpm.db")]
        db: String,
    },

    /// Start the update scheduler
    Serve {
        #[arg(long, default_value = "8080")]
        port: u16,
    },
}
```

---

## 6. Tests

### 6.1 Tests unitaires

Chaque module a ses propres tests unitaires :

- **decode** : Tester avec des bytes CP1252, UTF-8, et mélangés
- **parse** : Tester avec des lignes TSV valides et malformées
- **transform** : Tester la normalisation des dates, prix, taux
- **db** : Tester les insertions avec des données de test

### 6.2 Tests d'intégration

```rust
#[tokio::test]
async fn test_full_pipeline() {
    // Utiliser les fixtures (fichiers samples)
    let raw = std::fs::read("tests/fixtures/CIS_bdpm_sample.txt").unwrap();

    // Pipeline complet
    let decoded = BdpmDecoder::decode(&raw);
    let text = BdpmDecoder::normalize_line_endings(&decoded.text);
    let parsed = TsvParser::parse(&text, 12).unwrap();
    let conn = Connection::open_in_memory().unwrap();
    create_schema(&conn).unwrap();

    let importer = BdpmImporter::new(&conn);
    let stats = importer.import_file("CIS_bdpm.txt", &parsed.rows).unwrap();

    assert!(stats.rows_imported > 0);
    assert_eq!(stats.rows_skipped, 0);
}
```

### 6.3 Fixtures

Les fichiers fixtures sont des extraits des vrais fichiers BDPM (100-200 lignes), suffisants pour tester le parsing sans télécharger l'intégralité des données.

---

## 7. Dépendances Cargo.toml

```toml
[package]
name = "bdpm-importer"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async
tokio = { version = "1", features = ["full"] }

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# HTML parsing
scraper = "0.20"

# Encoding
encoding_rs = "0.8"

# Database
rusqlite = { version = "0.31", features = ["bundled"] }

# Crypto
sha2 = "0.10"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# Regex
regex = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Config
config = "0.14"

# Error handling
anyhow = "1"
thiserror = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## 8. Plan de développement

### Phase 1 : Fondations (Semaine 1-2)
- [x] Structure du projet
- [ ] Module `config` (lecture fichier TOML + variables d'environnement)
- [ ] Module `decode` (détection d'encodage, conversion CP1252/UTF-8)
- [ ] Module `parse` (parseur TSV, schémas de fichiers)
- [ ] Module `transform` (normalisation dates, prix, HTML)
- [ ] Tests unitaires pour chaque module

### Phase 2 : Base de données (Semaine 3)
- [ ] Module `db` (schéma SQLite, migrations, import)
- [ ] Module `models` (structs Rust pour chaque entité)
- [ ] Tests d'intégration avec fixtures

### Phase 3 : Pipeline (Semaine 4)
- [ ] Module `fetch` (téléchargement avec retry, vérification de date)
- [ ] Module `pipeline` (orchestration complète)
- [ ] CLI avec clap (commandes import, validate, stats)

### Phase 4 : Mise à jour automatique (Semaine 5)
- [ ] Scheduler (vérification quotidienne/hebdomadaire)
- [ ] Comparaison par hash SHA-256
- [ ] Import incrémental
- [ ] Logging et alertes

### Phase 5 : API et export (Semaine 6)
- [ ] Serveur HTTP (Actix-web ou Axum) avec endpoints REST
- [ ] Export JSON/CSV
- [ ] Documentation API (OpenAPI)

---

## 9. Considérations futures

### 9.1 Extensions possibles
- **Full-text search** : Utiliser la FTS5 de SQLite pour la recherche en texte intégral
- **RCP/SmPC scraping** : Extraire les Résumés des Caractéristiques du Produit depuis le site BDPM
- **API web** : Exposer les données via une API REST locale
- **Notifications** : Envoyer des emails/webhooks lors de ruptures de stock
- **Historique** : Conserver les versions historiques des données pour suivi des changements

### 9.2 Sources de données complémentaires
- **ANSM** : Informations de pharmacovigilance
- **HAS** : Avis complets de la Commission de la Transparence
- **Ameli** : Taux de remboursement et conditions
- **Thériaque** : Base de données indépendante sur les médicaments
- **Vidal** : Données complémentaires (sous licence)
