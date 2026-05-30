# Minimal Rust Stack for SQLite-over-HTTP

**Date:** 2026-05-26
**Purpose:** Evaluate the simplest production-grade Rust stack for serving SQLite over HTTP

---

## Executive Summary

For a minimal SQLite-over-HTTP stack in Rust, the choice narrows to three paths:

| Approach | Framework | Best For |
|----------|-----------|----------|
| **Sync-first** | `rouille` + `rusqlite` | Maximum simplicity, trivial async bridging |
| **Async-first** | `axum` + `rusqlite` (via `spawn_blocking`) | Production-grade async, Tower ecosystem |
| **Emerging** | `salvo` | HTTP/3, auto-TLS, OpenAPI generation |

**Recommendation:** Use **axum + sqlx** for production with SQLite, or **rouille + rusqlite** if binary size and simplicity are paramount.

---

## Framework Analysis

### 1. Rouille + Rusqlite

**Status:** STALE (last commit ~2022, but actively used)

| Metric | Value |
|--------|-------|
| GitHub Stars | 1.2k |
| Maintenance | Minimal (author tomaka semi-active) |
| Last Release | ~2022 (v3.0 stable) |
| Binary Size (stripped) | ~1.2MB |
| Compile Time | Fast (no async deps) |
| Learning Curve | 1/5 (linear, no magic) |
| SQLite Integration | 5/5 (sync direct) |

**Key Findings:**
- Built on `tiny-http`, pure synchronous I/O
- Each request handled in dedicated thread (thread pool)
- FAQ explicitly states: "Once async I/O has been figured out, rouille will be updated"
- Benchmarks show ~22k req/sec (vs 77k tokio-minihttp, 51k Go, 39k nginx)
- No keep-alive support (per FAQ)
- Perfect for: scripts, internal tools, minimal binaries

**Production Considerations:**
- No middleware system (linear request handling)
- No built-in database support (but integrates trivially with rusqlite)
- No HTTPS built-in (would need tiny-http + rustls)
- Acceptable for internal use; questionable for public-facing

**Example Pattern:**
```rust
use rouille::{router, Response};
use rusqlite::Connection;

fn main() {
    let db = Connection::open("data.db").unwrap();

    rouille::start_server("0.0.0.0:8000", move |request| {
        router!(request,
            (GET) ("/api/items") => {
                let mut stmt = db.prepare("SELECT * FROM items").unwrap();
                let items: Vec<_> = stmt.query_map([], |row| {
                    Ok((row.get::<_, i64>(0), row.get::<_, String>(1)))
                }).unwrap().filter_map(|r| r.ok()).collect();
                Response::json(&items)
            },
            _ => Response::empty_404()
        )
    });
}
```

---

### 2. Tiny-HTTP + Rusqlite

**Status:** ACTIVE (maintained, used by rouille)

| Metric | Value |
|--------|-------|
| GitHub Stars | ~600 |
| Crates.io Version | 0.11 (stable) |
| Maintenance | Active (CI passing) |
| Binary Size | ~800KB |
| Compile Time | Fast |
| Learning Curve | 2/5 |
| SQLite Integration | 5/5 |

**Key Findings:**
- Pure synchronous I/O, thread-per-connection
- Supports HTTPS via rustls or openssl
- Request pipelining support
- Used as foundation by rouille (which adds higher-level routing)
- More control than rouille, less convenience

**Production Considerations:**
- No routing built-in (raw request/response)
- No middleware (you handle everything)
- Good for building custom servers with minimal overhead
- Thread pool with auto-cleanup (idle threads die after 5s)

---

### 3. Nucleus HTTP

**Status:** EMERGING (appears maintained, limited ecosystem)

| Metric | Value |
|--------|-------|
| Crates.io Version | 0.15.1 |
| Maintenance | Active (single maintainer PGIII) |
| Documentation | 6% documented (major concern) |
| Binary Size | Unknown (heavy deps: tokio, serde, rustls) |
| Compile Time | Medium |
| Learning Curve | 3/5 (unfamiliar patterns) |

**Key Findings:**
- Async runtime on tokio
- Modern feature set: cookies, routes, state, thread pools, virtual hosts
- Heavy dependency tree (29 dependencies in docs.rs listing)
- Limited community adoption
- **Not recommended** for production without more documentation

---

### 4. Axum + Rusqlite

**Status:** ACTIVE (Tokio team, production-grade)

| Metric | Value |
|--------|-------|
| GitHub Stars | 15k+ |
| Current Version | 0.8.x |
| Maintenance | Very Active |
| Binary Size | ~3-5MB (with full tokio) |
| Compile Time | Slow (heavy deps) |
| Learning Curve | 3/5 |
| SQLite Integration | 3/5 (needs spawn_blocking bridge) |

**The Async-Sync Bridge Problem:**

SQLite is synchronous. Axum is async. The bridge is `tokio::task::spawn_blocking`:

```rust
use axum::{Router, Extension};
use rusqlite::Connection;
use std::sync::Arc;

// Shared connection pool pattern
fn create_app() -> Router {
    let db = Arc::new(Connection::open("data.db").unwrap());

    Router::new()
        .route("/api/items", axum::routing::get(get_items))
        .layer(Extension(db))
}

async fn get_items(Extension(db): Extension<Arc<Connection>>) -> impl IntoResponse {
    // BRIDGE: Move sync DB call to blocking thread pool
    let items = tokio::task::spawn_blocking({
        let db = db.clone();
        move || {
            let mut stmt = db.prepare("SELECT * FROM items").unwrap();
            stmt.query_map([], |row| {
                Ok((row.get::<_, i64>(0), row.get::<_, String>(1)))
            }).unwrap().filter_map(|r| r.ok()).collect::<Vec<_>>()
        }
    }).await.unwrap();

    Json(items)
}
```

**Production Pattern for Connection Pooling:**

For higher concurrency, use `r2d2` or `sqlx`:

```rust
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

async fn create_pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(5)  // SQLite has connection limits
        .connect("sqlite://data.db").await.unwrap()
}
```

**Key Finding:** `spawn_blocking` has overhead. For high-throughput SQLite serving, the blocking bridge becomes the bottleneck. Consider `rusqlite` with `bundled` feature + connection pool.

---

### 5. Salvo

**Status:** ACTIVE (well-maintained, modern features)

| Metric | Value |
|--------|-------|
| Current Version | 0.89.1 |
| Maintenance | Active (monthly releases) |
| Binary Size | ~4MB |
| Compile Time | Medium |
| Learning Curve | 2/5 |
| SQLite Integration | 3/5 |

**Key Differentiators:**
- HTTP/3 support (QUIC)
- Auto-TLS via ACME (Let's Encrypt)
- Built-in OpenAPI generation
- Tree-based routing with middleware at any level
- `#[handler]` attribute (simple, no generics)

**Example:**
```rust
use salvo::prelude::*;

#[handler]
async fn hello() -> &'static str {
    "Hello World"
}

#[tokio::main]
async fn main() {
    let router = Router::new().get(hello);
    let acceptor = TcpListener::new("127.0.0.1:7878").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

**For SQLite:**
```rust
#[handler]
async fn get_items(depot: &Depot, res: &mut Response) {
    let db = depot.get::<SqlitePool>().unwrap();
    let items = sqlx::query!("SELECT * FROM items")
        .fetch_all(db)
        .await
        .unwrap();
    res.render(Json(items));
}
```

**Downside:** Less community mileage than Axum; middleware ecosystem smaller.

---

### 6. Poem

**Status:** ACTIVE (well-documented, async-focused)

| Metric | Value |
|--------|-------|
| Current Version | Active (requires Rust 1.85+) |
| Maintenance | Active |
| Binary Size | ~3-4MB |
| Compile Time | Medium |
| Learning Curve | 3/5 |
| SQLite Integration | 3/5 |

**Key Features:**
- `tower::Service` and `tower::Layer` compatible
- OpenAPI support via `poem-openapi`
- Feature-gated (enable only what you need)
- Safety: `#![forbid(unsafe_code)]`

**Example:**
```rust
use poem::{get, handler, listener::TcpListener, web::Path, Route, Server};

#[handler]
fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Route::new().at("/hello/:name", get(hello));
    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
```

---

## Binary Size Analysis

| Stack | Release Build | Stripped | Notes |
|-------|--------------|----------|-------|
| rouille + rusqlite | ~1.5MB | ~1.2MB | Sync, no tokio |
| tiny-http + rusqlite | ~1.0MB | ~800KB | Minimal sync |
| axum + tokio (full) | ~5-8MB | ~3-5MB | Async overhead |
| salvo + tokio | ~5-7MB | ~4-5MB | HTTP/3 adds size |
| poem + tokio | ~4-6MB | ~3-4MB | Feature-gated |

**Key Insight:** Sync stacks are ~3-4x smaller than async stacks due to no tokio dependency.

---

## MUSL Static Build Compatibility

| Framework | musl Support | Notes |
|-----------|-------------|-------|
| rouille | WORKS | Pure std, no issues |
| tiny-http | WORKS | Pure std |
| axum | WORKS | tokio supports musl |
| salvo | WORKS | Built on hyper/tokio |
| poem | WORKS | tokio-based |
| rusqlite | WORKS | `bundled` feature compiles with musl |

**Build Command:**
```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Build static
cargo build --release --target x86_64-unknown-linux-musl
```

**rusqlite with musl:** Use `features = ["bundled"]` to avoid system SQLite dependency. The bundled amalgamation compiles cleanly with musl.

---

## Compile Time Estimates

| Stack | Cold Build | Incremental |
|-------|------------|-------------|
| rouille + rusqlite | ~30s | ~5s |
| tiny-http + rusqlite | ~25s | ~4s |
| axum + tokio | ~3-5min | ~15-30s |
| salvo | ~2-3min | ~10-20s |
| poem | ~2-3min | ~10-20s |

**Key Insight:** Async frameworks have 5-10x longer cold builds due to tokio compilation.

---

## Production Readiness Checklist

| Criterion | rouille | tiny-http | axum | salvo | poem |
|-----------|---------|-----------|------|-------|------|
| Active Maintenance | 2/5 | 4/5 | 5/5 | 5/5 | 5/5 |
| Community Size | 1.2k stars | 600 stars | 15k stars | Growing | Stable |
| Production Usage | Limited | Moderate | High | Growing | Moderate |
| Async Support | No | No | Yes | Yes | Yes |
| HTTPS Support | No | Yes | Yes | Yes | Yes |
| Middleware | None | None | Tower | Built-in | Tower compat |
| Learning Curve | 1/5 | 2/5 | 3/5 | 2/5 | 3/5 |

---

## Recommendation Matrix

| Use Case | Recommended Stack |
|----------|-------------------|
| **Maximum simplicity** | rouille + rusqlite |
| **Binary size critical** | tiny-http + rusqlite |
| **Production REST API** | axum + sqlx |
| **Modern features (HTTP/3, auto-TLS)** | salvo |
| **Type-safe OpenAPI** | poem |
| **Single-file script** | rouille + rusqlite |

---

## SQLite Connection Patterns

### Single Connection (rouille/sync):
```rust
let db = Connection::open("app.db").unwrap();
// Share via closure capture or Arc<Mutex<Connection>>
```

### Connection Pool (async):
```rust
// sqlx approach (recommended)
let pool = SqlitePoolOptions::new()
    .max_connections(5)  // SQLite has 1 writer limit
    .connect("sqlite://app.db").await.unwrap();

// rusqlite + r2d2
let pool = r2d2::Pool::builder()
    .max_size(5)
    .build(rusqlite::Connection::open("app.db").unwrap()).unwrap();
```

**Critical Note:** SQLite has a single writer. Connection pools > 5 connections provide no benefit.

---

## Quick Start Templates

### Minimal Sync (rouille):
```toml
[dependencies]
rouille = "3.6"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Production Async (axum):
```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-native-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Modern with HTTP/3 (salvo):
```toml
[dependencies]
salvo = "0.89"
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-native-tls"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## Research Sources

- [Rouille GitHub](https://github.com/tomaka/rouille) (1.2k stars, FAQ on performance)
- [Tiny HTTP](https://crates.io/crates/tiny_http) (0.11, active CI)
- [Nucleus HTTP Docs](https://docs.rs/nucleus-http) (0.15.1, 6% documented)
- [Axum GitHub](https://github.com/tokio-rs/axum) (15k+ stars, Tokio team)
- [Salvo Crates](https://crates.io/crates/salvo) (0.89.1, active)
- [Poem Crates](https://crates.io/crates/poem) (active, Rust 1.85+)
- [Rust Web Framework Comparison](https://github.com/flosse/rust-web-framework-comparison) (comprehensive)
- [Aarambh Dev Hub Framework Review](https://aarambhdevhub.medium.com/rust-web-frameworks-in-2026) (Feb 2026)

---

## Next Steps

1. **For BDMP_DB specifically:** Evaluate whether async is required. If serving internal/local use, sync stack (rouille) provides maximum simplicity.

2. **If async required:** Use `axum + sqlx` with SQLite pool of 3-5 connections.

3. **If binary size matters:** Use `rouille + rusqlite` with bundled feature.

4. **If needing HTTP/3 or auto-TLS:** Use `salvo` despite smaller community.