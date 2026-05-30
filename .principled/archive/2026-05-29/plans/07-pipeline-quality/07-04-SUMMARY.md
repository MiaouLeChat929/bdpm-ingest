# 07-04 SUMMARY — Compile-time field count const assertions

## What was done

### Task 1: Const assertions in all normalizers
Added to `src/normalize/mod.rs` — one `const ASSERT` per normalizer function:

| Function | Count | Line |
|----------|-------|------|
| normalize_cis_bdpm | 13 | 95 |
| normalize_cis_cip | 14 | 269 |
| normalize_compo | 10 | 316 |
| normalize_smr | 6 | 339 |
| normalize_asmr | 6 | 357 |
| normalize_gener | 5 | 375 |
| normalize_cpd | 2 | 392 |
| normalize_dispo | 8 | 406 |
| normalize_mitm | 3 | 426 |
| normalize_liens | 2 | 442 |
| normalize_info_importantes | 5 | 456 |

Also fixed the comment in `normalize_cis_bdpm` from "12 fields" to "13 fields".

### Task 2: Dedup const assertion + test
- Added `const COMPO_EXPECTED_FIELDS: usize = 10` assertion in `dedup_compo`
- Added `test_dedup_key_matches_pk` — verifies different seq values are NOT deduplicated (documents Phase 06 fix)

## Verification
- `cargo test --lib`: 177 passed
- `cargo clippy -- -D warnings`: clean
- `cargo build --release`: succeeds

## Files modified
- `src/normalize/mod.rs` — const assertions in all 11 normalizers + comment fix
- `src/normalize/dedup.rs` — const assertion + test_dedup_key_matches_pk test
