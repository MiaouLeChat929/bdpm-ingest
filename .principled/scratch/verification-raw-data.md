# BDMP Raw Data Verification Report

Generated: 2026-05-26
Source: /home/devadmin/Desktop/BDMP_DB/raw/

## Summary of Claims vs Actual Findings

| Claim | Status | Details |
|-------|--------|---------|
| CIS_HAS_SMR_bdpm.txt has 22,253 bytes with value 0x92 | **VERIFIED** | Exactly 22,253 occurrences of byte 0x92 |
| CIS_HAS_ASMR_bdpm.txt has 29,704 bytes with value 0x92 | **VERIFIED** | Exactly 29,704 occurrences of byte 0x92 |
| CIS_CIP_Dispo_Spec.txt is latin-1 (not UTF-8) | **VERIFIED** | UTF-8 decode fails at byte 0xe0; latin-1 decodes OK |
| CIS_CPD_bdpm.txt has mixed line endings (\\r\\r\\n) | **VERIFIED** | Found exactly 6 instances of \\r\\r\\n |
| CIS_CIP_bdpm.txt uses LF (not CRLF) | **VERIFIED** | 0 CRLF sequences, 20,903 LF-only |
| CIS_InfoImportantes.txt is UTF-8 | **MISSING FILE** | File does not exist anywhere in BDMP_DB/ |

## Detailed Findings

### 1. CIS_HAS_SMR_bdpm.txt — 0x92 Byte Count

- **File size:** 4,493,611 bytes
- **0x92 byte count:** 22,253
- **File encoding:** Non-ISO extended-ASCII, CRLF line terminators
- **Line count:** 15,257
- **Tab count:** 76,285 (confirms TSV format)
- **Status:** Claim verified. The 22,253 figure refers to the 0x92 byte count, NOT the file size.

### 2. CIS_HAS_ASMR_bdpm.txt — 0x92 Byte Count

- **File size:** 4,480,434 bytes
- **0x92 byte count:** 29,704
- **File encoding:** Non-ISO extended-ASCII, CRLF line terminators
- **Line count:** 9,906
- **Tab count:** 49,530 (confirms TSV format)
- **Status:** Claim verified. The 29,704 figure refers to the 0x92 byte count, NOT the file size.

### 3. CIS_CIP_Dispo_Spec.txt — Encoding

- **File size:** 168,769 bytes
- **UTF-8 decode:** FAILS — `'utf-8' codec can't decode byte 0xe0 in position 207: invalid continuation byte`
- **Latin-1 decode:** OK
- **File encoding (file cmd):** ISO-8859 text, with CRLF line terminators
- **Line count:** 766
- **Tab count:** 5,362 (confirms TSV format)
- **Status:** Claim verified. File is latin-1 (ISO-8859), not UTF-8.

### 4. CIS_CPD_bdpm.txt — Mixed Line Endings

- **File size:** 1,313,810 bytes
- **CRLF count:** 28,154
- **LF-only count:** 0
- **CR-only count:** 6
- **\\r\\r\\n count:** 6
- **File encoding (file cmd):** ISO-8859 text, with CRLF line terminators
- **Line count:** 28,154
- **Tab count:** 28,151 (confirms TSV format)
- **Sample \\r\\r\\n location:** byte offset 1,277,362: `b"\r\r\n66446220\tl'adm..."`
- **Status:** Claim verified. File has 6 instances of \\r\\r\\n (CR+CR+LF), confirming mixed line endings.

### 5. CIS_CIP_bdpm.txt — Line Endings

- **File size:** 4,151,119 bytes
- **CRLF count:** 0
- **LF-only count:** 20,903
- **CR-only count:** 0
- **File encoding (file cmd):** Unicode text, UTF-8 text, with very long lines (1,378)
- **Line count:** 20,903
- **Tab count:** 250,836 (confirms TSV format)
- **Status:** Claim verified. File uses LF line endings exclusively, not CRLF.

### 6. CIS_InfoImportantes.txt — Encoding

- **File exists:** NO
- **Searched in:** /home/devadmin/Desktop/BDMP_DB/ (all subdirectories)
- **Status:** CLAIM IS WRONG. The file does not exist in the BDMP raw data.

## Complete File Inventory

| File | Size (bytes) | Lines | Tabs | Encoding | Line Endings |
|------|-------------|-------|------|----------|--------------|
| CIS_bdpm.txt | 3,164,943 | 15,848 | 174,328 | CSV ISO-8859 | CRLF |
| CIS_CIP_bdpm.txt | 4,151,119 | 20,903 | 250,836 | UTF-8 | LF-only |
| CIS_CIP_Dispo_Spec.txt | 168,769 | 766 | 5,362 | ISO-8859 (latin-1) | CRLF |
| CIS_COMPO_bdpm.txt | 2,733,708 | 32,389 | 226,723 | ISO-8859 | CRLF |
| CIS_CPD_bdpm.txt | 1,313,810 | 28,154 | 28,151 | ISO-8859 | CRLF+6x\\r\\r\\n |
| CIS_GENER_bdpm.txt | 1,215,963 | 10,704 | 42,816 | Non-ISO ext-ASCII | CRLF |
| CIS_HAS_ASMR_bdpm.txt | 4,480,434 | 9,906 | 49,530 | Non-ISO ext-ASCII | CRLF |
| CIS_HAS_SMR_bdpm.txt | 4,493,611 | 15,257 | 76,285 | Non-ISO ext-ASCII | CRLF |
| CIS_MITM.txt | 1,136,234 | 7,710 | 23,133 | ISO-8859 | CRLF |
| HAS_LiensPageCT_bdpm.txt | 510,490 | 10,342 | 10,342 | ASCII | CRLF |

## TSV Format Confirmation

All 10 files use **tab characters** as field separators, confirming they are TSV (Tab-Separated Values) files, not fixed-width. The tab counts are consistent with the line counts (slightly fewer tabs due to trailing empty fields not being tab-terminated).

## Encoding Summary

- **UTF-8:** CIS_CIP_bdpm.txt only
- **ISO-8859 (latin-1):** CIS_bdpm.txt, CIS_CIP_Dispo_Spec.txt, CIS_COMPO_bdpm.txt, CIS_CPD_bdpm.txt, CIS_MITM.txt
- **Non-ISO extended-ASCII:** CIS_GENER_bdpm.txt, CIS_HAS_ASMR_bdpm.txt, CIS_HAS_SMR_bdpm.txt (these contain high-byte values like 0x92, 0xe9, 0xe0 that are outside ISO-8859 but used by Windows-1252)
- **ASCII:** HAS_LiensPageCT_bdpm.txt
- **Does not exist:** CIS_InfoImportantes.txt
