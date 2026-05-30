# Rust Stack for SQLite-Backed Drug Database API

**Project:** CLI tool (TSV import) + read-only HTTP API  
**Scale:** ~150K rows across 10 tables, ~50-100MB SQLite  
**Requirements:** FTS5 full-text search, static musl binary, GitHub Actions CI/CD  
**Constraints:** Solo developer, simplicity > raw performance

---

## 1. HTTP FRAMEWORK

### Landscape Summary (2025-2026)

| Framework | Status | Stars (approx) | Key Characteristic |
|-----------|--------|----------------|-------------------|
| **Axum** | Actively maintained (v0.8.x) | ~14K | Tokio-based, Tower ecosystem, extractor pattern |
| **Actix Web** | Mature, stable | ~17K | Fastest raw performance, actor model |
| **Rocket** | Stable (v0.5.x) | ~24K | Developer happiness, batteries-included |
| **Salvo** | Actively developed | ~4K | HTTP/3 support, auto-TLS, OpenAPI generation |
| **Warp** | Minimal maintenance | ~5K | Functional filter composition |
| **Viz** | Niche | ~1K | Minimalist, LinkChecker ecosystem |
| **Thruster** | Niche | ~1K | Express-inspired |

### Analysis

**Axum** is now the de facto standard for async Rust APIs. Built by the Tokio team, it offers:
- Extractor pattern for type-safe request parsing
- Full Tower middleware compatibility (rate limiting, tracing, compression)
- Modular routing that scales to 100+ endpoints
- Clean compile errors

**Actix Web** offers 10-15% better raw throughput, but the actor model adds complexity. For a read-only API with modest traffic, this difference is negligible.

**Viz** is minimalist (under 5K LOC) and clean, but smaller ecosystem.

**For SQLite read-only APIs specifically:** The synchronous-vs-async question matters here. SQLite itself is synchronous. Adding async adds ceremony without benefit unless you need concurrent connection handling.

### Recommendation: **Axum**

- Best ecosystem integration
- Scales cleanly as requirements grow
- Tower middleware covers common needs
- Extractor pattern reduces boilerplate

---

## 2. ASYNC RUNTIME: Do You Even Need Async?

### The Key Insight

SQLite's locking model and synchronous nature means async doesn't provide the concurrency benefits it does for network-heavy workloads. A thread-per-request model with `std::thread` or scoped threads performs comparably for read-heavy workloads.

### Options Considered

| Runtime | Status | Memory/Traits |
|---------|--------|---------------|
| **tokio** | Dominant (~42% of new projects) | Heavy, multi-threaded by default |
| **smol** | Active, lightweight | ~1000 LOC executor |
| **async-std** | **DISCONTINUED** (March 2025) | Was alternative, now abandoned |

`async-std` was officially discontinued in March 2025. The recommended path forward is **`smol`** for lightweight async needs.

### Practical Decision

For a read-only SQLite API serving ~50-100MB of data:

**Option A (Synchronous):** `tiny-http` or raw `hyper` with synchronous handlers
- Simpler mental model
- No `Send + 'static` constraints
- No trait bounds gymnastics
- Thread per request scales fine for moderate load

**Option B (Async if needed):** Just use Axum which wraps Tokio
- Mature ecosystem
- Tower middleware ready
- Standard patterns if you expand to other backends

### Recommendation: **Synchronous is fine** for this use case

For a read-only API with SQLite as backend, a synchronous framework like `tiny-http` (4K stars, plain HTTP, no async) would be simpler. However, **Axum with a synchronous wrapper** or **using threads** achieves the same simplicity while keeping async optional if you later add network calls (HTTP clients, etc.).

If staying pure sync: **`tiny-http`** or **`rouille`**

If embracing minimal async: **Axum** (Tokio is pulled in transitively anyway)

---

## 3. SQLITE BINDINGS

### Options

| Binding | Type | Async | Key Feature |
|---------|------|-------|-------------|
| **rusqlite** | Synchronous wrapper | No | `bundled` SQLite, `r2d2` pool support |
| **sqlx** | SQL toolkit | Fake-async (spawns threads) | Compile-time query checking |
| **turbosql** | Derive macros | No | Simpler API than rusqlite |
| **sea-orm** | ORM | Yes | Django-like, but overkill here |

### Recommendation: **rusqlite**

For this project:

- **Bundled SQLite feature is ideal:** Compiles SQLite into your binary, no system dependency
- **Synchronous by nature:** Matches SQLite's design
- **FTS5 exposes directly:** Full access to SQLite features
- **Simple pool with `r2d2` or `bb8`:** Connection pooling for concurrent requests

`sqlx` adds complexity without benefit for SQLite. The "async" mode is fake-async—it spawns blocking threads. You'd get async/await syntax but sync execution underneath. For a read-only API, this is unnecessary.

---

## 4. SERIALIZATION

### Options

| Library | Performance | Notes |
|---------|------------|-------|
| **serde_json** | Baseline | Standard, well-supported |
| **simd-json** | ~2x faster parsing | SIMD-based, serde-compatible |

### Recommendation: **serde_json** (for simplicity)

For ~150K rows, you're not JSON-constrained—you're I/O-bound at the SQLite level. `serde_json` is:
- The standard, battle-tested choice
- Zero additional overhead beyond your data
- Widely understood by the community

Use `simd-json` if profiling shows JSON parsing as a bottleneck. For a CLI tool imported from TSV, this is unlikely.

---

## 5. CLI FRAMEWORK

### Options

| Library | Size | Features | Best For |
|---------|------|----------|----------|
| **clap** | Large | Full-featured, derive macros | Complex CLIs, 5+ subcommands |
| **pico-args** | Minimal | Zero deps, fast | Simple argument parsing |
| **lexopt** | Minimal | No macros | Hand-rolled, 2-3 args |
| **argh** | Medium | Google's approach | Fuchsia-friendly (not Unix-friendly) |

### Recommendation: **clap** (derive style, v4)

For ~4-5 subcommands:
- Derive-style is clean: `#[derive(Parser)]`
- Handles help generation, suggestions (Jaro-Winkler distance), all Unix conventions
- Well-documented ecosystem
- Slightly slower compile time, but acceptable for CLI tools

`pico-args` is tempting for size, but for 5 subcommands, clap's ergonomic benefits outweigh binary size concerns for your use case.

---

## 6. ERROR HANDLING

### Options

| Library | Use Case | Pattern |
|---------|---------|---------|
| **thiserror** | Defining custom error types | Derive macros |
| **anyhow** | Context-rich application errors | `Result<T, anyhow::Error>` |
| **eyre** | anyhow alternative | Similar API |
| **snafu** | Typed errors with context | Verbose but flexible |
| **error-stack** | Modern, context frames | Newer entrant |

### Recommendation: **thiserror + anyhow** (standard pair)

- `thiserror` for library/command error enums (you likely need domain errors)
- `anyhow` for application entry points where you want context-rich errors
- 2025/2026 standard recommendation from community

Avoid `snafu` unless you have complex nested error scenarios—it adds significant learning curve.

---

## 7. LOGGING/TRACING

### Options

| Approach | Complexity | Use Case |
|----------|-----------|----------|
| **log + env_logger** | Minimal | Simple CLI tools |
| **tracing + tracing-subscriber** | Medium | Distributed systems, async |
| **opentelemetry** | Heavy | Production observability |

### Recommendation: **log + env_logger** (for CLI)

For a CLI tool:
- `log` crate provides the facade
- `env_logger` or `pretty_env_logger` for output
- No need for structured spans unless doing distributed tracing

`tracing` is overkill unless you have async tasks you want to instrument with spans. Your use case is straightforward and benefits from simplicity.

However: If using Axum/tokio, `tracing` integrates naturally via tower layers. In that case, use `tracing` for consistency with your HTTP layer.

---

## 8. TESTING

### Recommendation: Built-in `#[test]` + minimal setup

For this project:
- Unit tests with `#[cfg(test)]`
- Integration tests for CLI scenarios
- SQLite in-memory for test fixtures

Property testing (`proptest`) is valuable for data transformations but adds complexity you likely don't need.

Minimal viable setup:
```toml
[dev-dependencies]
tempfile = "3"
```

---

## 9. BUILD/TOOLING

### Options

| Tool | Complexity | Use Case |
|------|-----------|----------|
| **cargo alone** | Minimal | Simple projects |
| **just** | Low | Task runner, like `make` |
| **cargo-make** | Medium | Complex pipelines |

### Recommendation: **cargo alone or just**

For your case:
- **cargo alone** is sufficient for most build/CI tasks
- **just** if you want shell-like convenience for common tasks (clean, build, test sequences)
- CI tasks in GitHub Actions can be cargo commands

`cargo-make` adds YAML complexity for marginal benefit unless you have complex multi-step pipelines.

---

## 10. FTS5 INTEGRATION

### Approach

SQLite FTS5 is used via SQL queries directly with `rusqlite`:

```sql
-- Create FTS5 virtual table
CREATE VIRTUAL TABLE drugs_fts USING fts5(drug_name, content='drugs', content_rowid='id');

-- Populate
INSERT INTO drugs_fts(rowid, drug_name) SELECT id, drug_name FROM drugs;

-- Query
SELECT d.* FROM drugs d JOIN drugs_fts f ON d.id = f.rowid 
WHERE drugs_fts MATCH 'aspirin*';
```

`rusqlite` handles parameterized queries correctly for FTS5 syntax.

---

## RECOMMENDED STACK

For a solo developer prioritizing **simplicity, maintainability, and modern Rust practices**:

| Dimension | Choice | Rationale |
|-----------|--------|-----------|
| **HTTP Framework** | **Axum** | Standard ecosystem, Tower middleware, scales to 100+ endpoints |
| **Async Runtime** | **tokio** (via Axum) | Dominant ecosystem; async not strictly needed but Axum requires it |
| **SQLite Binding** | **rusqlite** | Bundled SQLite, direct FTS5 access, synchronous simplicity |
| **Serialization** | **serde_json** | Standard, sufficient for 150K rows |
| **CLI Framework** | **clap** (derive) | Clean subcommands, help generation, proven |
| **Error Handling** | **thiserror + anyhow** | Standard pair, compile-time and context errors |
| **Logging** | **tracing + tracing-subscriber** | If using Axum (already there via tower); otherwise log + env_logger |
| **Testing** | Built-in test | Minimal fixtures, in-memory SQLite |
| **Build Tooling** | **cargo alone** | CI scripts in GitHub Actions |
| **Static Binary** | **musl target** | `rustup target add x86_64-unknown-linux-musl` |

---

## ALTERNATIVE: PURE SYNCHRONOUS STACK

If you want absolute minimal complexity and don't need async for anything:

| Dimension | Choice |
|-----------|--------|
| HTTP Framework | **tiny-http** or **rouille** |
| SQLite | **rusqlite** (same) |
| CLI | **clap** (same) |
| Runtime | None (std::thread or scoped threads) |

This is simpler but gives up the Tower ecosystem. For a read-only API on SQLite, this may actually be ideal—but Axum is close enough in complexity that the ecosystem benefits win.

---

## BUILD CONFIGURATION FOR MUSL

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
linker = "musl-gcc"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

GitHub Actions CI/CD:
```yaml
- name: Build musl binary
  run: |
    rustup target add x86_64-unknown-linux-musl
    cargo build --release --target x86_64-unknown-linux-musl
```

---

## CONCLUSION

The 2025-2026 Rust ecosystem has matured. For SQLite-backed APIs:
- **rusqlite** is the obvious choice—discontinue overthinking it
- **Axum** provides the best ecosystem balance for HTTP
- **Clap** gives you ergonomic CLI without ceremony
- **thiserror + anyhow** is the standard error pair

The synchronous vs async question for SQLite is a nuanced one. For your scale, synchronous with `rusqlite` is fine. But Axum + tokio is close enough in complexity that the ecosystem wins.

Start with the recommended stack. Only optimize if profiling shows issues.
