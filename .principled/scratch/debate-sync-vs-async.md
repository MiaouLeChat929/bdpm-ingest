# BDMP Project: Sync vs Async I/O — Architectural Debate

**Date:** 2026-05-26
**Status:** Resolved
**Verdict:** SYNC wins — concrete recommendation below

---

## Research Summary

### What the web says about rusqlite + async

- **Rusqlite 0.38.0** (Dec 2025) is the definitive SQLite wrapper for Rust. It's sync-only by design. The pattern for using rusqlite with async frameworks is: `rusqlite calls happen inside tokio::spawn_blocking` to avoid blocking the async executor. This is not a hack — it's the intended pattern.

- The LogRocket 2025 Rust web frameworks article notes Rouille explicitly as a "microweb framework" for simple apps — ~1.1k stars, no ORM, no batteries. Tiny-http is similarly minimal.

- The Aarambh Dev Hub 2026 ORM comparison (Feb 2026) verdict: **"Is it SQLite? → Rusqlite. Done. Don't overthink it."** Direct quote from the article that compares Diesel, SQLx, SeaORM, and Rusqlite across 4 production projects.

- Axum's ecosystem (tower middleware, tower-sessions-rusqlite-store, etc.) does support rusqlite as a session backend, proving that rusqlite integration into async stacks is a solved problem — but that doesn't mean you need async for it.

### The async tax reality

- Tokio spawn_blocking has ~microsecond overhead. For a read-only API serving 15K records, this is completely negligible. But it adds cognitive complexity: every DB call becomes `tokio::task::spawn_blocking(|| db_query(...))`.

- Async fn in traits is now stable in Rust (1.75+), removing one of the historical friction points. But rusqlite won't suddenly become async.

### The sync landscape in 2026

- Rouille: 1.1k stars, no async, minimal API surface. Still maintained as of 2026. Simple enough that a solo dev can understand it in an afternoon.

- Tiny-http: lighter, async-capable variants exist but the sync API is straightforward. Good for low-level HTTP needs.

- Tide: uses async-std, 5k stars, but async-std is effectively unmaintained as of 2025 (no major releases). Not a good bet.

- The "2026 standard" is indeed axum/tokio for async Rust web. But standards exist to serve use cases — and this use case is a read-only SQLite API.

---

## CASE FOR SYNC (Rouille / Tiny-http)

### Arguments

1. **SQLite is fundamentally synchronous.** rusqlite uses file locks, not epoll/kqueue. Any async wrapper around it is a thread pool that blocks. There's no native async SQLite path.

2. **You pay the async tax for no benefit.** The "async benefit" for HTTP servers is handling slow clients, keep-alive connections, and concurrent I/O. But:
   - Read-only API with local SQLite: no network I/O to wait on — the DB query blocks, not the network.
   - Slow clients are irrelevant for a JSON API with small payloads.
   - Keep-alive helps but doesn't justify the entire async stack.

3. **Compile times are measurably faster.** Tokio + axum pulls in ~15+ crates (tokio, hyper, tower, axum, etc.). Rouille is essentially hyper + a thin wrapper (~3K LOC). `cargo check` times drop significantly.

4. **Binary size shrinks.** A musl static binary with tokio includes the full async runtime. Rouille produces smaller binaries.

5. **Simpler debugging.** Stack traces through async code are harder to follow. Sync code is linear.

6. **No `spawn_blocking` bridges needed.** Direct `rusqlite::Connection` in a thread-per-request model (or even single-threaded with a Mutex) is simpler than wrapping every DB call.

7. **CI build times improve.** Fewer dependencies = faster GitHub Actions builds. For a solo dev, this matters.

8. **Rouille is production-proven for this exact pattern.** Look at the github.com/patte/tower-sessions-rusqlite-store repo — it uses rusqlite as the backend for async tower-sessions. The bridge works. But you don't need the bridge if you stay sync.

### Concrete example

```rust
// SYNC (Rouille)
fn handler(_req: &Request, db: &Connection) -> Response {
    let drugs: Vec<Drug> = db_query(db).unwrap();
    Response::json(&drugs)
}

// ASYNC (Axum + spawn_blocking)
async fn handler(State(db): State<DbPool>) -> Json<Value> {
    tokio::task::spawn_blocking(|| {
        let drugs: Vec<Drug> = db_query(&db);
        drugs
    }).await.unwrap()
}
```

The async version adds a closure, an await, an unwrap, and a thread handoff — for functionally identical behavior.

### When sync wins

- CLI tools
- Embedded systems
- Single-purpose servers with local I/O
- Projects where compile time matters
- Solo developers who value simplicity over ecosystem breadth

---

## CASE FOR ASYNC (Tokio + Axum)

### Arguments

1. **The ecosystem is the 2026 standard.** 18k+ GitHub stars. Actively maintained by the Tokio team. Tower middleware ecosystem. Better docs, more Stack Overflow answers, more blog posts.

2. **Tower middleware is only available for async.** If you ever want: rate limiting, request tracing, compression, CORS handling — tower provides battle-tested implementations. Rouille has no equivalent.

3. **Future-proofing for extensibility.** If you later need:
   - HTTP proxy to another service
   - Redis caching layer
   - WebSocket updates
   - Background job processing
   
   You'll need async anyway. Starting sync means a full rewrite later.

4. **Better concurrent request handling.** SQLite serializes writes via file locks, but concurrent reads can be served in parallel. Async allows multiple DB reads to be processed while one waits on the file system.

5. **Health checks and graceful shutdown.** Tokio's cancellation model and graceful shutdown support is mature.

6. **The "async tax" is smaller than it was.** With async fn in traits stable and `#[tokio::main]` ergonomic, the overhead is minimal compared to 2022-era async Rust.

7. **Axum's error handling is better.** `Result<T, E>` propagation with `?` operator works naturally. Rouille's error handling requires more manual work.

### Concrete example

```rust
// ASYNC (Axum) — future-proof for middleware
let app = Router::new()
    .route("/drugs", get(list_drugs))
    .layer(TraceLayer::new_for_http())
    .layer(RateLimitLayer::fixed(100))
    .layer(CorsLayer::permissive());

// SYNC (Rouille) — middleware requires manual implementation
// No tower, no tracing, no standard patterns
```

### When async wins

- Complex APIs with middleware needs
- Projects that will scale or add features
- Teams that value ecosystem over minimalism
- Production services that need observability (tracing, metrics)

---

## VERDICT

### Decision: **SYNC wins for this specific project**

#### Rationale

The project characteristics that determine this:

| Factor | Value | Impact |
|--------|-------|--------|
| Database | SQLite (sync) | Forces sync bridge in async path anyway |
| API type | Read-only | No WebSocket, no streaming, no long-polling |
| Scale | ~15K records | Trivial load — no concurrency benefits needed |
| Team size | Solo dev | Simplicity > ecosystem breadth |
| Target | GitHub Actions, musl static | Smaller binary + faster CI matters |
| Future scope | Unknown but likely small | No compelling feature need for async |

The async stack would add: more dependencies, larger binary, compile time overhead, `spawn_blocking` boilerplate for every DB call — all for zero practical benefit given that SQLite is the bottleneck anyway.

SQLite queries are CPU/IO-bound. They block. Async doesn't make them faster. It just adds a thread handoff layer on top.

#### Concrete Stack Recommendation

**Primary:** `rouille` + `rusqlite` (with `bundled` feature)

**Fallback option:** If you want slightly more structure, `tiny-http` is also fine — but rouille's API is more ergonomic for JSON APIs.

**Do NOT use:** sqlx with SQLite (fake async, extra dependency, no compile-time query checking without a live DB), actix-web (overkill), Rocket (async but heavier).

#### Implementation sketch

```toml
# Cargo.toml
[dependencies]
rusqlite = { version = "0.38", features = ["bundled"] }
rouille = "3.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
// main.rs — rough structure
use rouille::{Request, Response};
use rusqlite::Connection;

fn main() {
    let db = Connection::open("bdmp.db").expect("DB open");
    rouille::start_server("0.0.0.0:8080", move |req| {
        router(req, &db)
    });
}

fn router(req: &Request, db: &Connection) -> Response {
    match (req.method(), req.url().as_str()) {
        ("GET", "/drugs") => list_drugs(db),
        ("GET", "/drugs/") if !req.url().contains('?') => {
            let id = req.url().trim_end_matches('/').split('/').last();
            get_drug(db, id)
        }
        _ => Response::empty_404(),
    }
}
```

#### Trade-offs of the chosen approach (SYNC)

| Trade-off | Impact | Mitigation |
|-----------|--------|------------|
| No tower middleware ecosystem | Must implement CORS, rate limiting manually | Lightweight — do it yourself or use a single crate |
| Smaller community | Fewer Stack Overflow answers | Project is simple; code is self-explanatory |
| Less future-proof | Harder to add WebSocket/HTTP proxy later | Rewrite is acceptable for a small project |
| Manual connection management | No connection pooling abstraction | Single connection with Mutex is fine for 15K records |
| Less "resume after I/O" benefit | Blocking thread while SQLite queries | Acceptable — SQLite is the bottleneck anyway |

#### Why not ACTIX WEB (the other main async contender)

Actix Web is the performance leader with 21k+ stars, but:
- Steeper learning curve than Axum
- Heavier dependency tree
- For a read-only SQLite API, the performance difference is irrelevant
- You don't need actor-model semantics for this use case

#### Final recommendation summary

```
If you care about:  → Use this:
─────────────────────────────────────────────────────
Compile time         → SYNC (rouille) ✅
Binary size          → SYNC (rouille) ✅
Simplicity           → SYNC (rouille) ✅
CI build speed       → SYNC (rouille) ✅
Future extensibility  → ASYNC (axum) — but you don't need it yet
Middleware ecosystem → ASYNC (axum) — but you don't need it yet
Production credibility → Both — both are proven

Verdict: SYNC. Keep it simple.
```

---

## Appendix: Key sources consulted

- Aarambh Dev Hub: "Rust ORMs in 2026: Diesel vs SQLx vs SeaORM vs Rusqlite" (Feb 2026) — rusqlite verdict
- LogRocket: "Exploring the top Rust web frameworks" (May 2025, updated) — framework landscape
- Level Up Coding: "Rust/Axum: User Session Management" (Jul 2025) — axum + rusqlite bridge pattern
- tower-sessions-rusqlite-store (GitHub) — production example of rusqlite + async integration
- Rust Users Forum: "Recommend alternatives to Axum" (Aug 2025) — compile time complaints