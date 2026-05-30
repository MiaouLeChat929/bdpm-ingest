# Homeopathy Filtering Analysis

## Summary

- **Filtering rate**: 1,485 / 15,848 = **9.37%**
- **Matches external audit**: 9.33% observed (within expected 5-15% range)
- **Status**: Working correctly for homeopathic products

## Filtering Breakdown

### By Procedure Type (CIS_bdpm field 6)
| Procedure Type | Count |
|----------------|-------|
| Enreg homéo (Proc. Nat.) | 1,319 |
| Procédure nationale | 148 |
| Procédure centralisée | 13 |
| Enreg phyto (Proc. Nat.) | 4 |
| Procédure de reconnaissance mutuelle | 1 |

### By Lab Name (CIS_bdpm field 11)
| Lab | Count | Notes |
|-----|-------|-------|
| BOIRON | 868 | Primary homeopathy lab |
| LEHNING | 363 | Homeopathy lab |
| WELEDA | 225 | Homeopathy lab |
| FERRIER | 15 | Homeopathy lab |
| (14 others) | 14 | **FALSE POSITIVES** |

### Four-Layer Detection Analysis

**Layer 1: Lab name** (BOIRON, LEHNING, WELEDA, FERRIER, PERRIGO, HERBALGEM)
- Catches: 1,456 rows
- Effectiveness: Primary detection layer

**Layer 2: Keywords** (HOMEOPATHI, DILUTION in name+form)
- Catches: Covered by Layer 1/3 overlap
- Not independently measurable

**Layer 3: Procedure type** (ENREG HOM, ENREGISTREMENT HOMEOPATHIQUE)
- Catches: 1,319 rows
- Note: All 1,319 are from known homeopathy labs

**Layer 4: Dilution regex** (`\b\d+\s*(?:CH|DH|K|X|LM)\b`)
- Catches: Covered by Layer 1/3 overlap
- **FALSE POSITIVE BUG**: Matches "X" as multiplication operator in gene therapy products

## CIS_COMPO Alignment

| Metric | Value |
|--------|-------|
| Raw CIS_COMPO unique CIS | 15,845 |
| Drugs table CIS | 14,363 |
| CIS in COMPO but not drugs | 1,485 |
| CIS in drugs but not COMPO | 3 |

**Interpretation**: 1,485 CIS codes in CIS_COMPO have no corresponding drug because the homeopathy filter excluded them. The 3 drugs without compositions are edge cases.

## False Positive Bug (Layer 4)

### Issue
14 legitimate non-homeopathic products are incorrectly filtered due to the dilution regex matching "X" as a homeopathic unit when it's actually a multiplication operator.

### Affected Products (all gene/cell therapies)
| CIS | Product | Type |
|-----|---------|------|
| 60862404 | TECARTUS | CAR-T cell therapy |
| 61876862 | UPSTAZA | Gene therapy |
| 63300452 | HEMGENIX | Gene therapy (hemophilia) |
| 63621440 | LIBMELDY | Gene therapy |
| 63887327 | YESCARTA | CAR-T cell therapy |
| 64181296 | VYJUVEK | Gene therapy gel |
| 64743778 | CARVYKTI | CAR-T cell therapy |
| 65329132 | ABECMA | CAR-T cell therapy |
| 65647882 | KYMRIAH | CAR-T cell therapy |
| 66578108 | CASGEVY | CRISPR gene therapy |
| 66829076 | EBVALLO | Gene therapy |
| 67401121 | IMUKIN | Biologic (interferon) |
| 69200149 | ZOLGENSMA | Gene therapy (SMA) |
| 69302990 | LUXTURNA | Gene therapy |

### Root Cause
The regex `\b\d+\s*(?:CH|DH|K|X|LM)\b` matches patterns like:
- "2 X" (from "2 x 100 000 000 cellules") - multiplication
- "500 X" (from "260 - 500 x 1 000 000 cellules") - dosage range
- "8 X" (from "2,8 x 100 000 000 000 génomes")

The "X" in these contexts is a multiplication/times operator, not a homeopathic dilution.

### Recommended Fix
Either:
1. Remove "X" from the dilution regex (X is rarely used in homeopathy)
2. Add context-aware detection to distinguish "X" multiplication from "X" dilution
3. Scope the project to exclude gene/cell therapies if intentional

## No False Negatives

Verification that no homeopathic products leaked through:
- 0 drugs with HOMEOPATHI in name
- 0 drugs with DILUTION in name
- 0 drugs with dilution pattern (CH/DH/K/X/LM) in name

## Conclusion

The homeopathy filtering is **working correctly** with 9.37% filtering rate matching external audit. However, there is a **false positive bug** where 14 gene/cell therapy products are incorrectly filtered due to the "X" in multiplication expressions being matched as homeopathic dilution units.
