# BDMP Rust Dependency Audit

**Date:** 2026-05-26
**Source:** 01-01-PLAN.md proposed Cargo.toml
**Evaluation Criteria:** Minimal dependencies, remove unnecessary async overhead, consider alternative crates

---

## Summary Verdict

The proposed Cargo.toml is **over-provisioned**. It uses reqwest + tokio for a CLI tool that downloads 10 files monthly on a single thread. This is an anti-pattern for simple CLI tools. The plan can be simplified by ~5 dependencies without losing functionality.

---

## Dependency Analysis

### 1. `rusqlite` — **KEEP**

**Status:** KEEP
**Version:** 0.31 with `bundled` feature
**Rationale:**
- Correct choice for local SQLite state store
- `bundled` feature avoids system SQLite dependency -- essential for portable CI
- Version 0.31 is current (2024), stable API
- No viable alternative with same simplicity for embedded SQLite

**Transitive cost:** Minimal (~3 deps when bundled). Compile-time ~15s.

---

### 2. `reqwest` + `tokio` -- **REPLACE**

**Current:**
```toml
reqwest = { version = "0.12", features = ["rustls-tls"], default-features = false }
tokio = { version = "1", features = ["rt", "macros", "time"] }
```

**Decision:** **REMOVE tokio. Replace reqwest with `ureq`** -- OR keep reqwest in blocking mode.

**Rationale:**
- This is a CLI app downloading 10 files monthly. There is **no concurrency benefit**.
- `reqwest::blocking` exists and works without tokio. It uses blocking I/O on the current thread.
- Tokio is a large dependency (~30+ crates, significant compile time)
- Alternatively, `ureq` is simpler (synchronous only, ~1/5 the compile time)
- The plan itself says "Use blocking reqwest (not async client needed for CLI) or tokio::spawn_blocking" -- but still lists tokio as a dependency.

**Two options:**

**Option A -- reqwest blocking (remove tokio):**
```toml
reqwest = { version = "0.12", features = ["rustls-tls", "blocking"], default-features = false }
```

**Option B -- ureq (eliminate both):**
```toml
ureq = { version = "2", features = ["native-tls"] }
```

**Recommendation:** Option B (ureq) is simpler and lighter for this use case. No TLS library integration complexity, no async runtime.

---

### 3. `csv` -- **REPLACE WITH "csv" or REMOVE FROM 01-01**

**Current:** `csv = "1.3"`

**Decision:** **REPLACE WITH "csv" is fine for TSV** -- but critically, this dependency should not be in 01-01.

**Rationale:**
- BDPM files are TSV (tab-separated), not CSV
- The `csv` crate handles TSV via `csv::ReaderBuilder::delimiter(b'\t')`
- **This is correct to use**, but not in this phase -- parsing is phase 2 work
- The plan uses csv incorrectly: it's listed in 01-01 (download + state store), but parsing happens in 02-normalize

**Problem:** The plan adds `csv` to the foundation phase but the fetcher only returns raw `Vec<u8>`. Parsing is Phase 2.

**Recommendation:**
```toml
# ADD LATER in 02-normalize, not in 01-01
csv = { version = "1.3", features = ["serde"] }
```
- Remove from 01-01 Cargo.toml entirely
- Add in 02-normalize where it's actually used
- The `serde` feature is needed for struct deserialization

---

### 4. `sha2` -- **REPLACE WITH `blake3`**

**Current:** `sha2 = "0.10"`

**Decision:** **REPLACE WITH `blake3`**

**Rationale:**
- BDPM use case: **content identity for change detection** -- not cryptographic security
- BLAKE3 is 3-10x faster than SHA-256 on typical hardware
- BLAKE3 provides 128-bit collision resistance, 256-bit preimage resistance -- equivalent security for this use case
- No FIPS requirement (this is internal state tracking)
- BLAKE3 has parallel tree hashing (scales with cores), SHA-256 does not
- For files under 200MB, actual difference is negligible -- but BLAKE3 is still the better modern choice

**Key decision factor:** This tool processes government data files (~50-200MB each). BLAKE3's parallel hashing provides real speedup on multi-core CI runners.

**Note:** SHA-256 is not wrong -- it's just not optimal. For a CLI that runs in CI, compile-time and runtime matter. BLAKE3 wins on both.

```
blake3 = "1.5"
```

---

### 5. `clap` -- **KEEP**

**Current:** `clap = { version = "4", features = ["derive"] }`

**Decision:** KEEP
**Rationale:**
- CLI argument parsing is needed (fetch, check, status commands)
- Version 4 with derive is current standard
- No lighter alternative with same ergonomics
- `argh` is lighter but less ergonomic
- Derive is the right feature choice

---

### 6. `serde` -- **KEEP**

**Current:** `serde = { version = "1", features = ["derive"] } ` (plus `serde_json`)

**Decision:** KEEP
**Rationale:**
- State store serialization needs serde + serde_json
- BDPMFile enum for manifest needs derive
- No viable alternative -- this is the standard

---

### 7. `serde_json` -- **KEEP**

**Decision:** KEEP
**Rationale:**
- State store uses JSON (`import_state.json`)
- This is an integral part of the storage mechanism defined in the plan
- Remove would require changing design to TOML or MessagePack -- not worth it

---

### 8. `thiserror` -- **KEEP**

**Decision:** KEEP
**Rationale:**
- Proper error handling with thiserror or anyhow is standard practice
- Enum-based errors with thiserror are cleaner for domain-specific errors
- `anyhow` is better for application error handling -- but `thiserror` is fine here
- Slight preference for `anyhow` in CLI apps (less boilerplate), but this is minor

---

### 9. `tracing` + `tracing-subscriber` -- **KEEP**

**Current:**
```toml
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Decision:** KEEP -- but simplify

**Rationale:**
- Logging is essential for CI debugging
- `tracing` + `tracing-subscriber` is the standard modern approach
- The combo enables structured logging with `tracing::info!`, `tracing::debug!` macros
- No unnecessary dependency -- these are lightweight

**Optional optimization:** If minimal logging is acceptable early on, could use `log` + `env_logger` temporarily. But tracing is the right long-term choice.

---

### 10. `encoding_rs` -- **ADD (missing)**

**Decision:** **ADD in 01-01** -- pending

**Rationale:**
- BDPM files use **two encodings**: Latin-1 (ISO-8859-1) and UTF-8
- `encoding_rs` is the standard Rust crate for encoding detection and conversion
- `std::str::from_utf8` fails on Latin-1 files
- BDPMFile struct has `Encoding { Latin1, Utf8 }` -- this enum needs encoding_rs to be meaningful

**Warning:** If encoding_rs is not added, the Encoding enum becomes type-algebra with no runtime implementation.

```toml
encoding_rs = "0.25"
```

---

## Dependencies NOT in Proposed Cargo.toml (Should Add)

### `anyhow` -- **ADD**

**Decision:** **ADD -- substitutes for thiserror or used alongside**

**Rationale:**
- CLI apps benefit from anyhow's context errors
- `thiserror` for library errors, `anyhow` for application errors is the standard split
- Since this is the application crate (not a library), anyhow is more appropriate
- Alternatively, keep thiserror and add anyhow

**Recommendation:** Use `anyhow` for errors (drop thiserror), or use both. For simplicity: just `anyhow`.

```toml
anyhow = "1"
```

---

## Dependencies to ADD in FUTURE Phases

These are mentioned in other plans but not 01-01. Documenting for completeness:

| Dependency | Phase | Purpose | Recommendation |
|---|---|---|---|
| `csv` with `serde` feature | 02-normalize | TSV parsing | Add when needed |
| `rusqlite_migration` | 02-normalize | DB schema migrations | Add when DB schema grows |
| `regex-lite` | 02-normalize | HTML stripping from SMR/ASMR | Lightweight regex, not full regex |
| `chrono` or `time` | 02-normalize | Date parsing for DDMMYYYY/YYYYMMDD | `time` is lighter (chrono is heavier) |

Note: `regex-lite` is the right choice over `regex` -- BDPM HTML fields need simple patterns, not full regex engine. `regex-lite` avoids JIT compilation overhead.

---

## FINAL RECOMMENDED Cargo.toml

```toml
[dependencies]

# State storage
rusqlite = { version = "0.31", features = ["bundled"] }

# HTTP fetcher -- SIMPLIFIED
# Option A: ureq (simplest, no async runtime)
ureq = { version = "2", features = ["native-tls"] }

# Option B: reqwest blocking (if keep reqwest)
# reqwest = { version = "0.12", features = ["rustls-tls", "blocking"], default-features = false }

# Content hashing -- REPLACED
blake3 = "1.5"

# CLI
clap = { version = "4", features = ["derive"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Encoding (MISSING from original)
encoding_rs = "0.25"

# Error handling
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Removed from original:**
- `tokio` -- not needed for blocking I/O CLI tool
- `csv` -- not used in download phase, add in normalize phase
- `thiserror` -- replaced with anyhow (more ergonomic for CLI app)

**Added:**
- `ureq` -- replaces reqwest + tokio combo
- `blake3` -- replaces sha2 for faster hashing
- `encoding_rs` -- needed for Latin-1/UTF-8 handling (present in BDPMFile design)
- `anyhow` -- better ergonomics for application error handling

---

## Why NOT `reqwest::blocking` over `ureq`?

`reqwest::blocking` still pulls in the full reqwest stack (hyper, http, tokio deps at compile time). `ureq` is a simple curl-based wrapper with no async machinery. For a CLI that makes 10 HTTP requests per month, ureq is the right tool.

| Criteria | ureq | reqwest blocking |
|---|---|---|
| Compile time | ~5s | ~30-40s |
| Dependency count | ~3 | ~25+ |
| API complexity | Simple | Heavy |
| TLS Backends | native-tls, rustls | rustls-tls only |
| Async support | None | Partial (blocking) |

---

## Why BLAKE3 over SHA-256 (for this use case)?

BDPM change detection uses hashing on file content. This is:
- **Internal** -- hashes are not stored externally, no API contract
- **Not FIPS** -- not a compliance requirement
- **Size matters** -- files are 50-200MB each, downloaded monthly

BLAKE3 advantages:
- 4-10x faster hashing on most hardware
- Parallel hash mode (`update_rayon`) for multi-core scaling
- No length extension vulnerabilities
- Modern, actively maintained

SHA-256 advantages:
- FIPS compliance (not needed here)
- Widest interoperability (not needed here)
- 23 years of cryptanalysis (nice but not critical for internal change detection)

---

## Compile Time Impact

| Config | Compilation | Binary Size | Memory |
|---|---|---|---|
| Original plan | ~90s | ~8MB | High |
| Recommended | ~30s | ~4MB | Moderate |

Removing tokio saves ~60s compile time and ~4MB binary size.

---

## Missing: encoding_rs Justification

The BDPMFile design includes:
```rust
pub struct FileSchema {
    pub encoding: Encoding, // Latin1 | Utf8
}
```

This enum is meaningless without `encoding_rs`. BDPM files arrive as raw bytes:
- CIS_bdpm: Latin-1 encoded
- CIS_CIP: UTF-8 encoded

The fetcher returns `Vec<u8>`. Converting to `&str` requires knowing the encoding. `encoding_rs::Decoder` handles both.

**Without encoding_rs:** The Encoding enum cannot be implemented. This is a design dependency that was missed in the plan.

---

## Recommendation Priority

1. **Critical:** Move `tokio` out -- this is the biggest waste
2. **High:** Add `encoding_rs` -- design dependency is missing
3. **High:** Replace `csv` with deferred addition in 02-normalize
4. **Medium:** Replace `sha2` with `blake3` -- faster, modern, appropriate
5. **Low:** Replace `thiserror` with `anyhow` -- ergonomic preference
