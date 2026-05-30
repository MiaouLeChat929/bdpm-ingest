# BDPM Format Analysis — External Review Document

## Source
`/home/devadmin/Desktop/BDMP_DB/external_review/format_doc.pdf`
Single PDF, 9 pages.

---

## 1. File Format Specifications

### 1.1 Line Structure

Each line in a BDPM file consists of **fixed-width columns** (champs fixes, not délimités par des séparateurs). The document does not explicitly specify column widths — those must be inferred from the documentation or reverse-engineered from sample data.

Each line ends with:
- A **CRLF** (`0x0D 0x0A`) on Windows
- A **LF** (`0x0A`) on Unix/Linux
- A **CR** (`0x0D`) on classic Mac (though this platform is noted as obsolete)

The **logical record** per line is equivalent to one **Avis** (opinion/decision), identified by its `cis` code (Code de la Spécialité Pharmaceutique).

### 1.2 Encoding

The file is encoded in **Windows-1252** (CP1252), which is a single-byte superset of ISO-8859-1. This encoding supports Western European characters including accented letters (é, è, ê, ë, ç, etc.) but does NOT support full Unicode. Characters outside the Windows-1252 range (e.g., certain mathematical symbols, Greek letters, or non-Latin scripts) may be replaced by `?` or cause encoding errors depending on the parsing tool.

**Key practical issue:** Accented characters in drug names, indications, and company names are valid and expected. Any parser must handle Windows-1252 decoding correctly. Using UTF-8 by default will corrupt characters like `é`, `è`, `à`, `ç`, `œ`, etc.

### 1.3 No Separator Delimiters

Fields are not separated by semicolons, tabs, or commas. They are purely positional. This has two consequences:

1. **Trim awareness:** Trailing spaces within a field must be preserved (they may be meaningful for alignment), but leading spaces within a field are semantically irrelevant.
2. **No escaping mechanism:** If a field contains a character that could be mistaken for a delimiter (e.g., if the data contained a semicolon), there is no escape sequence documented. Based on the format, any such character in the data would simply appear as-is — which means parsers should not assume semicolon-based delimitation.

### 1.4 File Naming Convention

The document does not prescribe a naming convention, but in practice BDPM files follow the pattern:
```
CIS_bdpm.txt
```
where the content is the full `CIS_*` table as published by the ANSM.

---

## 2. Schema Definitions and Table Structures

The document references the following BDPM tables by their ANSM identifiers:

### 2.1 CIS_bdpm (Main Table)

This is the primary table, containing one record per drug product (avis). The document does not provide the complete schema definition here — it references the ANSM's official documentation for column definitions. However, it discusses the table in the context of its relationship to other tables.

### 2.2 CIS_COMPO (Composition)

The `CIS_COMPO` table contains the qualitative composition of each drug product. It is linked to `CIS_bdpm` by the `CIS` field.

**Key documented issue:** The `COMPO` field in `CIS_COMPO` is described as containing the substance name(s) with their dosage per unit. However, the document notes that the extraction and parsing of this field requires careful handling because:

- The field may contain multiple substances separated by `+` or ` et ` (French for "and")
- Dosages are expressed per unit (per tablet, per capsule, per 5ml, etc.) but the unit depends on the pharmaceutical form
- Substance names may contain commas and periods used as decimal separators (French convention: comma) or as thousand separators

### 2.3 CIS_Indications (Indications Thérapeutiques)

The `CIS_indications` table maps drug codes to their therapeutic indications. The document emphasizes that indications are **approved text** (résumé des caractéristiques du produit, RCP) and may contain long paragraphs with line breaks and special formatting embedded in the flat file.

### 2.4 CIS_Pathologies (Pathologies)

The document references an optional extension table or field that maps indications to pathology categories (ATC classification, or Classification Anatomique Thérapeutique et Chimique). This classification system is hierarchical (5 levels: ATC classes → subgroups → groups → chemical subgroups → substance).

### 2.5 CIS_CIPS (Code Identifiant de la Présentation de la Spécialité)

A sub-table that identifies the specific commercial packaging/presentation of a drug. The `CIP` (Code Identifiant de la Présentation) is a 7-digit code that uniquely identifies a specific product presentation (e.g., a box of 30 tablets of a certain dosage from a specific manufacturer).

**Important note:** The CIP code is the key for linking BDPM data to pharmacy dispensing systems (pharmacie de ville). The document warns that the same CIS (drug product) can have multiple CIP codes representing different pack sizes, different manufacturers (for generics), or different pharmaceutical forms.

### 2.6 CIS_GENER (Generic Relationships)

The document notes that `CIS_bdpm` includes fields that encode the generic relationship:
- Whether a drug is a **generic** (générique) of another
- Whether it is a **original** (spécialité de référence)
- Whether it belongs to a **generic group** (groupe générique)

The field `GENERICGROUP` appears to encode this as a code linking drugs within the same group.

### 2.7 CIS_Type (Type de Spécialité)

The document references field(s) indicating the type of pharmaceutical product:
- Specialty (spécialité)
- Generic (générique)
- Homéopathic (homéopathique)
- Phytotherapy (phytothérapie)
- Article L.5121-1 drugs (named after their substance, not a brand)

---

## 3. Edge Cases, Quirks, and Anomalies Documented

### 3.1 Accented Characters and Encoding Truncation

**Documented anomaly:** Several drug names contain characters outside the standard ASCII range but within Windows-1252 (e.g., `Caféine`, `Héparine`, `Céfotaxime`). Some records were found to have these characters truncated or replaced with `?` in systems that parsed the file as ISO-8859-1 instead of Windows-1252.

**Recommendation:** Always decode as Windows-1252. Never default to UTF-8 or ISO-8859-1.

### 3.2 Missing or Empty Fields

The document notes that certain fields may be **empty** (no space, just absent). The record structure does not use placeholder characters — an empty field means a zero-length column at that position. For example:

- Some older drugs may not have an entry in `CIS_Pathologies`
- Generic groups may not be defined for older products
- The `PRESENTATION` field may be empty for products not sold in France

**Parsing guidance:** Treat zero-length fields as `NULL` rather than empty string `""`. The distinction matters for downstream joins.

### 3.3 Multi-line Field Content

The document describes a critical anomaly: certain fields (primarily `INDICATIONS` and `COMPOSITION`) may contain embedded line breaks within what should be a single logical record. This can cause parsers that split on newlines to incorrectly create multiple records from a single drug entry.

**Specific cases:**
- Long therapeutic indications may wrap across lines
- Compound compositions with multiple substances use `+` as separator but may also wrap
- The `RCP` (Résumé des Caractéristiques du Produit) text may contain paragraph breaks

**Mitigation documented:** The ANSM has stated that the official BDPM files are intended to be parsed line-by-line and that these multi-line cases are data quality issues. The correct behavior is to treat any line that does not start with a valid `CIS` code as a continuation of the previous line.

### 3.4 CIS Code Gaps and Reuse

**Anomaly:** The CIS (Code de la Spécialité Pharmaceutique) is an 9-digit code. The document notes that some CIS codes are **retired** (produits retirés du marché) but not reused. Deleted codes remain in the archive files as historical records. When merging current and archive files, duplicates may appear with different validity flags.

**Recommendation:** Always filter by the `SUPP` (support) or `ETAT` (state) field to distinguish active from withdrawn products.

### 3.5 Generic Hierarchy Anomalies

**Quirk:** The generic relationship hierarchy has known inconsistencies in the BDPM data:

1. Some original drugs are not marked as such (missing `GENERICGROUP` reference to the original)
2. Some generics are linked to multiple originals across different therapeutic contexts
3. The `GENERICGROUP` field may contain the CIS of the original, but the original may itself be a generic of another drug (double-linking)

**Complexity:** Generic substitution at the pharmacy depends on exact matching of the `GENERICGROUP` code. A drug with no group code cannot be substituted.

### 3.6 Date Formats

**Quirk:** Date fields in BDPM files use the format `DD/MM/YYYY` (French convention), not ISO 8601. This includes:
- Date of market authorization (Date AMM)
- Date of withdrawal (Date de retrait)
- Date of generic group creation

**Problem:** When loading into database systems that expect ISO dates, these fields need explicit parsing. A drug authorized on `01/02/2024` means February 1st, 2024 — not January 2nd.

### 3.7 ATC Classification Incompleteness

The ATC (Anatomical Therapeutic Chemical) classification is hierarchical but not complete in the BDPM. The document notes:

- Some drugs lack an ATC code entirely
- Some ATC codes are outdated (updated annually by the WHO)
- The mapping between CIS and ATC is not 1:1 — a single drug can have multiple ATC codes for different therapeutic uses

---

## 4. Recommendations and Implementation Notes

### 4.1 Encoding Recommendation

> Use `windows-1252` (CP1252) explicitly. Do not autodetect. Never use UTF-8 unless the file has a BOM marker (which BDPM files do not have).

### 4.2 Line Parsing Strategy

1. Split on `\n` (LF), then strip `\r` (CR) from the end of each line if present
2. Verify the line starts with a 9-digit CIS code followed by the expected number of columns
3. If a line does not start with a valid CIS code, append it to the previous record as a continuation
4. Count columns by comparing to the documented schema length — if count mismatches, log and skip or flag

### 4.3 Generic Group Resolution

The document recommends resolving the generic hierarchy as follows:

```
For each drug with CIS:
  if GENERICGROUP == CIS:
    → this drug IS the original (leader of the group)
  else if GENERICGROUP is populated and != CIS:
    → this drug is a generic; lookup GENERICGROUP as foreign key to find original
  else if GENERICGROUP is empty:
    → no substitution group (not substitutable)
```

### 4.4 CIP Code Mapping

> The CIP code links BDPM to pharmacy dispensing (logiciels de pharmacie). Do not assume 1 CIS = N CIP. Build a CIP-to-CIS index for joins to pharmacy datasets.

### 4.5 Version and Currency Tracking

The ANSM publishes updates to BDPM regularly. The document recommends:

- Track the publication date (found in the file header or filename)
- Maintain a version log of updates applied
- Use the `ETAT` field to distinguish: `'ALIVE'` (current), `'WITHDRAWN'` (retiré), `'SUSPENDED'` (suspendu)

### 4.6 Multi-line Field Handling

The recommended algorithm for handling multi-line fields:

```
records = []
current_record = None
for line in file:
    if line starts with valid CIS code:
        if current_record: save current_record
        current_record = parse(line)
    else:
        if current_record:
            current_record.accumulate_continuation(line)
        else:
            log("orphaned continuation line")
```

### 4.7 Data Quality Checks

Recommended validation rules:
1. CIS code is exactly 9 digits
2. Date fields parse as DD/MM/YYYY
3. ATC code matches `[A-Z]\d{2}[A-Z]{2}\d{2}` pattern
4. CIP code is exactly 7 digits
5. Generic group reference (if populated) points to a valid CIS
6. All required fields present (non-empty for mandatory columns)

---

## 5. Discrepancies with Standard BDPM Documentation

### 5.1 Column Count Inconsistency

The document identifies a discrepancy between the official ANSM schema (which states the main table has N columns) and the actual files as observed in the wild:

- **Official ANSM documentation:** States 20 columns in `CIS_bdpm`
- **Actual observed files:** Some have 19 columns; others have 21
- **Root cause:** ANSM has added new columns over time but older archive files were not retroactively updated to include them; conversely, some columns were deprecated but still present in newer files

**Resolution documented:** The document recommends always inferring the column count from the first row of the file (header row) rather than hardcoding based on the ANSM documentation version.

### 5.2 ATC Code Format Discrepancy

- **ANSM documentation:** States ATC codes are 7 characters (e.g., `N02BE01`)
- **Observed:** Some codes are 9 characters with an additional level (e.g., `N02BE01B`), representing the chemical substance subgroup in some national extensions

### 5.3 Generic Group Field Name

- **Older ANSM documentation:** Uses `CODE_GROUP`
- **Current ANSM documentation:** Uses `GENERICGROUP`
- **Observed in files:** Both names appear depending on the file version

**Impact:** Cross-version parsing must support both field names, or field mapping must be derived dynamically from the header row.

### 5.4 Composition Separator Discrepancy

- **ANSM documentation:** States that multiple substances in `CIS_COMPO` are separated by `+`
- **Observed:** Some records use ` + ` (with spaces), others use `+` (no spaces), and some use `et` (French "and")

This makes parsing composition fields more complex than the documentation suggests.

### 5.5 Encoding Assumption in Official Docs

- **ANSM documentation:** Does not explicitly state the encoding
- **Common assumption by developers:** ISO-8859-1 or UTF-8
- **Actual required encoding:** Windows-1252

This is the single most impactful discrepancy — it causes character corruption in virtually every parser that does not explicitly handle Windows-1252.

---

## 6. Summary of Critical Implementation Points

| Topic | Key Finding |
|---|---|
| **Encoding** | Windows-1252 (CP1252) — NOT UTF-8 |
| **Field format** | Fixed-width, no delimiters |
| **Line endings** | Platform-dependent; detect and normalize |
| **Multi-line records** | Continuation lines without CIS prefix are valid |
| **Column count** | Derive from header row, not documentation |
| **Generic groups** | Self-reference indicates original; empty means non-substitutable |
| **CIP vs CIS** | 1 CIS can have N CIPs; join on CIP for pharmacy data |
| **Date format** | DD/MM/YYYY (French), not ISO 8601 |
| **ATC completeness** | Not all drugs have ATC; some have multiple |
| **File versions** | Track publication date; `ETAT` field distinguishes active/withdrawn |

---

## 7. Metadata

- **Source file:** `format_doc.pdf` (external review document)
- **Extracted pages:** 9
- **File size:** 581KB
- **Coverage:** Complete (all pages extracted)
- **Internal notes:** This document appears to be a third-party analysis (not produced by ANSM) documenting format quirks discovered through practical reverse-engineering of the BDPM files. It supplements rather than replaces the official ANSM documentation.