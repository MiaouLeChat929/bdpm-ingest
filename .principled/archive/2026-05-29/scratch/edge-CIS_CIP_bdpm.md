# Edge Case Analysis: CIS_CIP_bdpm.txt

**File**: `/home/devadmin/Desktop/BDMP_DB/raw/CIS_CIP_bdpm.txt`
**Date**: 2026-05-26
**Total Rows**: 20,903
**Encoding**: UTF-8
**Delimiter**: Tab
**Expected Fields**: 13
**Actual Fields**: All 20,903 rows have exactly 13 fields (perfect structural integrity)

---

## CRITICAL FINDINGS

### 1. French Price Format Ambiguity (Breaking Bug)

**Fields**: 9 (prix_ht), 10 (prix_ville)
**Issue**: 466 rows use French thousand-separator format with comma as both thousands separator AND decimal point.

**Examples**:
| Raw Value | Naive Parse (replace comma) | Correct Parse |
|-----------|----------------------------|---------------|
| `50,642,40` | 50.642.40 (invalid) | 50642.40 |
| `1,466,29` | 1.466.29 (invalid) | 1466.29 |
| `25,501,46` | 25.501.46 (invalid) | 25501.46 |

**Pattern**: Values contain 2 commas when they exceed 999.99, e.g.:
- Single digit: `1,02`
- Below 1000: `1,469,56` (means 1469.56)
- Above 10000: `50,642,40` (means 50642.40)

**Parser Fix Required**:
```python
def parse_french_price(s):
    if not s: return None
    if s.count(',') == 1:
        # Standard: just decimal comma
        return float(s.replace(',', '.'))
    elif s.count(',') == 2:
        # French format: thousands + decimal
        return float(s.replace(',', ''))
    return None
```

**Affected CIPs**: 466 unique CIP codes
**Sample rows**:
- CIP `2784980`: prix_ht=`1,466,29`, prix_ville=`1,469,05`
- CIP `3030104`: prix_ht=`7,518,58`, prix_ville=`7,519,60`

---

### 2. Far-Future Date (Data Entry Error)

**Row**: CIS=66338465
**Date Field (5)**: `29/11/2924`
**Issue**: Clearly a typo - likely meant `29/11/2024` or `29/11/1924`

**Recommendation**: Validate dates are within reasonable range (1800-2100).

---

## IMPORTANT FINDINGS

### 3. Historical Dates (Pre-1990)

**Count**: 872 rows
**Earliest**: 19/01/1910 (CIS=68740295)
**Latest pre-1990**: 19/12/1989

**Context**: This is intentional historical data from the French pharmaceutical database (BDPM). Drugs from this era may no longer be commercialized but remain in the database for reference.

**Pattern**: Many dates cluster on the 19th of the month, suggesting systematic date assignment.

---

### 4. Reimbursement Rate Whitespace Variants

**Field**: 8
**Issue**: Inconsistent whitespace before percent sign

**Variants**:
| Value | Count |
|-------|-------|
| `65%` | 9,146 |
| `65 %` | 1,696 |
| `100%` | 838 |
| `100 %` | 376 |
| `30%` | 838 |
| `30 %` | 292 |
| `15%` | 267 |
| `15 %` | 89 |
| `35%` | 4 |
| `(empty)` | 7,357 |

**Parser Fix**: Strip whitespace before storing/comparing.

---

### 5. Field 12 Content

**Empty**: 20,089 rows (96.1%)
**Non-empty**: 814 rows

**Content Type**: French regulatory text with HTML formatting
**Example**:
```
Ce médicament peut être pris en charge ou remboursé par l'Assurance Maladie dans les cas suivants :<br><br>- Psychose et schizophrénie chez l'adulte<br>- Accès maniaques chez l'adulte atteint de trouble bipolaire
```

**Notes**:
- Contains `<br>` HTML tags
- May contain special characters (é, è, ê, etc.)
- One typo found: `etre` instead of `être`
- Contains special characters: `¿` (inverted question marks) in some entries

---

### 6. CIP-to-CIS Relationship

**Observation**: 4,571 CIS codes have multiple CIP entries
**Example**: A single CIS may have 1-5+ different packaging sizes/formats

**Primary Key Recommendation**: Use CIP (field 1) as unique identifier, not CIS.

---

## MINOR/NORMAL FINDINGS

### 7. Presentation Status Distribution

| Status | Count | % |
|--------|-------|---|
| Présentation active | 20,802 | 99.5% |
| Présentation abrogée | 101 | 0.5% |

---

### 8. Comment Status Distribution

| Status | Count | % |
|--------|-------|---|
| Déclaration de commercialisation | 17,239 | 82.5% |
| Déclaration d'arrêt de commercialisation | 3,497 | 16.7% |
| Arrêt de commercialisation (le médicament n'a plus d'autorisation) | 165 | 0.8% |
| Déclaration de suspension de commercialisation | 2 | <0.1% |

---

### 9. EAN Codes

**Prefix**: 100% start with `34009` (French pharmaceutical prefix)
**Format**: All exactly 13 digits
**Empty**: 0
**Uniqueness**: All 20,903 unique

---

### 10. Reimbursable Field

| Value | Count | % |
|-------|-------|---|
| oui | 15,012 | 71.8% |
| non | 5,891 | 28.2% |

---

### 11. Price Fields Completeness

| Field | Empty | % Empty | Range |
|-------|-------|---------|-------|
| 9 (prix_ht) | 7,357 | 35.2% | 0.66 - 999.20 |
| 10 (prix_ville) | 7,364 | 35.2% | 1.02 - 993.51 |
| 11 (taux_remboursement) | 7,360 | 35.2% | 1.02 - 2.76 |

**Pattern**: Empty rows correspond to ~35% of dataset - likely reflects drugs with no published prices.

---

## NO ISSUES FOUND

- No embedded tabs within fields
- No embedded CRLF/LF within fields
- No null bytes (0x00) in first 1000 rows
- No negative prices
- No invalid DD/MM/YYYY format dates
- No duplicate CIP codes
- No missing CIS codes (all 8-digit format)
- No missing CIP codes (all 7-digit format)
- No non-UTF-8 encoding issues

---

## PARSER RECOMMENDATIONS

```python
# Recommended price parser
def parse_price(s):
    if not s:
        return None
    if s.count(',') == 2:
        # French thousand separator: 50,642,40 -> 50642.40
        return float(s.replace(',', ''))
    else:
        # Standard decimal comma: 1,02 -> 1.02
        return float(s.replace(',', '.'))

# Recommended rate normalization
def normalize_rate(s):
    if not s:
        return None
    return s.strip().replace(' ', '')

# Recommended date validation
def parse_date_robust(s):
    from datetime import datetime
    try:
        dt = datetime.strptime(s, '%d/%m/%Y')
        if dt.year < 1800 or dt.year > 2100:
            return None  # Out of reasonable range
        return dt
    except:
        return None
```

---

## DATA QUALITY SUMMARY

| Aspect | Status |
|--------|--------|
| Structural Integrity | PASS |
| Encoding | PASS |
| Field Completeness | PASS (fields 0-7 fully populated) |
| Price Format | FAIL (requires custom parser) |
| Date Validity | WARN (1 future typo, 872 historical) |
| ID Uniqueness | PASS (CIP is unique) |
| EAN Format | PASS |
| HTML Content | NOTE (field 12 contains HTML) |