# 06 — Architecture du pipeline Rust

---

## 6.1 Vue d'ensemble : 5 crates

L'architecture se décompose en 5 crates Rust organisés en pipeline séquentiel, chacun avec une responsabilité unique et une interface claire. Cette décomposition permet le développement incrémental, les tests unitaires isolés et la compilation parallèle.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  bdpm-core  │◄────│  bdpm-fetch │◄────│  bdpm-parse │◄────│bdpm-validate│◄────│   bdpm-db   │
│  (types,    │     │  (HTTP,     │     │  (décodage, │     │  (checks    │     │  (SQLite,   │
│   config,   │     │   SHA-256,  │     │   split,    │     │   ref, enum,│     │   insert,   │
│   traits)   │     │   archive)  │     │   normalize)│     │   coherence)│     │   log)      │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
      ▲                                                                                   │
      └─────────────────── Types partagés + Config ───────────────────────────────────────┘
```

---

## 6.2 Crate bdpm-core

**Responsabilité** : Définir les structures de données partagées, les traits, les types énumérations et la configuration.

### Types d'énumération

```rust
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "title_case")]
pub enum StatutBdm {
    #[strum(serialize = "")]
    Aucun,
    Alerte,
    #[strum(serialize = "Warning disponibilité")]
    WarningDisponibilite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
pub enum NatureComposant {
    #[strum(serialize = "SA")]
    SubstanceActive,
    #[strum(serialize = "FT")]
    FractionTherapeutique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
pub enum TypeGenerique {
    Princeps = 0,
    Generique = 1,
    ComplementaritePosologique = 2,
    GeneriqueSubstituable = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
pub enum CodeStatutDispo {
    RuptureDeStock = 1,
    TensionApprovisionnement = 2,
    ArretCommercialisation = 3,
    RemiseADisposition = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
pub enum SurveillanceRenforcee {
    Oui,
    Non,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
pub enum AgrementCollectivites {
    Oui,
    Non,
    Inconnu,
}
```

### Format de date

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    DdMmYyyy,   // DD/MM/YYYY — utilisé par la plupart des fichiers
    Yyyymmdd,   // YYYYMMDD — utilisé par les fichiers HAS
}
```

### Configuration par fichier

```rust
#[derive(Debug, Clone)]
pub struct FileConfig {
    pub filename: &'static str,
    pub url_path: &'static str,
    pub encoding: FileEncoding,
    pub expected_columns: usize,
    pub date_format: DateFormat,
    pub is_dynamic: bool,  // true pour CIS_InfoImportantes
}

#[derive(Debug, Clone, Copy)]
pub enum FileEncoding {
    Windows1252,
    Iso8859_1,
    Utf8,
}

pub const FILE_CONFIGS: &[FileConfig] = &[
    FileConfig { filename: "CIS_bdpm.txt", url_path: "/download/file/CIS_bdpm.txt", encoding: FileEncoding::Windows1252, expected_columns: 12, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_CIP_bdpm.txt", url_path: "/download/file/CIS_CIP_bdpm.txt", encoding: FileEncoding::Utf8, expected_columns: 13, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_COMPO_bdpm.txt", url_path: "/download/file/CIS_COMPO_bdpm.txt", encoding: FileEncoding::Windows1252, expected_columns: 8, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_HAS_SMR_bdpm.txt", url_path: "/download/file/CIS_HAS_SMR_bdpm.txt", encoding: FileEncoding::Windows1252, expected_columns: 6, date_format: DateFormat::Yyyymmdd, is_dynamic: false },
    FileConfig { filename: "CIS_HAS_ASMR_bdpm.txt", url_path: "/download/file/CIS_HAS_ASMR_bdpm.txt", encoding: FileEncoding::Windows1252, expected_columns: 6, date_format: DateFormat::Yyyymmdd, is_dynamic: false },
    FileConfig { filename: "HAS_LiensPageCT_bdpm.txt", url_path: "/download/file/HAS_LiensPageCT_bdpm.txt", encoding: FileEncoding::Utf8, expected_columns: 2, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_GENER_bdpm.txt", url_path: "/download/file/CIS_GENER_bdpm.txt", encoding: FileEncoding::Windows1252, expected_columns: 5, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_CPD_bdpm.txt", url_path: "/download/file/CIS_CPD_bdpm.txt", encoding: FileEncoding::Windows1252, expected_columns: 2, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_CIP_Dispo_Spec.txt", url_path: "/download/file/CIS_CIP_Dispo_Spec.txt", encoding: FileEncoding::Iso8859_1, expected_columns: 8, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_MITM.txt", url_path: "/download/file/CIS_MITM.txt", encoding: FileEncoding::Windows1252, expected_columns: 4, date_format: DateFormat::DdMmYyyy, is_dynamic: false },
    FileConfig { filename: "CIS_InfoImportantes.txt", url_path: "/download/CIS_InfoImportantes.txt", encoding: FileEncoding::Utf8, expected_columns: 4, date_format: DateFormat::DdMmYyyy, is_dynamic: true },
];
```

### Structures de données typées

```rust
use chrono::NaiveDate;
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct Specialite {
    pub code_cis: String,
    pub denomination: String,
    pub forme_pharmaceutique: Option<String>,
    pub voies_administration: Option<Vec<String>>,  // Split sur ";"
    pub statut_amm: Option<String>,
    pub type_procedure_amm: Option<String>,
    pub etat_commercialisation: Option<String>,
    pub date_amm: Option<NaiveDate>,
    pub date_amm_raw: Option<String>,
    pub statut_bdm: StatutBdm,
    pub numero_autorisation_euro: Option<String>,
    pub titulaires: Option<Vec<String>>,  // Split sur ";"
    pub surveillance_renforcee: SurveillanceRenforcee,
}

#[derive(Debug, Clone)]
pub struct Presentation {
    pub code_cis: String,
    pub code_cip7: String,
    pub libelle: Option<String>,
    pub statut_administratif: Option<String>,
    pub etat_commercialisation: Option<String>,
    pub date_declaration: Option<NaiveDate>,
    pub date_declaration_raw: Option<String>,
    pub code_cip13: Option<String>,
    pub agrement_collectivites: Option<AgrementCollectivites>,
    pub taux_remboursement: Option<String>,
    pub prix_ht: Option<Decimal>,
    pub prix_ttc: Option<Decimal>,
    pub honoraires: Option<Decimal>,
    pub indications_remboursement: Option<String>,
}

// ... structures similaires pour chaque fichier
```

### Trait de validation

```rust
pub trait BdpmRecord: Sized {
    /// Parse une ligne de champs en un enregistrement typé
    fn from_fields(fields: &[String], config: &FileConfig) -> Result<Self, ParseError>;
    
    /// Valide la cohérence sémantique de l'enregistrement
    fn validate(&self) -> Vec<ValidationWarning>;
    
    /// Retourne le Code CIS (si applicable)
    fn code_cis(&self) -> Option<&str>;
}
```

---

## 6.3 Crate bdpm-fetch

**Responsabilité** : Téléchargement, gestion HTTP, archivage, calcul de hash.

### Dépendances Rust

```toml
[dependencies]
bdpm-core = { path = "../bdpm-core" }
reqwest = { version = "0.12", features = ["rustls-tls"] }
tokio = { version = "1", features = ["full"] }
sha2 = "0.10"
chrono = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
```

### Interface principale

```rust
pub struct Fetcher {
    client: reqwest::Client,
    config: FetchConfig,
}

impl Fetcher {
    pub fn new(config: FetchConfig) -> Result<Self>;
    
    /// Télécharge un fichier et retourne les données brutes + métadonnées
    pub async fn fetch_file(&self, file_config: &FileConfig) -> Result<FetchedFile>;
    
    /// Télécharge tous les fichiers avec intervalle entre les requêtes
    pub async fn fetch_all(&self) -> Result<Vec<FetchedFile>>;
    
    /// Vérifie si un fichier a changé depuis le dernier import (comparaison SHA-256)
    pub async fn has_changed(&self, file_config: &FileConfig, previous_hash: &str) -> Result<bool>;
    
    /// Archive le fichier brut avec horodatage
    pub fn archive(&self, file: &FetchedFile) -> Result<PathBuf>;
}

pub struct FetchedFile {
    pub config: &'static FileConfig,
    pub data: Vec<u8>,
    pub sha256: String,
    pub size_bytes: u64,
    pub http_status: u16,
    pub content_disposition: Option<String>,
    pub download_duration: Duration,
}
```

### Gestion des encodages avec encoding_rs

```rust
use encoding_rs::{WINDOWS_1252, ISO_8859_1, UTF_8};

pub fn decode_bytes(data: &[u8], encoding: FileEncoding) -> String {
    let cow = match encoding {
        FileEncoding::Windows1252 => WINDOWS_1252.decode_without_bom_handling(data),
        FileEncoding::Iso8859_1 => ISO_8859_1.decode_without_bom_handling(data),
        FileEncoding::Utf8 => UTF_8.decode_without_bom_handling(data),
    };
    // Normaliser les fins de ligne : supprimer tous les \r
    cow.replace('\r', "")
}
```

---

## 6.4 Crate bdpm-parse

**Responsabilité** : Décodage, split tabulation, validation structurelle, normalisation par champ.

### Dépendances Rust

```toml
[dependencies]
bdpm-core = { path = "../bdpm-core" }
encoding_rs = "0.8"
chrono = "0.4"
rust_decimal = "1"
scraper = "0.19"  # Pour le parsing HTML de CIS_InfoImportantes
thiserror = "1"
tracing = "0.1"
```

### Interface principale

```rust
pub struct Parser;

impl Parser {
    /// Parse un fichier complet et retourne les enregistrements typés
    pub fn parse<T: BdpmRecord>(&self, data: &[u8], config: &FileConfig) -> Result<ParseResult<T>>;
}

pub struct ParseResult<T> {
    pub records: Vec<T>,
    pub rows_read: usize,
    pub rows_skipped: usize,
    pub warnings: Vec<ParseWarning>,
}

pub struct ParseWarning {
    pub line_number: usize,
    pub field: Option<usize>,
    pub message: String,
    pub raw_content: String,
}
```

### Normalisation des champs

```rust
impl Parser {
    fn normalize_date(value: &str, format: DateFormat) -> (Option<NaiveDate>, Option<String>) {
        let trimmed = value.trim();
        if trimmed.is_empty() { return (None, None); }
        let date = match format {
            DateFormat::DdMmYyyy => NaiveDate::parse_from_str(trimmed, "%d/%m/%Y"),
            DateFormat::Yyyymmdd => NaiveDate::parse_from_str(trimmed, "%Y%m%d"),
        };
        match date {
            Ok(d) => (Some(d), Some(trimmed.to_string())),
            Err(_) => (None, Some(trimmed.to_string())),  // Conserver la valeur brute
        }
    }
    
    fn normalize_decimal(value: &str) -> (Option<Decimal>, Option<String>) {
        let trimmed = value.trim();
        if trimmed.is_empty() { return (None, None); }
        let normalized = trimmed.replace(',', ".");
        match normalized.parse::<Decimal>() {
            Ok(d) => (Some(d), Some(trimmed.to_string())),
            Err(_) => (None, Some(trimmed.to_string())),
        }
    }
    
    fn normalize_apostrophes(value: &str) -> String {
        value.replace('\u{2019}', "'")   // Right single quotation mark
             .replace('\u{2018}', "'")   // Left single quotation mark
    }
    
    fn parse_html_content(html: &str) -> (String, Option<String>) {
        // Extraire le texte et l'URL des balises <a>
        // Décoder les entités HTML
        // Retourner (texte, url_optionnelle)
    }
    
    fn split_multi_value(value: &str) -> Vec<String> {
        if value.trim().is_empty() { return vec![]; }
        value.split(';')
             .map(|s| s.trim().to_string())
             .filter(|s| !s.is_empty())
             .collect()
    }
}
```

---

## 6.5 Crate bdpm-validate

**Responsabilité** : Vérification sémantique, contrôles référentiels, détection de régressions.

### Dépendances Rust

```toml
[dependencies]
bdpm-core = { path = "../bdpm-core" }
thiserror = "1"
tracing = "0.1"
```

### Interface principale

```rust
pub struct Validator {
    referentiel_cis: HashSet<String>,  // Codes CIS de la table maîtresse
    previous_stats: Option<ImportStats>,
}

impl Validator {
    pub fn new(referentiel_cis: HashSet<String>) -> Self;
    
    /// Valide un enregistrement individuel
    pub fn validate_record<T: BdpmRecord>(&self, record: &T) -> Vec<ValidationWarning>;
    
    /// Vérifie si un Code CIS est orphelin
    pub fn is_orphan(&self, code_cis: &str) -> bool;
    
    /// Génère un rapport de validation pour un import complet
    pub fn generate_report(&self, results: &[ValidationResult]) -> ValidationReport;
}

pub struct ValidationReport {
    pub total_records: usize,
    pub valid_records: usize,
    pub warnings: Vec<ValidationWarning>,
    pub orphan_count: usize,
    pub enum_violations: Vec<EnumViolation>,
    pub regression_alerts: Vec<RegressionAlert>,
}
```

---

## 6.6 Crate bdpm-db

**Responsabilité** : Insertion SQLite, transactions, migrations de schéma, import_log.

### Dépendances Rust

```toml
[dependencies]
bdpm-core = { path = "../bdpm-core" }
rusqlite = { version = "0.31", features = ["bundled"] }
refinery = { version = "0.8", features = ["rusqlite"] }
chrono = "0.4"
thiserror = "1"
tracing = "0.1"
```

### Migrations avec refinery

Les migrations sont des fichiers SQL numérotés dans le répertoire `migrations/` :

```
migrations/
├── V001__initial_schema.sql
├── V002__add_fulltext_search.sql
└── V003__add_content_hash.sql
```

### Interface principale

```rust
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    /// Ouvre la base et exécute les migrations en attente
    pub fn open(path: &Path) -> Result<Self>;
    
    /// Configure les PRAGMA
    fn configure_pragmas(&self) -> Result<()>;
    
    /// Insère ou met à jour un enregistrement de spécialité
    pub fn upsert_specialite(&self, record: &Specialite, import_id: i64) -> Result<UpsertResult>;
    
    /// Marque les enregistrements absents comme inactifs (soft delete)
    pub fn soft_delete_missing(&self, table: &str, active_codes: &[String], import_id: i64) -> Result<usize>;
    
    /// Crée un enregistrement dans import_log
    pub fn log_import(&self, log: &ImportLogEntry) -> Result<i64>;
    
    /// Exécute une transaction pour un fichier complet
    pub fn import_file<T: BdpmRecord>(
        &self,
        records: &[T],
        file_name: &str,
        import_id: i64,
    ) -> Result<ImportStats>;
    
    /// Exécute les checks de qualité post-import
    pub fn run_quality_checks(&self) -> Result<QualityReport>;
    
    /// Retourne le set des Codes CIS actifs (pour la validation)
    pub fn active_cis_codes(&self) -> Result<HashSet<String>>;
}

pub enum UpsertResult {
    Inserted,
    Updated,
    Unchanged,
}
```

### Performance d'insertion

```rust
impl Database {
    pub fn import_batch<T: BdpmRecord>(
        &self,
        records: &[T],
        import_id: i64,
    ) -> Result<ImportStats> {
        let mut stats = ImportStats::default();
        
        // Batch size de 1000 pour équilibrer mémoire et performance
        for chunk in records.chunks(1000) {
            let tx = self.conn.unchecked_transaction()?;
            
            for record in chunk {
                match self.upsert_record(record, import_id)? {
                    UpsertResult::Inserted => stats.rows_inserted += 1,
                    UpsertResult::Updated => stats.rows_updated += 1,
                    UpsertResult::Unchanged => {}
                }
            }
            
            tx.commit()?;
        }
        
        Ok(stats)
    }
}
```

---

## 6.7 Binaire CLI principal

Le binaire principal orchestre les 5 crates :

```rust
// main.rs
use bdpm_core::FILE_CONFIGS;
use bdpm_fetch::Fetcher;
use bdpm_parse::Parser;
use bdpm_validate::Validator;
use bdpm_db::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Ouvrir la base
    let db = Database::open("bdpm.db")?;
    
    // 2. Créer le fetcher
    let fetcher = Fetcher::new(FetchConfig::default())?;
    
    // 3. Télécharger les fichiers
    let fetched = fetcher.fetch_all().await?;
    
    // 4. Parser et valider
    let parser = Parser::new();
    let referentiel_cis = db.active_cis_codes()?;
    let validator = Validator::new(referentiel_cis);
    
    for file in fetched {
        if !file.has_changed() { continue; }
        
        // Parser selon le type de fichier
        let records = parser.parse_file(&file)?;
        
        // Valider
        let validated = validator.validate(&records)?;
        
        // Insérer en base
        let import_id = db.log_import(&file)?;
        db.import_file(&validated, &file.config.filename, import_id)?;
        
        // Archiver
        fetcher.archive(&file)?;
    }
    
    // 5. Exécuter les checks qualité
    let report = db.run_quality_checks()?;
    println!("{:?}", report);
    
    Ok(())
}
```

---

## 6.8 Dépendances résumées

| Crate | Dépendances Rust | Rôle |
|-------|-----------------|------|
| bdpm-core | strum, serde, chrono, rust_decimal, thiserror | Types, config, traits |
| bdpm-fetch | reqwest, tokio, sha2, chrono, serde_json | HTTP, hash, archive |
| bdpm-parse | encoding_rs, chrono, rust_decimal, scraper | Décodage, split, normalisation |
| bdpm-validate | bdpm-core, thiserror, tracing | Vérification sémantique |
| bdpm-db | rusqlite, refinery, chrono, thiserror, tracing | SQLite, migrations, insertion |
