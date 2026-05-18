# Multi-upstream Load Balancing, Request Caching, and Enhanced Analytics

## Goal

Extend DeepLX Monitor with three major features: (1) support multiple upstream DeepLX endpoints with load balancing and automatic failover, (2) cache translation results to reduce upstream calls for repeated text, and (3) provide richer analytics including per-language-pair stats, time-based heatmaps, error rate trends, and data export.

## Requirements

### Feature 1: Multi-upstream Load Balancing
- `config.toml` supports `[[upstream.endpoints]]` array, each with url + api_key + weight(optional)
- Round-robin distribution, mark unhealthy after 3 consecutive failures, skip unhealthy endpoints
- On failure, immediately retry next healthy endpoint (transparent to client)
- When all endpoints unhealthy: fallback to last-known-good endpoint
- Background probe every 60 seconds to recover unhealthy endpoints
- `TranslationBackend` trait abstraction for future engine extensibility
- Dashboard shows each endpoint's status (health/latency/success rate)

### Feature 2: Request Caching
- `TranslationCache` trait abstraction, MVP uses in-memory LRU implementation
- Key = SHA256(text + source_lang + target_lang)
- TTL and max entries both configurable (default 1h / 10,000 entries)
- Cache bypass via `X-No-Cache: true` request header
- Cache hits don't count as upstream requests in stats, but logged as cache_hit
- Dashboard shows cache hit rate and current size

### Feature 3: Enhanced Analytics
- Error rate trend chart (hourly/daily)
- Heatmap: hour × day-of-week + hour × date, tab switch between views
- Data export API: `GET /api/export?format=csv|json&start=&end=&lang=&status=`
- Export uses chunked streaming response to avoid OOM
- Frontend export button reuses current filter conditions

## Acceptance Criteria

- [ ] Can configure 2+ upstream endpoints and requests are distributed via round-robin
- [ ] If one upstream is down, requests automatically route to healthy ones
- [ ] All endpoints down → fallback to last-known-good endpoint
- [ ] Background probe recovers unhealthy endpoints within 60s
- [ ] Repeated identical translations served from cache within TTL
- [ ] Cache stats (hit rate, size) visible in dashboard
- [ ] Cache bypass works via X-No-Cache header
- [ ] Error rate trend chart available in dashboard
- [ ] Heatmap visualization shows activity patterns (both views)
- [ ] Can export filtered logs as CSV/JSON via API endpoint
- [ ] Export handles large datasets without OOM (streaming)
- [ ] Health check tests all upstream endpoints

## Definition of Done

- Tests added/updated (unit/integration where appropriate)
- `cargo build --release` succeeds
- Frontend builds without errors (`npm run build`)
- Docs/config examples updated (config.toml.example, README)
- Docker build works

## Decision (ADR-lite)

**Context**: Need to choose load balancing strategy, cache implementation, and analytics scope.

**Decisions**:
1. Load balancing: Round-robin + failover (3 failures → unhealthy, 60s probe recovery)
2. Failover: Immediate retry to next healthy endpoint; all-down fallback to last-known-good
3. Cache: SHA256 key, configurable TTL (default 1h) and max entries (default 10,000), LRU eviction
4. Export: Filtered export (date range, language, status), CSV/JSON, streaming response
5. Heatmap: Both views (hour×weekday + hour×date), tab switch
6. Architecture: Trait abstractions for upstream (`TranslationBackend`) and cache (`TranslationCache`)

**Consequences**: Trait abstractions add slight complexity but enable future Redis cache and multi-engine support without refactoring. Streaming export adds complexity but prevents OOM on large datasets.

## Design Decisions (from diverge)

### 预留扩展点
- 上游用 trait 抽象（`TranslationBackend` trait），方便未来加不同翻译引擎（Google、OpenAI）
- 缓存用 trait 抽象（`TranslationCache` trait），MVP 用内存 LRU 实现，未来可换 Redis
- 按语言对路由到不同上游：MVP 不实现，但数据结构预留

### 增强健壮性
- 所有上游不健康时：仍尝试最近一次成功过的端点（last-known-good fallback）
- 导出用流式响应（chunked transfer）避免大数据量 OOM
- 健康检查扩展为检测所有上游端点，返回每个端点的状态

## Out of Scope (explicit)

- 不同翻译引擎类型（Google、OpenAI）— 仅预留 trait 接口
- 持久化缓存（重启后缓存清空）
- 分布式缓存（Redis）
- 按语言对路由到不同上游
- 前端动态增删上游端点（MVP 通过 config.toml 配置，热更新支持修改已有端点参数）

## Technical Approach

### New files
- `src/upstream.rs`: `TranslationBackend` trait + `LoadBalancer` struct (round-robin, health tracking, failover, probe task)
- `src/cache.rs`: `TranslationCache` trait + `LruTranslationCache` impl (based on `moka` crate)

### Modified files
- `config.rs`: Add `[[upstream.endpoints]]` array and `[cache]` section
- `proxy.rs`: Call LoadBalancer instead of direct single-URL request; check cache before upstream
- `state.rs`: Add `Arc<LoadBalancer>` and `Arc<dyn TranslationCache>` to AppState
- `api.rs`: Add `/api/export`, `/api/cache/stats`, `/api/upstream/status`, `/api/analytics/heatmap`, `/api/analytics/error-trend`
- `db.rs`: Add heatmap queries (hour×weekday, hour×date) and error rate trend query
- `main.rs`: Initialize LoadBalancer, spawn probe task, register new routes
- Frontend: upstream status panel, cache stats card, heatmap component, error trend chart, export button

### Dependencies
- `moka` (async LRU cache with TTL)
- `sha2` (SHA256 hashing for cache keys)
- `csv` (CSV export serialization)

## Implementation Plan

- PR1: Multi-upstream + load balancing + failover (backend: upstream.rs, config changes, proxy changes)
- PR2: Request caching (backend: cache.rs, proxy integration, cache stats API)
- PR3: Enhanced analytics API + export (backend: new DB queries, export endpoint)
- PR4: Frontend — upstream status panel + cache stats + heatmap + error trend + export button

## Technical Notes

- SQLite single-writer (Mutex<Connection>) — cache is in-memory, no DB involvement
- `rust-embed` requires `dist/` at compile time — frontend changes need `npm run build` first
- Config is TOML-based, hot-reloadable via `POST /api/config`
- Existing `get_lang_hourly_stats()` and `get_lang_daily_stats()` in db.rs can be extended for heatmap
- `moka` supports async and has built-in TTL + max capacity — ideal for our use case
