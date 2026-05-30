# Substance Name Quality & Salt Stripping Analysis

## Database: bdpm.db

---

## 1. Sample Substance Names (Top 30 by Frequency)

| substance_name | substance_code | count |
|---|---|---|
| HYDROCHLOROTHIAZIDE | 02064 | 284 |
| PARACÉTAMOL | 02202 | 230 |
| AMLODIPINE | 39727 | 166 |
| ATORVASTATINE | 19809 | 159 |
| BÉSILATE D'AMLODIPINE | 93748 | 157 |
| PÉRINDOPRIL | 52431 | 150 |
| ATORVASTATINE CALCIQUE TRIHYDRATÉE | 92464 | 141 |
| AMOXICILLINE | 05248 | 132 |
| VALSARTAN | 15734 | 130 |
| ÉZÉTIMIBE | 73242 | 129 |
| PRÉGABALINE | 39422 | 124 |
| ÉTHINYLESTRADIOL | 01807 | 121 |
| AMOXICILLINE TRIHYDRATÉE | 28165 | 116 |
| SODIUM | 00901 | 116 |
| GLUCOSE | 00529 | 112 |
| GLUCOSE MONOHYDRATÉ | 17150 | 107 |
| IBUPROFÈNE | 02092 | 104 |
| FENTANYL | 17477 | 100 |
| TRAMADOL | 86571 | 99 |
| CANDÉSARTAN CILEXÉTIL | 73594 | 92 |
| IRBÉSARTAN | 31416 | 85 |
| ROSUVASTATINE | 33207 | 82 |
| FUMARATE DE BISOPROLOL | 07436 | 80 |
| CHOLÉCALCIFÉROL | 01525 | 77 |
| OLANZAPINE | 17723 | 77 |
| SIMVASTATINE | 10171 | 77 |
| METFORMINE | 24321 | 74 |
| MÉTHOTREXATE | 02345 | 70 |
| PÉRINDOPRIL ARGININE | 69916 | 70 |
| TADALAFIL | 98159 | 69 |

**Key observation:** Salt forms and hydrated forms appear frequently in the raw data.

---

## 2. Salt Forms Still Present in substance_name

| Pattern | Count |
|---|---|
| Contains "chlorhydrate" | 378 |
| Contains "sulfate de" | 72 |
| Contains parentheses "(...)" | 1,328 |

---

## 3. Salt Stripping Effectiveness

### Code Review

The `normalize_compo` function in `src/normalize/mod.rs` calls `strip_salt()`:

```rust
fn normalize_compo(f: &[String]) -> NormalizedRow {
    let substance_name_clean = strip_salt(&normalize_spaces(&strip_field(&f[3])));
    NormalizedRow {
        table: "compositions",
        values: vec![
            // ...
            Some(strip_salt(&strip_field(&f[3]))),  // substance_name (raw)
            // ...
            Some(substance_name_clean),             // substance_name_clean
            // ...
        ],
    }
}
```

Both `substance_name` AND `substance_name_clean` are stored, with salt stripping applied to both.

### Current SALT_SUFFIXES (from `src/normalize/fields.rs`)

```rust
pub static SALT_SUFFIXES: &[&str] = &[
    "chlorhydrate monohydrate", "chlorhydrate anhydre",
    "chlorhydrate",
    "sulfate anhydre", "sulfate",
    "malate", "bromhydrate",
    "tartrate", "glycolate",
    "base anhydre", "base",
    "sel sodique", "sel",
    "dihydrate", "trihydrate", "monohydrate", "anhydre",
    "sel de sodium",
    "chlorhydrate de sodium",
];
```

### Gaps Found

| Gap | Example Raw | Expected Clean | Actual Clean |
|---|---|---|---|
| Missing "calcique" suffix | ATORVASTATINE CALCIQUE TRIHYDRATÉE | ATORVASTATINE | ATORVASTATINE CALCIQUE TRIHYDRATÉE |
| Missing "trihydratée" (accent) | AMOXICILLINE TRIHYDRATÉE | AMOXICILLINE | AMOXICILLINE TRIHYDRATÉE |
| Missing "sodique" suffix | DICLOFÉNAC SODIQUE | DICLOFÉNAC | DICLOFÉNAC SODIQUE |
| Missing "arginine" suffix | PÉRINDOPRIL ARGININE | PÉRINDOPRIL | PÉRINDOPRIL ARGININE |
| Missing "cilexétil" suffix | CANDÉSARTAN CILEXÉTIL | CANDÉSARTAN | CANDÉSARTAN CILEXÉTIL |

### Salt Forms Remaining in substance_name_clean

| substance_name_clean | Count |
|---|---|
| LÉVOTHYROXINE SODIQUE | 113 |
| ATORVASTATINE CALCIQUE TRIHYDRATÉE | 141 |
| AMOXICILLINE TRIHYDRATÉE | 116 |
| ROSUVASTATINE CALCIQUE | 82 |
| AMOXICILLINE SODIQUE | 16 |
| DICLOFÉNAC SODIQUE | 67 |
| CÉFTRIAXONE SODIQUE | 39 |
| MONTÉLUKAST SODIQUE | 44 |
| PRAVASTATINE SODIQUE | 43 |
| RABÉPRAZOLE SODIQUE | 29 |
| AND 50+ more "sodique" forms |

**Total "sodique" entries in substance_name_clean:** ~600+

---

## 4. Code vs Name Cardinality

| Metric | Count |
|---|---|
| Distinct substance_code | 3,039 |
| Distinct substance_name | 3,215 |
| Distinct substance_name_clean | ~2,800 (estimated) |

The 176 extra distinct names vs codes suggests some codes map to multiple name variants (e.g., different salt forms).

---

## 5. FT vs SA Distribution

| pharm_code | Count |
|---|---|
| FT | 5,496 |
| SA | 18,600 |

**Ratio:** SA (substance active) is 3.4x more common than FT (excipient).

---

## 6. Schema

The `compositions` table has both fields:

```sql
substance_name     TEXT,     -- raw, with strip_salt applied
substance_name_clean TEXT,   -- also with strip_salt applied
```

---

## 7. Findings Summary

### What's Working
- Salt prefixes are stripped (e.g., "chlorhydrate de X" -> "X")
- Multi-pass suffix stripping works
- "dihydrate", "monohydrate", "anhydre" are stripped

### What's Not Working
1. **Accented suffixes not matched:** "trihydrate" is stripped but "trihydratée" (with accent) is not
2. **"calcique" (calcium) not in SALT_SUFFIXES:** Affects atorvastatin, rosuvastatine, nadroparine, etc.
3. **"sodique" (sodium) not stripped:** ~90+ substance names affected, including common drugs (diclofénac, pravastatine, cefriaxone)
4. **Prodrug suffixes not stripped:** "arginine", "cilexétil", " fumarate", etc.
5. **Parentheses not stripped in substance_name:** 1,328 entries contain "(...)"

### Recommendations

1. **Add to SALT_SUFFIXES:**
   - "calcique" (calcium salts)
   - "sodique" (sodium salts)
   - "trihydratée", "dihydratée", "monohydratée" (accented forms)

2. **Consider adding:**
   - "arginine"
   - "cilexétil"
   - "fumarate" (already has "malate", "tartrate", etc.)

3. **Call `strip_parens()` in `normalize_compo`:**
   ```rust
   Some(strip_parens(&strip_salt(&strip_field(&f[3])))),
   ```

4. **Alternatively:** Create a mapping table for known salt-to-base conversions based on substance_code.
