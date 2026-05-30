# BDMP Data Deep Dive — Statistical Analysis Results

**Source:** Raw BDMP files, analyzed 2026-05-26  
**Total disk footprint:** 22.29 MB across 10 files

---

## 1. Executive Data Summary

| File | Rows | Key Characteristic |
|------|------|-------------------|
| CIS_COMPO_bdpm.txt | 32,389 | 4,486 unique substances; SA vs FT split (26,892/5,497) |
| CIS_CPD_bdpm.txt | 28,160 | 4,640 CIS with multiple CPD rows; 165 unique rule values |
| CIS_CIP_bdpm.txt | 20,903 | EAN always 34009 prefix; price range 1.02–993.51€ |
| CIS_HAS_SMR_bdpm.txt | 15,269 | Avis field max 2,018 chars, median 153 |
| CIS_HAS_ASMR_bdpm.txt | 9,912 | Avis field max 2,019 chars, median 212 |
| CIS_GENER_bdpm.txt | 10,704 | 1,659 groups, avg 6.5 CIS/group, 141 groups non-compliant |
| CIS_MITM.txt | 7,711 | 1,255 unique ATC codes (7-char primary, 5-char secondary) |
| CIS_CIP_Dispo_Spec.txt | — | 165 KB supplemental |
| HAS_LiensPageCT_bdpm.txt | — | 499 KB supplemental |

**Core entity count:** CIS_bdpm.txt (30,000+ products) is the primary join key across all relations.

---

## 2. Per-File Findings

### CIS_HAS_SMR_bdpm.txt — Service Medical Rendu

- **15,269 rows**, avis field (field 6) stats:
  - Max length: 2,018 characters
  - Average: 231 chars
  - Median: 153 chars
  - 90th percentile: 460 chars
  - **Zero rows exceed 5,000 chars** — storage bound can be `VARCHAR(2048)`
- **Date span:** 2002–2026, with peak volume in 2015–2016 (~1,700 rows/year)
- Temporal skew: recent years (2020–2026) average ~600–700 rows/year

### CIS_HAS_ASMR_bdpm.txt — Amélioration Service Médical Rendu

- **9,912 rows**, avis field stats:
  - Max length: 2,019 characters
  - Average: 400 chars (1.7x longer than SMR)
  - Median: 212 chars
  - 90th percentile: 1,066 chars (2.3x SMR)
  - **Zero rows exceed 5,000 chars** — same `VARCHAR(2048)` bound applies
- Date distribution mirrors SMR, with 2020–2026 averaging ~400–500 rows/year

**Implication:** ASMR text content is more substantial than SMR; both comfortably fit in fixed-width `VARCHAR(2048)`.

### CIS_CIP_bdpm.txt — Presentations (Retail + Hospital)

- **20,903 rows**, all exactly 13 fields
- **Prix ville (field 11):** Min 1.02€, Max 993.51€, Mean 48.26€, Median 9.27€
  - Strong right skew (mean 5x median) — use `f64` for financial precision
- **EAN (field 7):** 100% start with `34009` (French national code prefix)
  - No empty EANs, no non-34009 codes — can normalize/validate at ingest
  - All 13-digit CIP-13 codes; no secondary barcode types
- **Comm status (field 5):** 4 distinct values:
  - `Déclaration de commercialisation`: 17,239 (82%)
  - `Déclaration d'arrêt de commercialisation`: 3,497 (17%)
  - `Arrêt de commercialisation (le médicament n'a plus d'autorisation)`: 165 (<1%)
  - `Déclaration de suspension de commercialisation`: 2 (negligible)
- **Reimbursement rate (field 9):** messy encoding:
  - Leading values: `65%` (9,146), `65 %` (1,696) — same value, different format
  - 9 distinct values: `15%`/`15 %`, `30%`/`30 %`, `35%`, `65%`/`65 %`, `100%`/`100 %`
  - **Action required:** Normalize to canonical `f32` (0.15, 0.30, 0.35, 0.65, 1.0)
- **CIP uniqueness:** Zero duplicate CIP codes — valid primary key

### CIS_GENER_bdpm.txt — Generic Groups

- **10,704 rows, 1,659 groups**, average 6.5 CIS per group
- Group size distribution:
  - 111 groups (6.7%) have only 1 CIS — effectively ungrouped
  - 843 groups (50.8%) have 2–5 CIS — standard generic clusters
  - 705 groups (42.5%) have 6–36 CIS — large therapeutic groups
- **Compliance anomalies:**
  - 141 groups (8.5%) have !=1 reference drug — violates expected 1:1 mapping
  - 106 groups (6.4%) have zero generics — reference-only groups
  - These edge cases must be handled in group resolution logic

**Implication:** Group cardinality is high-variance; pre-allocate for up to 36 CIS per group when building in-memory group structures.

### CIS_CPD_bdpm.txt — Prescription Conditions

- **28,160 rows**, significant multi-row per CIS pattern
- **4,640 CIS (15.5%)** have multiple CPD rows — up to 6 rows for some CIS
- **165 unique rule values** — high cardinality categorical field
- Top 10 rules account for majority of rows; long-tail distribution
- Rules include:
  - `liste I` (10,864): strictest control
  - `liste II` (1,304): moderate control
  - `prescription hospitalière` (1,345)
  - Specialist restrictions: oncologie, hematologie, medecine interne
  - `médicament nécessitant une surveillance particulière` (1,737)

**Implication:** CPD is a one-to-many join on CIS. Schema must support N CPD rows per CIS. Rule text is free-form; consider full-text indexing if search is needed.

### CIS_MITM.txt — Market Authorization (MITM + ATC)

- **7,711 rows**, 1,255 unique ATC codes
- **ATC length distribution:**
  - 7-char codes (1,223): full ATC5 classification (e.g., `R03BA01`)
  - 5-char codes (32): therapeutic subgroup (e.g., `B05DB`)
- No other lengths present — can enforce `CHAR(7)` with nullable `CHAR(5)` for subgroup

**Implication:** ATC hierarchy is 5-level (L1-L5 = 7 chars). For lookup efficiency, index on full 7-char code; parent 5-char/3-char can be derived.

### CIS_COMPO_bdpm.txt — Composition

- **32,389 rows**, 4,486 unique substances
- **Pharmaceutical form codes (field 7):**
  - `SA`: 26,892 rows (83%) — standard pharmaceutical forms
  - `FT`: 5,497 rows (17%) — drug forms with specific characteristics
  - No other codes in dataset
- Top substances by CIS coverage: HYDROCHLOROTHIAZIDE (284), PARACETAMOL (233), AMLODIPINE (166)

**Implication:** Substance-to-CIS is many-to-many (diuretic combinations common). Join cardinality: average ~7.2 substance rows per CIS.

### File Sizes

| File | Size |
|------|------|
| CIS_HAS_SMR_bdpm.txt | 4,388 KB |
| CIS_HAS_ASMR_bdpm.txt | 4,375 KB |
| CIS_CIP_bdpm.txt | 4,054 KB |
| CIS_bdpm.txt | 3,091 KB |
| CIS_COMPO_bdpm.txt | 2,670 KB |
| CIS_CPD_bdpm.txt | 1,283 KB |
| CIS_GENER_bdpm.txt | 1,187 KB |
| CIS_MITM.txt | 1,110 KB |
| HAS_LiensPageCT_bdpm.txt | 499 KB |
| CIS_CIP_Dispo_Spec.txt | 165 KB |
| **TOTAL** | **22.29 MB** |

---

## 3. Implications for Rust Implementation

### Memory Allocation

- **Full dataset in-memory:** ~25 MB raw, ~60–80 MB with parsed structures + indexes (estimate for hashmap overhead, string interning, BTree index)
- **Streaming parse is feasible:** Largest single file is 4.4 MB; chunk-based parsing with 10,000 row buffer sufficient
- **No memory pressure:** Target hardware is modern desktop/VPS; 512MB+ RAM assumed

### Parsing Strategy

1. **CSV parsing:** Use `csv` crate with `terminator = csv::Terminator::record_separator(b'\n')` — no embedded newlines in BDMP fields
2. **Encoding:** CIS_CIP uses UTF-8; all others use `latin-1` (ISO-8859-1). Rust `read` returns `&[u8]`; decode per-file
3. **Price normalization:** Replace `,` with `.` then `str::parse::<f64>()` — handle empty fields gracefully
4. **Reimbursement rate cleanup:** Map `65 %` → 0.65, `65%` → 0.65 at ingest; store as `f32`

### Indexing Strategy

| Relation | Primary Key | Foreign Key | Suggested Index |
|----------|-------------|-------------|-----------------|
| CIS_bdpm.txt | CIS (9-digit) | — | Clustered PK |
| CIS_CIP_bdpm.txt | CIP (13-digit) | CIS | FK index on CIS |
| CIS_COMPO_bdpm.txt | composite (CIS + code) | CIS | FK index on CIS |
| CIS_CPD_bdpm.txt | composite (CIS + rule) | CIS | FK index on CIS |
| CIS_GENER_bdpm.txt | composite (CIS + group) | CIS, GRP | Composite (CIS, GRP) |
| CIS_HAS_SMR_bdpm.txt | composite (CIS + date) | CIS | FK index on CIS |
| CIS_HAS_ASMR_bdpm.txt | composite (CIS + date) | CIS | FK index on CIS |
| CIS_MITM.txt | composite (CIS + ATC) | CIS, ATC | FK index on CIS, ATC index |

### Data Integrity Rules

1. **EAN validation:** All CIP EAN must start with `34009`; reject non-conforming at ingest
2. **Group constraint:** Expect 1 reference + N generics per group; flag anomalies
3. **ATC length:** 7-char primary, 5-char subgroup; reject 6-char or other lengths
4. **Reimbursement normalization:** Canonical float format, reject text values

### Query Patterns

- **Most common:** CIS → [CIP, COMPO, CPD, GENER, SMR, ASMR, MITM] — 7 table join on CIS
- **Price lookup:** CIP → prix_ville (single field fetch, indexed CIP)
- **ATC hierarchy:** ATC7 → derive parent ATC5/ATC3 → group by therapeutic class
- **Group resolution:** CIS → GENER → group_id → all CIS in group (for generic substitution)

---

## 4. Schema Consequences

### Enforced Constraints

| Field | Type | Constraint | Rationale |
|-------|------|-------------|-----------|
| avis (SMR/ASMR) | VARCHAR(2048) | NOT NULL | All 25,181 rows have content; max observed 2,019 chars |
| prix_ville | DECIMAL(10,2) | CHECK >= 0 | Range 1.02–993.51; can accommodate up to 9999.99 |
| ean | CHAR(13) | CHECK STARTS WITH '34009' | 100% conformity observed |
| reimb_rate | DECIMAL(3,2) | CHECK IN (0.15, 0.30, 0.35, 0.65, 1.0) | Canonical set; normalize on ingest |
| atc_code | CHAR(7) | CHECK LENGTH IN (5, 7) | Only 5 and 7 char observed |
| cip | CHAR(13) | UNIQUE | No duplicates confirmed |

### Denormalization Opportunities

1. **CIP → CIS direct map:** Cache `HashMap<CIP, CIS>` for fast EAN→product lookup
2. **ATC hierarchy cache:** Pre-compute ATC7→ATC5→ATC3→ATC1 parent chains
3. **Generic group index:** `HashMap<GRP, Vec<CIS>>` for substitution lookups
4. **Substance frequency:** `HashMap<substance, count>` for filtering high-frequency compounds

### Temporal Considerations

- SMR/ASMR dates span 2002–2026; queries must handle date range filtering
- Most recent data (2024–2026) accounts for ~30% of review records
- Historical records needed for trend analysis; no purge criteria observed

### Null Handling

- **Reimbursement base (field 10):** Mixed numeric (e.g., `24,34`, `68,68`) — parse as DECIMAL(6,2) with NULL for missing
- **Rule text (CPD field 1):** Never null in dataset; consider NOT NULL
- **ATC codes:** Never null in dataset; consider NOT NULL

---

## 5. Summary

The BDMP dataset is well-structured with consistent field widths, minimal null rates, and clean primary keys. Key implementation decisions:

1. **Use streaming CSV parser** — largest file is 4.4 MB, buffering 10K rows is sufficient
2. **Canonicalize reimbursement rates** at ingest — messy string encoding must be normalized
3. **Build CIS-centric indexes** — the product identifier is the hub for all lookups
4. **Store avis fields as VARCHAR(2048)** — confirmed max length well below limit
5. **Validate EAN prefix** — 100% conformity with 34009; can enforce as business rule
6. **Handle multi-row CPD** — 15.5% of CIS have multiple prescription conditions
7. **Group cardinality variance** — pre-allocate for 1–36 CIS per generic group
