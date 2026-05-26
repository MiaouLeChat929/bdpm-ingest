# Schéma de Base de Données SQLite — BDPM

> Conception complète du schéma SQLite pour accueillir l'ensemble des données de la BDPM.
> Date : 26 mai 2026

---

## 1. Principes de conception

### 1.1 Objectifs

- **Fidélité** : Conserver l'intégralité des données source sans perte d'information
- **Intégrité** : Maintenir les relations entre fichiers via des clés étrangères
- **Performance** : Optimiser les requêtes courantes avec des index appropriés
- **Traçabilité** : Enregistrer la date d'import et les métadonnées de source
- **Cohérence** : Normaliser les encodages, dates et formats à l'import

### 1.2 Conventions

- Toutes les données texte stockées en **UTF-8**
- Toutes les dates stockées au format **ISO 8601** (`YYYY-MM-DD`)
- Les prix stockés en **REAL** (virgule → point à l'import)
- Les champs pouvant être vides sont **NULLABLE**
- Le Code CIS est la clé primaire de la table centrale
- Les clés étrangères sont **optionnelles** (orphelins possibles)

---

## 2. Schéma complet

### 2.1 Table centrale : `specialites`

```sql
CREATE TABLE specialites (
    code_cis          INTEGER PRIMARY KEY,
    denomination      TEXT NOT NULL,
    forme_pharma      TEXT NOT NULL,
    voies_admin       TEXT,            -- Séparateur : point-virgule
    statut_amm        TEXT,
    type_procedure    TEXT,
    etat_commercial   TEXT,
    date_amm          DATE,            -- Normalisé en YYYY-MM-DD
    statut_bdm        TEXT,            -- 'Alerte', 'Warning disponibilité', NULL
    num_europe        TEXT,            -- Numéro autorisation européenne
    titulaires        TEXT,            -- Séparateur : point-virgule
    surveillance      INTEGER DEFAULT 0, -- 0=Non, 1=Oui
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    _source_hash      TEXT             -- Hash SHA256 du ligne source
);
```

### 2.2 Table : `presentations`

```sql
CREATE TABLE presentations (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    code_cip7         TEXT,
    libelle           TEXT,
    statut_admin      TEXT,
    etat_commercial   TEXT,
    date_declaration  DATE,
    code_cip13        TEXT,
    agree_collect     TEXT,            -- 'oui', 'non', 'inconnu'
    taux_remboursement INTEGER,        -- Pourcentage normalisé (65, 100, etc.)
    prix_ht           REAL,            -- Euro, point décimal
    prix_ttc          REAL,
    honoraires        REAL,
    indications_raw   TEXT,            -- HTML brut
    indications_clean TEXT,            -- Texte nettoyé
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_presentations_cis ON presentations(code_cis);
CREATE INDEX idx_presentations_cip7 ON presentations(code_cip7);
CREATE INDEX idx_presentations_cip13 ON presentations(code_cip13);
```

### 2.3 Table : `compositions`

```sql
CREATE TABLE compositions (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    designation       TEXT,            -- Désignation élément pharmaceutique
    code_substance    TEXT,
    nom_substance     TEXT,
    dosage            TEXT,
    ref_dosage        TEXT,
    nature            TEXT,            -- 'SA' ou 'FT'
    num_liaison       INTEGER,
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_compositions_cis ON compositions(code_cis);
CREATE INDEX idx_compositions_substance ON compositions(code_substance);
CREATE INDEX idx_compositions_nature ON compositions(nature);
```

### 2.4 Table : `avis_smr`

```sql
CREATE TABLE avis_smr (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    code_dossier_has  TEXT,
    motif_eval        TEXT,
    date_avis         DATE,            -- Normalisé en YYYY-MM-DD
    valeur_smr        TEXT,
    libelle_raw       TEXT,            -- HTML brut
    libelle_clean     TEXT,            -- Texte nettoyé
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_avis_smr_cis ON avis_smr(code_cis);
CREATE INDEX idx_avis_smr_dossier ON avis_smr(code_dossier_has);
CREATE INDEX idx_avis_smr_valeur ON avis_smr(valeur_smr);
```

### 2.5 Table : `avis_asmr`

```sql
CREATE TABLE avis_asmr (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    code_dossier_has  TEXT,
    motif_eval        TEXT,
    date_avis         DATE,
    valeur_asmr       TEXT,            -- I, II, III, IV, V
    libelle_raw       TEXT,
    libelle_clean     TEXT,
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_avis_asmr_cis ON avis_asmr(code_cis);
CREATE INDEX idx_avis_asmr_dossier ON avis_asmr(code_dossier_has);
CREATE INDEX idx_avis_asmr_valeur ON avis_asmr(valeur_asmr);
```

### 2.6 Table : `has_liens_ct`

```sql
CREATE TABLE has_liens_ct (
    code_dossier_has  TEXT PRIMARY KEY,
    lien_ct           TEXT,            -- URL vers la page d'avis
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
```

### 2.7 Table : `groupes_generiques`

```sql
CREATE TABLE groupes_generiques (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    id_groupe         INTEGER,
    libelle_groupe    TEXT,
    code_cis          INTEGER,
    type_generique    INTEGER,         -- 0=princeps, 1=générique, 2=complémentarité, 4=substituable
    num_tri           INTEGER,
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_generiques_groupe ON groupes_generiques(id_groupe);
CREATE INDEX idx_generiques_cis ON groupes_generiques(code_cis);
CREATE INDEX idx_generiques_type ON groupes_generiques(type_generique);
```

### 2.8 Table : `conditions_prescription`

```sql
CREATE TABLE conditions_prescription (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    condition         TEXT,
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_conditions_cis ON conditions_prescription(code_cis);
```

### 2.9 Table : `disponibilites`

```sql
CREATE TABLE disponibilites (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    code_cip13        TEXT,
    code_statut       INTEGER,         -- 1=Rupture, 2=Tension, 3=Arrêt, 4=Remise
    libelle_statut    TEXT,
    date_debut        DATE,
    date_maj          DATE,
    date_fin          DATE,            -- NULL si événement en cours
    lien_ansm         TEXT,
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_disponibilites_cis ON disponibilites(code_cis);
CREATE INDEX idx_disponibilites_statut ON disponibilites(code_statut);
CREATE INDEX idx_disponibilites_date ON disponibilites(date_debut);
```

### 2.10 Table : `mitm`

```sql
CREATE TABLE mitm (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    code_atc          TEXT,
    denomination      TEXT,
    lien_bdpm         TEXT,
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_mitm_cis ON mitm(code_cis);
CREATE INDEX idx_mitm_atc ON mitm(code_atc);
```

### 2.11 Table : `infos_importantes`

```sql
CREATE TABLE infos_importantes (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis          INTEGER,
    date_debut        DATE,
    date_fin          DATE,            -- NULL si information en cours
    texte_raw         TEXT,            -- HTML brut
    texte_clean       TEXT,            -- Texte nettoyé
    _import_date      TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    _source_timestamp TEXT,            -- Timestamp du fichier source
    FOREIGN KEY (code_cis) REFERENCES specialites(code_cis)
);

CREATE INDEX idx_infos_cis ON infos_importantes(code_cis);
CREATE INDEX idx_infos_dates ON infos_importantes(date_debut, date_fin);
```

---

## 3. Tables de métadonnées

### 3.1 Table : `import_history`

```sql
CREATE TABLE import_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name   TEXT NOT NULL,
    import_date TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    rows_count  INTEGER,
    sha256      TEXT,          -- Hash SHA256 du fichier source
    file_size   INTEGER,      -- Taille en octets
    encoding    TEXT,          -- Encodage détecté (utf-8, cp1252)
    status      TEXT DEFAULT 'success',  -- 'success', 'partial', 'error'
    error_msg   TEXT
);
```

### 3.2 Table : `source_metadata`

```sql
CREATE TABLE source_metadata (
    key         TEXT PRIMARY KEY,
    value       TEXT,
    updated_at  TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Valeurs attendues :
-- 'last_update_date'  : '2026-04-28'
-- 'base_url'          : 'https://base-donnees-publique.medicaments.gouv.fr'
-- 'license'           : 'Licence Ouverte Etalab 2.0'
-- 'spec_version'      : 'v4'
-- 'last_check_date'   : '2026-05-26'
```

### 3.3 Vue : `cis_orphelins`

```sql
CREATE VIEW cis_orphelins AS
SELECT DISTINCT code_cis, 'avis_smr' AS source_table
FROM avis_smr WHERE code_cis NOT IN (SELECT code_cis FROM specialites)
UNION ALL
SELECT DISTINCT code_cis, 'avis_asmr'
FROM avis_asmr WHERE code_cis NOT IN (SELECT code_cis FROM specialites)
UNION ALL
SELECT DISTINCT code_cis, 'groupes_generiques'
FROM groupes_generiques WHERE code_cis NOT IN (SELECT code_cis FROM specialites)
UNION ALL
SELECT DISTINCT code_cis, 'presentations'
FROM presentations WHERE code_cis NOT IN (SELECT code_cis FROM specialites)
UNION ALL
SELECT DISTINCT code_cis, 'disponibilites'
FROM disponibilites WHERE code_cis NOT IN (SELECT code_cis FROM specialites);
```

---

## 4. Requêtes utilitaires

### 4.1 Recherche par dénomination

```sql
SELECT s.code_cis, s.denomination, s.forme_pharma, s.etat_commercial,
       GROUP_CONCAT(DISTINCT c.nom_substance) AS substances
FROM specialites s
LEFT JOIN compositions c ON s.code_cis = c.code_cis AND c.nature = 'SA'
WHERE s.denomination LIKE '%amoxicilline%'
GROUP BY s.code_cis;
```

### 4.2 Disponibilité d'un médicament

```sql
SELECT s.denomination, d.libelle_statut, d.date_debut, d.date_fin
FROM disponibilites d
JOIN specialites s ON d.code_cis = s.code_cis
WHERE d.code_statut IN (1, 2)  -- Rupture ou tension
  AND d.date_fin IS NULL;      -- Toujours en cours
```

### 4.3 Génériques d'une spécialité

```sql
SELECT g.id_groupe, g.libelle_groupe,
       s.code_cis, s.denomination,
       CASE g.type_generique
         WHEN 0 THEN 'Princeps'
         WHEN 1 THEN 'Générique'
         WHEN 2 THEN 'Complémentarité posologique'
         WHEN 4 THEN 'Substituable'
       END AS type
FROM groupes_generiques g
JOIN specialites s ON g.code_cis = s.code_cis
WHERE g.id_groupe IN (
    SELECT id_groupe FROM groupes_generiques
    WHERE code_cis = 60002283  -- Code CIS du princeps
);
```

### 4.4 Statistiques de couverture

```sql
SELECT
    (SELECT COUNT(*) FROM specialites) AS total_specialites,
    (SELECT COUNT(DISTINCT code_cis) FROM presentations) AS avec_presentation,
    (SELECT COUNT(DISTINCT code_cis) FROM compositions) AS avec_composition,
    (SELECT COUNT(DISTINCT code_cis) FROM avis_smr) AS avec_smr,
    (SELECT COUNT(DISTINCT code_cis) FROM avis_asmr) AS avec_asmr,
    (SELECT COUNT(DISTINCT code_cis) FROM groupes_generiques) AS avec_generique,
    (SELECT COUNT(DISTINCT code_cis) FROM conditions_prescription) AS avec_cpd,
    (SELECT COUNT(DISTINCT code_cis) FROM mitm) AS avec_mitm,
    (SELECT COUNT(DISTINCT code_cis) FROM disponibilites) AS avec_dispo;
```

---

## 5. Script de création complet

```sql
-- Activation des clés étrangères (SQLite ne les active pas par défaut)
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

-- [Insérer ici tous les CREATE TABLE ci-dessus]

-- Trigger pour nettoyer les tables associées lors d'un réimport
CREATE TRIGGER clean_before_import_specialites
BEFORE DELETE ON specialites
BEGIN
    DELETE FROM presentations WHERE code_cis = OLD.code_cis;
    DELETE FROM compositions WHERE code_cis = OLD.code_cis;
    DELETE FROM avis_smr WHERE code_cis = OLD.code_cis;
    DELETE FROM avis_asmr WHERE code_cis = OLD.code_cis;
    DELETE FROM groupes_generiques WHERE code_cis = OLD.code_cis;
    DELETE FROM conditions_prescription WHERE code_cis = OLD.code_cis;
    DELETE FROM disponibilites WHERE code_cis = OLD.code_cis;
    DELETE FROM mitm WHERE code_cis = OLD.code_cis;
    DELETE FROM infos_importantes WHERE code_cis = OLD.code_cis;
END;
```

---

## 6. Estimation de taille

| Table | Lignes estimées | Taille estimée (sans index) |
|-------|----------------|----------------------------|
| specialites | 15 848 | ~4 Mo |
| presentations | 20 903 | ~8 Mo |
| compositions | 32 389 | ~4 Mo |
| avis_smr | 15 257 | ~12 Mo |
| avis_asmr | 9 906 | ~12 Mo |
| has_liens_ct | 10 342 | ~1 Mo |
| groupes_generiques | 10 704 | ~2 Mo |
| conditions_prescription | 28 154 | ~2 Mo |
| disponibilites | 766 | ~200 Ko |
| mitm | 7 711 | ~2 Mo |
| infos_importantes | variable | ~1 Mo |
| **Total** | **~152 000** | **~48 Mo** |

Avec les index, la taille totale estimée de la base SQLite est d'environ **60-70 Mo**.
