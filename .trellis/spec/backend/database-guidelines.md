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

Primary data table. Used for time-series queries (charts, heatmaps, period stats). Cumulative totals are tracked in `stats_anchor`.

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

### `stats_anchor` — 累计统计与缓存计数持久化

Persists cumulative request/char counters (survive log cleanup) and cache hit/miss counters (survive restarts).

```sql
CREATE TABLE stats_anchor (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    total_requests INTEGER NOT NULL DEFAULT 0,  -- 累计请求总数（只增不减）
    total_chars INTEGER NOT NULL DEFAULT 0,     -- 累计字符总数（只增不减）
    anchor_date TEXT NOT NULL,                  -- legacy, unused
    cache_hits INTEGER NOT NULL DEFAULT 0,
    cache_misses INTEGER NOT NULL DEFAULT 0
)
```

- `total_requests` / `total_chars`: incremented on every `log_translation()` call. Read by `get_current_log_totals()`. Not affected by log cleanup.
- `cache_hits` / `cache_misses`: saved periodically by background task, loaded on restart.
- `anchor_date`: legacy column, not actively used (kept to avoid SQLite rebuild).

---

## Design Decisions

### Cumulative Counters in stats_anchor

**Context**: Previously, stats were computed in real-time via `SELECT COUNT(*), SUM(chars) FROM translation_logs`. However, `cleanup_old_logs()` deletes old rows, causing the real-time totals to decrease — a confusing UX where "total requests" goes down.

**Decision**: `stats_anchor.total_requests` and `stats_anchor.total_chars` are incremented on every `log_translation()` call. `get_current_log_totals()` reads from `stats_anchor` directly. Cleanup no longer affects displayed totals.

**Why**:
- Totals are monotonically increasing (never decrease after cleanup)
- Single extra UPDATE per insert is negligible overhead
- No coupling between retention and stats display
- Migration in `init()` seeds from existing logs for backward compatibility

### Count-based Retention (not time-based)

**Context**: Previously used `retention_days` to delete logs older than N days.

**Decision**: Use `max_log_entries` to keep only the N most recent entries.

**Why**:
- Predictable storage usage regardless of traffic patterns
- Simpler mental model: "keep last 10000 entries"
- No time-zone edge cases

---

## Query Patterns

### Log insertion — INSERT + UPDATE anchor (same lock scope)

```rust
pub fn log_translation(...) -> SqliteResult<i64> {
    let conn = self.conn.lock().unwrap();
    conn.execute("INSERT INTO translation_logs ...", params![...])?;
    conn.execute(
        "UPDATE stats_anchor SET total_requests = total_requests + 1, total_chars = total_chars + ?1 WHERE id = 1",
        params![source_chars],
    )?;
    Ok(conn.last_insert_rowid())
}
```

Both statements run under the same `Mutex` lock, ensuring atomicity at the application level.

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

> **Warning**: SQLite cannot DROP COLUMN or rename columns without rebuilding the table. Dead columns are left in place (e.g., `stats_anchor.anchor_date`).

---

## Common Mistakes

### Don't: Compute totals from translation_logs directly

```rust
// Wrong: totals decrease when cleanup_old_logs() runs
conn.prepare_cached("SELECT COUNT(*), SUM(chars) FROM translation_logs")?
```

```rust
// Correct: read cumulative counters from stats_anchor
conn.prepare_cached("SELECT total_requests, total_chars FROM stats_anchor WHERE id = 1")?
```

### Don't: Forget to sync anchor after bulk data changes

```rust
// Wrong: replace_with_demo_data() inserts rows but doesn't update anchor
conn.execute("DELETE FROM translation_logs", [])?;
// ... insert demo rows ...
// anchor still has old totals!

// Correct: sync anchor from actual log counts after bulk operations
let (count, chars) = /* SELECT COUNT(*), SUM(chars) FROM translation_logs */;
conn.execute("UPDATE stats_anchor SET total_requests = ?1, total_chars = ?2 WHERE id = 1", ...)?;
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
