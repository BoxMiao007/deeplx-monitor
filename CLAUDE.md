# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

DeepLX Monitor — a translation proxy and monitoring dashboard for DeepLX API. Supports multi-upstream load balancing, translation caching, config hot-reload, and a Vue 3 dashboard with usage stats, charts, and health monitoring.

## Tech Stack

- **Backend**: Rust (Axum + Tokio), SQLite via `rusqlite`, static asset embedding via `rust-embed`, moka (LRU cache), notify (file watcher)
- **Frontend**: Vue 3 + TypeScript + Vite, Pinia for state, Chart.js for charts (single-page dashboard with settings drawer)
- **Build output**: Frontend builds to `../dist/` (project root `dist/`), Rust binary embeds `dist/` at compile time

## Commands

```bash
# Frontend (run from frontend/)
cd frontend
npm install        # install deps
npm run dev        # dev server on :5173, proxies /api to :5555
npm run build      # builds to ../dist/

# Backend (run from project root)
cargo build        # compile
cargo run          # start server on 127.0.0.1:5555
cargo build --release
```

The frontend dev server proxies `/api/*` and `/translate` to `localhost:5555` (configured in `frontend/vite.config.ts`).

## Architecture

```
Client ──POST /translate──► proxy.rs ──► LoadBalancer ──► upstream endpoints
                                │              │
                                ▼ cache hit?   ▼ (round-robin + failover)
                           cache.rs        upstream.rs
                                │
                                ▼ (logs to SQLite)
         ◄──GET /api/*───── api.rs ◄── db.rs
                                          │
         ◄──SPA /────────── main.rs ◄── dist/ (embedded)
```

- **`main.rs`**: Axum router setup, SPA/asset handlers, background tasks (log cleanup, endpoint probing, config file watcher). Listens on address from `config.toml` `[proxy]` section.
- **`proxy.rs`**: `/translate` handler — checks cache, selects upstream via LoadBalancer, forwards request, measures latency, logs result to DB.
- **`api.rs`**: All `/api/*` endpoints — stats, charts, request logs, health check, full config CRUD, upstream status, cache stats, analytics (heatmap, error trend), data export.
- **`db.rs`**: SQLite wrapper with `Mutex<Connection>`. Tables: `translation_logs` (per-request rows, stats computed in real-time) and `stats_anchor` (cache hit/miss counters only).
- **`state.rs`**: `AppState` holds `Arc<RwLock<Config>>`, `Arc<Database>`, `Arc<RwLock<HealthStatus>>`, `Arc<LoadBalancer>`, `Arc<TranslationCache>`, `reqwest::Client`.
- **`config.rs`**: TOML config with sections: `[upstream]`, `[proxy]`, `[monitor]`, `[health_check]`, `[cache]`, `[demo]`. Includes `watch_config_file()` for hot-reload via `notify`.
- **`upstream.rs`**: Multi-endpoint load balancer (round-robin + failover). Supports runtime `reload()` when config changes.
- **`cache.rs`**: Translation cache (moka LRU + TTL). Supports runtime `reload()` when config changes.
- **`utils.rs`**: Shared utilities (`chrono_now()`).

## Key Patterns

- **Frontend build is a prerequisite for backend**: `dist/` must exist before `cargo build` because `rust-embed` embeds it at compile time. Always `npm run build` first.
- **No ORM**: Raw SQL via `rusqlite`. Schema migrations are done inline in `db.rs:init()` with `ALTER TABLE` guards.
- **Config hot-reload**: All config changes take effect at runtime (~1s delay). Three paths: (1) file watcher detects `config.toml` changes, (2) `POST /api/config` from Web UI, (3) both sync LoadBalancer and Cache. Only `proxy.host`/`proxy.port` require restart.
- **Runtime-reactive components**: LoadBalancer and TranslationCache both have `reload()` methods called on config change. Background tasks (probe, cleanup) read config dynamically each iteration.
- **Health tracking**: Updated on every `/translate` call (implicit) and via `POST /api/health/check` (explicit test translation). Shown in upstream card.

## Docker

```bash
docker compose up -d        # build and run
docker compose build        # rebuild only
```

The `Dockerfile` uses multi-stage build: Node.js for frontend, Rust for backend. Mount `config.toml` as a volume.

## Git Status Note

`config.toml` is in `.gitignore` — use `config.toml.example` as template. `node_modules/` is also gitignored.

---

## Behavioral Guidelines

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

### 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

### 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

---

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->

---

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **deeplx-monitor** (832 symbols, 1355 relationships, 37 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/deeplx-monitor/context` | Codebase overview, check index freshness |
| `gitnexus://repo/deeplx-monitor/clusters` | All functional areas |
| `gitnexus://repo/deeplx-monitor/processes` | All execution flows |
| `gitnexus://repo/deeplx-monitor/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
