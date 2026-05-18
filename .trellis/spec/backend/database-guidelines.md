# Database Guidelines

> Database patterns and conventions for this project.

---

## Overview

- **Library**: `rusqlite` with `std::sync::Mutex<Connection>` (single-writer serialization)
- **No ORM**: Raw SQL with `prepare_cached()` for hot-path queries
- **Schema management**: Inline in `db.rs:init()` with `ALTER TABLE` guards (no migration framework)
- **PRAGMAs**: WAL mode, NORMAL sync, 5s busy timeout, 8MB cache

---

## Schema

### `translation_logs` — 请求日志

Primary data table. Stats are computed in real-time from this table.

```sql
CREATE TABLE translation_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chars INTEGER NOT NULL,
    source_lang TEXT NOT NULL,
    target_lang TEXT NOT NULL,
    source_chars INTEGER NOT NULL DEFAULT chars,
    target_chars INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL,          -- 'success' | 'error'
    error_msg TEXT,
    created_at TEXT DEFAULT (datetime('now', 'localtime'))
)
```

Indexes: `idx_created_at`, `idx_source_lang_created`, `idx_target_lang_created`

### `stats_anchor` — 缓存统计持久化

Only used for persisting cache hit/miss counters across restarts. The `total_requests`/`total_chars`/`anchor_date` columns are **dead** (kept to avoid SQLite rebuild migration).

```sql
-- Only these columns are actively read/written:
cache_hits INTEGER NOT NULL DEFAULT 0
cache_misses INTEGER NOT NULL DEFAULT 0
```

---

## Design Decisions

### Real-time Stats (not cumulative counters)

**Context**: Previously, `stats_anchor` accumulated total_requests/total_chars, and cleanup would roll up deleted rows before deletion. This created coupling between retention and stats.

**Decision**: Stats are computed in real-time via `SELECT COUNT(*), SUM(chars) FROM translation_logs`. The table is bounded by `max_log_entries` (default 10000), keeping queries fast.

**Why**:
- Simpler code (no transaction needed for log inserts)
- No data drift between anchor and actual logs
- Retention strategy is independent of stats calculation

### Count-based Retention (not time-based)

**Context**: Previously used `retention_days` to delete logs older than N days.

**Decision**: Use `max_log_entries` to keep only the N most recent entries.

**Why**:
- Predictable storage usage regardless of traffic patterns
- Simpler mental model: "keep last 10000 entries"
- No time-zone edge cases

---

## Query Patterns

### Log insertion — single INSERT, no transaction

```rust
pub fn log_translation(...) -> SqliteResult<i64> {
    let conn = self.conn.lock().unwrap();
    conn.execute("INSERT INTO translation_logs ...", params![...])?;
    Ok(conn.last_insert_rowid())
}
```

### Cleanup — delete oldest by count

```rust
pub fn cleanup_old_logs(&self, max_entries: u32) -> SqliteResult<i64> {
    let total_count = /* SELECT COUNT(*) */;
    let to_delete = total_count - max_entries as i64;
    if to_delete <= 0 { return Ok(0); }
    conn.execute(
        "DELETE FROM translation_logs WHERE id IN (SELECT id FROM translation_logs ORDER BY id ASC LIMIT ?1)",
        params![to_delete],
    )?;
    Ok(to_delete)
}
```

### Use `prepare_cached()` for repeated queries

All hot-path queries use `conn.prepare_cached()` to avoid re-parsing SQL on every call.

---

## Migrations

No migration framework. Schema changes are done inline in `db.rs:init()`:

```rust
// 检查列是否存在，不存在则添加
let cols: Vec<String> = stmt.query_map([], |row| row.get(1))?...;
if !cols.iter().any(|c| c == "new_column") {
    conn.execute("ALTER TABLE ... ADD COLUMN new_column ...", [])?;
}
```

> **Warning**: SQLite cannot DROP COLUMN or rename columns without rebuilding the table. Dead columns are left in place (e.g., `stats_anchor.total_requests`).

---

## Common Mistakes

### Don't: Accumulate stats in a separate table

```rust
// Wrong: coupling retention with stats
conn.execute("UPDATE stats_anchor SET total_requests = total_requests + 1", [])?;
// Then on cleanup: roll up before delete
```

```rust
// Correct: query logs directly
conn.prepare_cached("SELECT COUNT(*), SUM(chars) FROM translation_logs")?
```

### Don't: Use transactions for single statements

```rust
// Wrong: unnecessary transaction overhead
conn.execute_batch("BEGIN")?;
conn.execute("INSERT ...", params![...])?;
conn.execute_batch("COMMIT")?;

// Correct: single statement is already atomic
conn.execute("INSERT ...", params![...])?;
```
