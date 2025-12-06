# axum-ratekeeper

Production-ready, Redis-backed rate limiting service built with Axum/Tokio. Supports multi-tenant policies, token bucket and GCRA algorithms, admin-managed policy updates, per-tenant API keys, Prometheus metrics, and optional TLS.

## Features
- Token Bucket (burst-friendly) and GCRA (sliding-window-like) algorithms per policy.
- Multi-tenant policies stored in Redis; Moka cache to reduce config lookups.
- Admin-protected policy write API; per-tenant API key enforcement on reads/checks.
- Single-roundtrip Lua scripts for atomic rate decisions; TTL-based bucket cleanup.
- Optional TLS listener (Rustls) and structured logging.
- Basic Redis retry/backoff for transient failures; Prometheus metrics.

## Requirements
- Rust toolchain (1.75+ recommended)
- Redis server (local or remote)
- (Optional) TLS cert/key files for HTTPS
- (Optional) `wrk` for load testing

## Configuration (env vars)
- `REDIS_URL` (default `redis://127.0.0.1/`)
- `BIND_ADDR` (default `0.0.0.0:8080`)
- `POLICY_CACHE_TTL_SECS` (default `30`)
- `ADMIN_API_KEY` (required for policy writes)
- `TLS_CERT_PATH`, `TLS_KEY_PATH` (enable TLS when both set)
- `REDIS_MAX_RETRIES` (default `3`)
- `REDIS_BACKOFF_MS` (default `50`)

## Quickstart
```bash
brew services start redis   # or your preferred Redis

ADMIN_API_KEY=admin123 RUST_LOG=info cargo run
```

Seed a tenant policy + tenant API key (admin only):
```bash
curl -X PUT http://127.0.0.1:8080/v1/tenants/t123/policies/default \
  -H "Content-Type: application/json" \
  -H "x-admin-api-key: admin123" \
  -d '{"capacity":5,"refill_tokens_per_sec":1,"algorithm":"token_bucket","api_key":"tenantkey123"}'
```

Check a request (tenant key required):
```bash
curl -X POST http://127.0.0.1:8080/v1/check \
  -H "Content-Type: application/json" \
  -H "x-api-key: tenantkey123" \
  -d '{"tenant_id":"t123","identifier":"user:42","policy_key":"default","cost":1}'
```

Fetch a policy (tenant key or admin key):
```bash
curl http://127.0.0.1:8080/v1/tenants/t123/policies/default \
  -H "x-api-key: tenantkey123"
```

Health/metrics:
- Liveness/Readiness: `/health/live`, `/health/ready`
- Prometheus: `/metrics`

## API Surface
- `POST /v1/check` — enforce rate limit; body includes `tenant_id`, `identifier`, `policy_key`, `cost`. Requires `x-api-key`.
- `GET /v1/tenants/:tenant_id/policies/:policy_key` — read policy. Requires `x-api-key` (tenant) or `x-admin-api-key`.
- `PUT /v1/tenants/:tenant_id/policies/:policy_key` — upsert policy and optional tenant API key. Requires `x-admin-api-key`.

Auth headers:
- Tenant calls: `x-api-key`
- Admin calls: `x-admin-api-key`

## Redis Layout
- Tenant policy hash: `rl:tenant:{tenant_id}`
  - `policy:{policy_key}` → JSON { capacity, refill_tokens_per_sec, algorithm }
  - `api_key` → tenant API key
- Token bucket bucket hash: `rl:bucket:{tenant_id}:{policy_key}:{identifier}` fields `tokens`, `last_refill` (with TTL)
- GCRA key: `rl:gcra:{tenant_id}:{policy_key}:{identifier}` stores TAT with PX TTL

## Algorithms
- Token Bucket: burst-friendly; TTL ~ `max(60s, 2 * capacity/rate)`.
- GCRA: smooth rate; TTL similar window; suitable for stricter smoothing.
Select algorithm per policy (`algorithm: "token_bucket"` or `"gcra"`).

## TLS
Provide `TLS_CERT_PATH` and `TLS_KEY_PATH`; server binds HTTPS on `BIND_ADDR`. Plain HTTP is used when unset.

## Resilience
- Redis operations retried for transient errors (`REDIS_MAX_RETRIES`, `REDIS_BACKOFF_MS`).
- Buckets use TTL for automatic cleanup.
- Cached policies/API keys reduce Redis pressure; caches invalidated on writes.

## Benchmark (local)
- Command: `wrk -t2 -c50 -d10s --latency -s /tmp/wrk_check.lua http://127.0.0.1:8080/v1/check`
- Payload: token bucket policy (capacity 5, rate 1), valid tenant API key.
- Result (Apple Silicon laptop, localhost): ~61k req/s, p50 ~0.79ms, p99 ~1.07ms.

## Usefulness
- Drop-in HTTP service for multi-tenant rate limits with low latency and single-RTT Redis Lua execution.
- Per-tenant API keys and admin-protected policy writes support SaaS-style isolation.
- Supports bursty (token bucket) and smoother (GCRA) behaviors without code changes in callers.
- Metrics/health endpoints ready for scraping and container orchestration.

## Development
```bash
cargo fmt
cargo check
RUST_LOG=debug cargo run
```

## Next Steps (good first contributions)
1. Add configurable Redis timeouts and a circuit breaker/backoff policy with metrics.
2. Provide policy versioning/list APIs and bulk import/export tooling.
3. Add OpenAPI/Swagger docs and typed client stubs.
4. Add integration tests with testcontainers or embedded Redis, plus load-test harness scripts.
5. Support per-route limits or hierarchical limits (global + per-identifier).
6. Add HMAC-signed tokens or mTLS for auth; rotate tenant keys API.
7. Add structured tracing context propagation (trace/span IDs) and richer logs.
8. Package container image + Helm chart; enable horizontal autoscaling guidance.
9. Expose optional GCRA parameters (burst tolerance) and richer responses (limit metadata).
10. Add graceful degradation mode (fail-open/closed toggle) on Redis outages.

