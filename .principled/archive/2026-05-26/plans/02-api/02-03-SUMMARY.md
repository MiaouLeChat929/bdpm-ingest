# 02-03-SUMMARY — Generic Groups + ATC Browse + Availability API

## Completed

Implemented 4 new API endpoints:

### `/generic-groups` — Generic Groups Browse

**`src/api/groups.rs`**
- `GET /generic-groups` — lists all generic groups with CIS count
- `GET /generic-groups/:group_id` — drugs in a specific generic group

```rust
// list_generic_groups
SELECT group_id, group_name, COUNT(cis) as cis_count
FROM generic_groups GROUP BY group_id, group_name ORDER BY group_id

// generic_group_detail
SELECT g.cis, d.name, g.type, g.sort_order, g.is_orphan
FROM generic_groups g LEFT JOIN drugs d ON g.cis = d.cis
WHERE g.group_id = ?1 ORDER BY g.sort_order, d.name
```

### `/atc` — ATC Hierarchy Browse

**`src/api/atc.rs`**
- `GET /atc` — top-level ATC codes (1-char chapters)
- `GET /atc/:code` — detail with child codes and drug count via mitm join

```rust
// atc_top_level
SELECT atc_code, parent_1_char FROM atc_codes WHERE LENGTH(atc_code) = 1 ORDER BY atc_code

// atc_detail — child length by current length:
// 1→3, 3→4, 4→5, 5→7
SELECT atc_code FROM atc_codes WHERE atc_code LIKE ?1 AND LENGTH(atc_code) = ?2
SELECT COUNT(DISTINCT cis) FROM mitm WHERE atc_code = ?1
```

### `/availability` — Availability/Sales Status

**`src/api/availability.rs`**
- `GET /availability` — recent availability rows (limit 200)
- `GET /availability?cis=XXX` — availability for specific drug
- `GET /availability?status=1` — all drugs in rupture (status_type=1)

```rust
// availability — three query variants with LEFT JOIN drugs d ON a.cis = d.cis
```

## Implementation Details

All endpoints use `spawn_blocking` for rusqlite calls (blocking I/O in async runtime):
```rust
let rows = tokio::task::spawn_blocking(move || {
    let conn = Connection::open(&state.db_path).unwrap();
    // ... queries
}).await.unwrap();
```

## Verification

```
$ cargo build --lib
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s

$ cargo test --lib
test result: ok. 24 passed; 0 failed;
```

## Routes Added to `src/api/mod.rs`

```rust
.route("/generic-groups", get(groups::list_generic_groups))
.route("/generic-groups/:group_id", get(groups::generic_group_detail))
.route("/atc", get(atc::atc_top_level))
.route("/atc/:code", get(atc::atc_detail))
.route("/availability", get(availability::availability))
```

## Notes

- All 4 endpoint modules wire through `crate::api::AppState`
- No regressions to existing tests (24 passing)
- Compatible with 02-01 FTS5 scaffold (same `AppState` type)
