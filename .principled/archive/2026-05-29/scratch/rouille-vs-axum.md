# rouille vs axum Decision — Phase 2 API

**Date:** 2026-05-26
**Status:** DECIDED — RECOMMEND axum

---

## The Use Case Profile

BDPM Phase 2 is a **read-only SQLite API** over ~15K drugs with these endpoints:

| Endpoint | Workload |
|----------|----------|
| `GET /drugs?q=` | FTS5 full-text search, 15K rows |
| `GET /drugs/:cis` | Single-row detail + JOINs |
| `GET /generic-groups` | Simple browse, small result set |
| `GET /atc/:code` | Hierarchy traversal |
| `GET /availability` | Dispo table scan |
| `GET /health` | Trivial health check |
| `GET /openapi.yaml` | Static file |

All operations are **reads only**. SQLite is the database. No streaming, no websockets, no HTTP/2 requirement. The dataset is moderate (~15K drugs, estimated <100MB DB).

---

## rouille Verdict

**Can rouille handle these requirements? YES — but with caveats.**

### What works

- All 6 endpoint types fit comfortably within rouille's capabilities
- Static file serving for OpenAPI YAML is trivial
- WebSocket module exists in rouille (not needed here, but confirms feature scope)
- `start_server_with_pool` provides thread pool sizing
- Gzip compression, routing macro, JSON input handling — all present
- Last commit: 2025-06-17 (multipart vendoring, preparing republish) — not dead
- Benchmark: ~22k req/sec on hello-world (vs nginx ~39k, hyper ~53k)

### What fails at our scale

**Thread-per-request model under SQLite load:**

rouille's FAQ acknowledges it: *"each request is handled in its own dedicated thread."* With a thread pool of N workers, if SQLite query takes 50ms (typical FTS5 over 15K rows), and 20 concurrent requests arrive, workers block. New requests queue or get rejected.

For a read-only public API, this is manageable if concurrency stays low. But rouille provides no mechanism to shed load gracefully. Under load spikes (e.g., morning pharmacy lookup rush), the thread pool saturates and new connections wait or timeout.

**No HTTP/2:** Irrelevant for our use case — clients will use HTTP/1.1.

**No async ecosystem integration:** rouille has no tower, no tracing middleware integration, no way to plug into the broader Rust async web ecosystem. This matters for observability and future extensibility.

### Maintenance risk

- 1,233 stars, single maintainer (tomaka)
- 66 open issues, sparse commits (months between activity)
- The 2025-06-17 commit suggests the project is being revived for a republish
- But: rouille FAQ explicitly says "once async I/O is figured out, rouille will be updated" — this was written ~2017. Async I/O is figured out. rouille hasn't been updated to use it.
- **One year from now:** Possible the maintainer loses interest again. Migration path to axum is straightforward (handler function signature changes only).

---

## axum Verdict

**Is axum better for our use case? YES.**

### Pros

| Factor | Detail |
|--------|--------|
| Active maintenance | 26,046 stars, 1,405 forks, commits every few days (last: 2026-05-22) |
| tokio runtime | Non-blocking I/O, can handle thousands of concurrent connections with few threads |
| `spawn_blocking` | SQLite calls (`rusqlite`) run in blocking thread pool without blocking async workers |
| Ecosystem | tower middleware (tracing, timeouts, compression, rate limiting), integrates with hyper and tonic |
| Performance | Comparable to hyper (~53k+ req/sec), thin layer on top |
| MSRV 1.80 | Modern, stable |
| OpenAPI support | `utoipa` integrates cleanly with axum extractors |

### Cons

| Factor | Detail |
|--------|--------|
| Async runtime required | `#[tokio::main]` mandatory — adds compile time, binary size |
| Binary size impact | Real but acceptable: axum binary ~2-4MB larger than rouille. Not relevant for a server deployment. |
| Handler complexity | `async fn` signature required, `?` for errors — marginally more boilerplate than rouille's sync closure |
| **Does NOT force full-async** | SQLite (rusqlite) calls are still synchronous. Wrap in `tokio::task::spawn_blocking()` and the async runtime handles them fine. The sync import pipeline is unaffected. |

### The "async everywhere" fear is overblown

The common objection: *"adding tokio forces everything to be async."* This is wrong. The correct pattern in axum is:

```rust
async fn get_drug(State(db): State<DbPool>) -> Json<Drug> {
    // rusqlite is sync — spawn it to blocking thread pool
    let drug = tokio::task::spawn_blocking(move || {
        db.query_drug(cis)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(drug))
}
```

The DB queries stay synchronous. The HTTP layer stays async. The thread pool handles blocking calls without starving async workers.

---

## The Decision

**RECOMMEND: axum**

### Rationale

1. **Resource efficiency under load.** For a public API, load spikes are unpredictable. axum's async runtime handles 1,000 concurrent connections with 4 threads. rouille's thread pool would need 1,000 threads for the same concurrency.

2. **Maintenance trajectory.** rouille's revival in mid-2025 is encouraging but the project lacks community depth. axum is the Rust web standard — backed by tokio team, integrated with tower ecosystem, used in production by thousands.

3. **No blocking of Phase 1 work.** The import pipeline (rusqlite, sync) remains unchanged. Only the API server layer becomes async.

4. **Observability.** axum + tower-tracing gives structured logging, request tracing, and middleware composition out of the box. rouille requires manual wiring.

5. **Future-proof for Phase 3.** Phase 3 has background sync tasks. axum's async runtime can host these naturally. rouille has no story for background tasks.

### Counter-argument (rouille's case)

*"It's just a read-only drug lookup API. rouille is simpler, smaller, and fits."*

This is valid. For a private API with predictable low traffic, rouille is fine. But we don't know the usage pattern, and BDPM is a public dataset — it could gain unexpected traffic. The cost of axum is compile time and minor binary size. The cost of rouille under load is harder to recover from.

---

## If rouille Were Chosen

**Migration path to axum:** Trivial. The API is thin (6 endpoints, simple JSON responses). The handler functions are stateless. The only change is the function signature from `move |request| -> Response` to `async fn handler() -> Json<T>`. The DB layer is identical.

**One year from now:** Uncertain. The republish in 2025 is positive signal, but the project needs multiple active maintainers before it can be considered stable long-term.

---

## API Design Implication

With axum, Phase 2 structure:

```
src/
  api/
    mod.rs          # Router assembly
    routes/
      drugs.rs       # FTS5 + detail endpoints
      browse.rs     # generic_groups, atc
      dispo.rs       # availability
      health.rs      # health + openapi
    extractors.rs    # State, Path, Query extractors
  db/
    mod.rs           # DbPool (rusqlite wrapper)
    queries/
      drugs.rs       # rusqlite queries (sync, wrapped in spawn_blocking)
      browse.rs
      dispo.rs
  import/           # Phase 1 — unchanged (sync rusqlite)
```

**No changes to import pipeline.** `rusqlite` stays synchronous. The `api` module owns the async layer.

**Key insight:** This is the standard Rust pattern. You see it in sqlx examples, in axum SQL examples, and in production services. The async/Tokio runtime is an I/O coordinator — it doesn't mean your database layer must be async. `rusqlite` + `tokio::task::spawn_blocking` is a valid, tested combination used in production.

---

## Summary

| Criterion | rouille | axum | Winner |
|-----------|---------|------|--------|
| Fits API surface | Yes | Yes | Tie |
| Concurrency model | Thread-per-request (limited) | Async non-blocking (scalable) | **axum** |
| Maintenance | Single maintainer, sparse commits | tokio team, daily commits | **axum** |
| Ecosystem | Minimal | tower ecosystem, rich middleware | **axum** |
| Binary size | Small | Moderate | **rouille** |
| Compile time | Fast | Slower | **rouille** |
| Future-proof | Uncertain | Strong | **axum** |
| Migration complexity | N/A | Low (stateless handlers) | Tie |

**Decision: axum.** The resource efficiency and maintenance trajectory outweigh rouille's simplicity advantage for a public API over a real database.