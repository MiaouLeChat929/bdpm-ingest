# 05 — Schéma SQLite proposé

---

## 5.1 Principes de design

1. **1 fichier BDPM = 1 table SQLite** : Chaque fichier correspond exactement à une table, sans normalisation supplémentaire. Cela facilite la mise à jour incrémentale et le diagnostic.
2. **Types SQLite natifs** : INTEGER, TEXT, REAL. Les CHECK constraints assurent la validité des énumérations.
3. **Clés étrangères déclarées mais non strictes** : `PRAGMA foreign_keys = OFF` par défaut pour ne pas bloquer l'insertion des références orphelines.
4. **Colonnes `_raw`** : Pour les champs nécessitant une normalisation (dates, nombres), la valeur originale est conservée dans une colonne `_raw` suffixée.
5. **Traçabilité** : Chaque table possède une colonne `_import_id` référençant la table `import_log`.
6. **Soft delete** : La colonne `_is_active` (défaut 1) permet la suppression logique des enregistrements absents d'un nouveau fichier.
7. **PRAGMA WAL** : `journal_mode = WAL` pour permettre les lectures concurrentes pendant l'import.

---

## 5.2 DDL complet

### Table import_log

```sql
CREATE TABLE IF NOT EXISTS import_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp       TEXT NOT NULL,          -- ISO 8601 : "2026-05-26T03:00:00Z"
    file_name       TEXT NOT NULL,          -- "CIS_bdpm.txt"
    sha256          TEXT NOT NULL,          -- Hash SHA-256 du fichier source
    rows_read       INTEGER NOT NULL DEFAULT 0,
    rows_inserted   INTEGER NOT NULL DEFAULT 0,
    rows_updated    INTEGER NOT NULL DEFAULT 0,
    rows_deleted    INTEGER NOT NULL DEFAULT 0,  -- Soft deletes
    status          TEXT NOT NULL CHECK (status IN ('success', 'partial', 'failed', 'no_change')),
    duration_ms     INTEGER,
    error_message   TEXT
);

CREATE INDEX idx_import_log_timestamp ON import_log(timestamp);
CREATE INDEX idx_import_log_file ON import_log(file_name);
```

### Table cis_specialites (→ CIS_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_specialites (
    code_cis                    TEXT PRIMARY KEY,       -- Code CIS (8 chiffres, préfixe 6)
    denomination                TEXT NOT NULL,
    forme_pharmaceutique        TEXT,
    voies_administration        TEXT,                   -- Multi-valeurs séparées par ";"
    statut_amm                  TEXT,
    type_procedure_amm          TEXT,
    etat_commercialisation      TEXT,
    date_amm_raw                TEXT,                   -- Valeur originale DD/MM/YYYY
    date_amm                    TEXT,                   -- Normalisée YYYY-MM-DD (ou NULL)
    statut_bdm                  TEXT CHECK (statut_bdm IN ('', 'Alerte', 'Warning disponibilité')),
    numero_autorisation_euro    TEXT,
    titulaires                  TEXT,                   -- Multi-valeurs séparées par ";"
    surveillance_renforcee      TEXT CHECK (surveillance_renforcee IN ('Oui', 'Non', '')),
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_spec_denomination ON cis_specialites(denomination);
CREATE INDEX idx_cis_spec_statut ON cis_specialites(statut_amm);
CREATE INDEX idx_cis_spec_etat ON cis_specialites(etat_commercialisation);
CREATE INDEX idx_cis_spec_date_amm ON cis_specialites(date_amm);
CREATE INDEX idx_cis_spec_surveillance ON cis_specialites(surveillance_renforcee);
```

### Table cis_presentations (→ CIS_CIP_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_presentations (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis                    TEXT NOT NULL,           -- FK → cis_specialites
    code_cip7                   TEXT NOT NULL,
    libelle                     TEXT,
    statut_administratif        TEXT,
    etat_commercialisation      TEXT,
    date_declaration_raw        TEXT,                    -- DD/MM/YYYY
    date_declaration            TEXT,                    -- YYYY-MM-DD
    code_cip13                  TEXT,
    agrement_collectivites      TEXT CHECK (agrement_collectivites IN ('oui', 'non', 'inconnu', '')),
    taux_remboursement          TEXT,
    prix_ht_raw                 TEXT,                    -- Virgule décimale originale
    prix_ht                     REAL,                    -- Converti en nombre
    prix_ttc_raw                TEXT,
    prix_ttc                    REAL,
    honoraires_raw              TEXT,
    honoraires                  REAL,
    indications_remboursement   TEXT,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1,
    UNIQUE(code_cis, code_cip7)
);

CREATE INDEX idx_cis_pres_cis ON cis_presentations(code_cis);
CREATE INDEX idx_cis_pres_cip13 ON cis_presentations(code_cip13);
CREATE INDEX idx_cis_pres_prix ON cis_presentations(prix_ht);
CREATE INDEX idx_cis_pres_etat ON cis_presentations(etat_commercialisation);
CREATE INDEX idx_cis_pres_agrement ON cis_presentations(agrement_collectivites) WHERE agrement_collectivites != '';
```

### Table cis_compositions (→ CIS_COMPO_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_compositions (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis                    TEXT NOT NULL,           -- FK → cis_specialites
    designation_element         TEXT,
    code_substance              TEXT,
    denomination_substance      TEXT,
    dosage                      TEXT,
    reference_dosage            TEXT,
    nature                      TEXT CHECK (nature IN ('SA', 'FT')),
    numero_liaison_sa_ft        INTEGER,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_comp_cis ON cis_compositions(code_cis);
CREATE INDEX idx_cis_comp_substance ON cis_compositions(code_substance);
CREATE INDEX idx_cis_comp_nature ON cis_compositions(nature);
CREATE INDEX idx_cis_comp_denom ON cis_compositions(denomination_substance);
```

### Table cis_has_smr (→ CIS_HAS_SMR_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_has_smr (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis                    TEXT NOT NULL,           -- FK → cis_specialites (orphelins possibles)
    code_dossier_has            TEXT,                    -- FK → has_liens_ct
    motif_evaluation            TEXT,
    date_avis_raw               TEXT,                    -- YYYYMMDD originale
    date_avis                   TEXT,                    -- YYYY-MM-DD normalisée
    valeur_smr                  TEXT,
    libelle_smr                 TEXT,
    is_orphan                   INTEGER NOT NULL DEFAULT 0,  -- 1 si code_cis absent de cis_specialites
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_smr_cis ON cis_has_smr(code_cis);
CREATE INDEX idx_cis_smr_dossier ON cis_has_smr(code_dossier_has);
CREATE INDEX idx_cis_smr_date ON cis_has_smr(date_avis);
CREATE INDEX idx_cis_smr_orphan ON cis_has_smr(is_orphan);
```

### Table cis_has_asmr (→ CIS_HAS_ASMR_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_has_asmr (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis                    TEXT NOT NULL,
    code_dossier_has            TEXT,
    motif_evaluation            TEXT,
    date_avis_raw               TEXT,                    -- YYYYMMDD originale
    date_avis                   TEXT,                    -- YYYY-MM-DD normalisée
    valeur_asmr                 TEXT,
    libelle_asmr                TEXT,
    is_orphan                   INTEGER NOT NULL DEFAULT 0,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_asmr_cis ON cis_has_asmr(code_cis);
CREATE INDEX idx_cis_asmr_dossier ON cis_has_asmr(code_dossier_has);
CREATE INDEX idx_cis_asmr_date ON cis_has_asmr(date_avis);
CREATE INDEX idx_cis_asmr_orphan ON cis_has_asmr(is_orphan);
```

### Table has_liens_ct (→ HAS_LiensPageCT_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS has_liens_ct (
    code_dossier_has            TEXT PRIMARY KEY,
    lien_url                    TEXT,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);
```

### Table cis_generiques (→ CIS_GENER_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_generiques (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    identifiant_groupe          TEXT NOT NULL,
    libelle_groupe              TEXT,
    code_cis                    TEXT NOT NULL,           -- FK → cis_specialites (orphelins possibles)
    type_generique              INTEGER CHECK (type_generique IN (0, 1, 2, 4)),
    numero_tri                  INTEGER,
    is_orphan                   INTEGER NOT NULL DEFAULT 0,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_gen_cis ON cis_generiques(code_cis);
CREATE INDEX idx_cis_gen_groupe ON cis_generiques(identifiant_groupe);
CREATE INDEX idx_cis_gen_type ON cis_generiques(type_generique);
CREATE INDEX idx_cis_gen_orphan ON cis_generiques(is_orphan);
```

### Table cis_conditions_prescription (→ CIS_CPD_bdpm.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_conditions_prescription (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis                    TEXT NOT NULL,           -- FK → cis_specialites
    condition                   TEXT NOT NULL,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_cpd_cis ON cis_conditions_prescription(code_cis);
```

### Table cis_disponibilite (→ CIS_CIP_Dispo_Spec.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_disponibilite (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis                    TEXT NOT NULL,           -- FK → cis_specialites
    code_cip13                  TEXT,                   -- Peut être vide (95,4% de vacuité)
    code_statut                 INTEGER CHECK (code_statut IN (1, 2, 3, 4)),
    libelle_statut              TEXT,
    date_debut_raw              TEXT,
    date_debut                  TEXT,                   -- YYYY-MM-DD
    date_maj_raw                TEXT,
    date_maj                    TEXT,                   -- YYYY-MM-DD
    date_remise_raw             TEXT,
    date_remise                 TEXT,                   -- YYYY-MM-DD
    lien_ansm                   TEXT,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_disp_cis ON cis_disponibilite(code_cis);
CREATE INDEX idx_cis_disp_cip13 ON cis_disponibilite(code_cip13) WHERE code_cip13 IS NOT NULL;
CREATE INDEX idx_cis_disp_statut ON cis_disponibilite(code_statut);
CREATE INDEX idx_cis_disp_date ON cis_disponibilite(date_debut);
```

### Table cis_mitm (→ CIS_MITM.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_mitm (
    code_cis                    TEXT PRIMARY KEY,       -- FK → cis_specialites
    code_atc                    TEXT,
    denomination                TEXT,
    lien_bdpm                   TEXT,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_mitm_atc ON cis_mitm(code_atc);
```

### Table cis_info_importantes (→ CIS_InfoImportantes.txt)

```sql
CREATE TABLE IF NOT EXISTS cis_info_importantes (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    code_cis                    TEXT NOT NULL,           -- FK → cis_specialites (orphelins possibles)
    date_debut_raw              TEXT,                    -- DD/MM/YYYY
    date_debut                  TEXT,                    -- YYYY-MM-DD
    date_fin_raw                TEXT,
    date_fin                    TEXT,                    -- YYYY-MM-DD
    texte_brut                  TEXT,                    -- HTML original
    texte                       TEXT,                    -- Texte extrait (entités décodées)
    url                         TEXT,                    -- URL extraite du href
    is_orphan                   INTEGER NOT NULL DEFAULT 0,
    _import_id                  INTEGER REFERENCES import_log(id),
    _is_active                  INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cis_info_cis ON cis_info_importantes(code_cis);
CREATE INDEX idx_cis_info_date_debut ON cis_info_importantes(date_debut);
CREATE INDEX idx_cis_info_date_fin ON cis_info_importantes(date_fin);
CREATE INDEX idx_cis_info_orphan ON cis_info_importantes(is_orphan);
```

---

## 5.3 PRAGMA et configuration SQLite

```sql
-- Mode WAL pour lectures concurrentes
PRAGMA journal_mode = WAL;

-- Désactiver les FK strictes (orphelins HAS/GENER)
PRAGMA foreign_keys = OFF;

-- Performance d'écriture
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;  -- 64 Mo
PRAGMA temp_store = MEMORY;

-- Case-insensitive pour les recherches textuelles
-- (géré au niveau application via LOWER() ou COLLATE NOCASE)
```

---

## 5.4 Stratégie d'import incrémental

Pour chaque table, le processus d'import suit trois phases :

### Phase 1 : Insertion (nouveaux enregistrements)

```sql
INSERT INTO cis_specialites (code_cis, denomination, ...)
VALUES (?, ?, ...)
ON CONFLICT(code_cis) DO NOTHING;
```

### Phase 2 : Mise à jour (enregistrements modifiés)

Pour détecter les modifications, on compare le hash du contenu de chaque enregistrement :

```sql
-- Calculer le hash de la ligne actuelle
-- Si différent du hash stocké, mettre à jour
UPDATE cis_specialites
SET denomination = ?, forme_pharmaceutique = ?, ..., _import_id = ?
WHERE code_cis = ? AND content_hash != ?;
```

**Optimisation** : plutôt que de stocker un hash par enregistrement, on peut comparer champ par champ. Le hash SHA-256 de la ligne complète est plus simple et plus rapide.

### Phase 3 : Suppression logique (enregistrements absents)

```sql
-- Marquer comme inactifs les enregistrements non présents dans le nouveau fichier
UPDATE cis_specialites
SET _is_active = 0, _import_id = ?
WHERE code_cis NOT IN (SELECT code_cis FROM temp_new_data)
  AND _is_active = 1;
```

### Transaction par fichier

```sql
BEGIN TRANSACTION;
-- Phase 1 : Insertions
-- Phase 2 : Mises à jour
-- Phase 3 : Suppressions logiques
-- Enregistrement dans import_log
COMMIT;
```

---

## 5.5 Requêtes courantes

### Medicament par Code CIS avec toutes ses données associées

```sql
SELECT s.*, 
       GROUP_CONCAT(DISTINCT c.denomination_substance) as substances,
       GROUP_CONCAT(DISTINCT p.code_cip13) as presentations
FROM cis_specialites s
LEFT JOIN cis_compositions c ON s.code_cis = c.code_cis AND c._is_active = 1
LEFT JOIN cis_presentations p ON s.code_cis = p.code_cis AND p._is_active = 1
WHERE s.code_cis = '61266250' AND s._is_active = 1
GROUP BY s.code_cis;
```

### Recherche full-text par nom

```sql
SELECT code_cis, denomination, forme_pharmaceutique, etat_commercialisation
FROM cis_specialites
WHERE _is_active = 1
  AND denomination LIKE '%DOLIPRANE%'
ORDER BY denomination;
```

### Médicaments en rupture de stock

```sql
SELECT d.code_cis, s.denomination, d.libelle_statut, d.date_debut, d.lien_ansm
FROM cis_disponibilite d
JOIN cis_specialites s ON d.code_cis = s.code_cis
WHERE d._is_active = 1 AND d.code_statut IN (1, 2)
ORDER BY d.date_debut DESC;
```

### Dernières informations de sécurité

```sql
SELECT i.code_cis, s.denomination, i.date_debut, i.date_fin, i.texte, i.url
FROM cis_info_importantes i
JOIN cis_specialites s ON i.code_cis = s.code_cis
WHERE i._is_active = 1 AND i.is_orphan = 0
ORDER BY i.date_debut DESC
LIMIT 20;
```

### Évaluations HAS pour un médicament

```sql
SELECT smr.date_avis as date_smr, smr.valeur_smr, smr.libelle_smr,
       asmr.date_avis as date_asmr, asmr.valeur_asmr, asmr.libelle_asmr,
       ct.lien_url
FROM cis_specialites s
LEFT JOIN cis_has_smr smr ON s.code_cis = smr.code_cis AND smr._is_active = 1
LEFT JOIN cis_has_asmr asmr ON s.code_cis = asmr.code_cis AND asmr._is_active = 1
LEFT JOIN has_liens_ct ct ON smr.code_dossier_has = ct.code_dossier_has
WHERE s.code_cis = '61266250' AND s._is_active = 1;
```
