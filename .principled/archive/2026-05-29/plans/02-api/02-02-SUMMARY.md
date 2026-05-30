# 02-02-SUMMARY — Drug Detail + Presentations + Compositions API

## Objective
Implement `GET /drugs/:cis` endpoint returning full drug detail: basic info, all presentations, all compositions.

## Implementation

### Files Created/Modified

**`src/api/drugs.rs`** — Drug detail endpoint:
- `DrugDetail` struct with all drug fields + presentations + compositions
- `Presentation` struct for CIP codes with prices, EAN-13, reimbursement info
- `Composition` struct for substance name, dosage, pharm code
- `ApiError` enum with NotFound/Internal variants implementing `IntoResponse`
- `drug_detail` async handler using `spawn_blocking` for DB access

**`src/api/mod.rs`** — Route registration:
- Added `.route("/drugs/:cis", get(drugs::drug_detail))`
- Exported `drug_detail` for use in router

## Verification

- `cargo build --lib` — compiles successfully (15 warnings, no errors)
- `cargo test --lib` — all 24 tests pass (no regressions)
- `spawn_blocking` pattern verified in all API handlers
- HTTP error codes: 404 for unknown CIS, 500 for internal errors

## Key Design Decisions

1. **spawn_blocking wrapper** — all rusqlite calls run in tokio blocking thread pool to avoid blocking async runtime
2. **Error type** — `ApiError` enum with proper `IntoResponse` for axum compatibility
3. **Struct patterns** — derive Serialize for JSON responses, use Option<T> for nullable fields
4. **Query optimization** — separate queries for drug row, presentations, and compositions (no JOINs needed)

## Output

Endpoint available at `GET /drugs/:cis` returning:
```json
{
  "cis": "60003620",
  "name": "ASPIRINE",
  "form": "comprimé",
  "route": "orale",
  "auth_status": "Autorisation active",
  "procedure_type": "Procédure nationale",
  "auth_date": "1994-04-07",
  "lab_name": "BAYER HEALTHCARE",
  "is_patent": true,
  "atc_code": "N02BA01",
  "presentations": [...],
  "compositions": [...]
}
```