# Research: UI Bugs Investigation

- **Query**: Investigate 5 UI bugs across Overview, Analysis, and Logs tabs
- **Scope**: internal
- **Date**: 2026-05-26

## Bug 1: Overview Tab - Auto Refresh Persistence

### Files Found

| File Path | Description |
|---|---|
| `frontend/src/stores/useStatsStore.ts` | Zustand store with `refreshInterval` state |

### Findings

`refreshInterval` is stored as plain Zustand state with initial value `0` (line 40):

```typescript
refreshInterval: 0,
```

The setter is a simple `set()` call (line 44):

```typescript
setRefreshInterval: (seconds) => set({ refreshInterval: seconds }),
```

**Root cause**: There is NO `persist` middleware applied to this store. The `refreshInterval` value is purely in-memory and resets to `0` (off) on every page reload. Zustand supports `persist` middleware with localStorage, but it is not used here.

---

## Bug 2: Overview Tab - Refresh Button and Controls Layout

### Files Found

| File Path | Description |
|---|---|
| `frontend/src/pages/Overview.tsx` (lines 153-179) | Controls section with Select + Button |
| `frontend/src/pages/Overview.module.scss` (lines 7-12) | `.controls` CSS class |

### Findings

The controls section renders:
1. A `<Select>` for date range
2. A `<Select>` for auto-refresh interval
3. A `<Button variant="ghost" size="sm">` for manual refresh

The `.controls` CSS:

```scss
.controls {
  display: flex;
  align-items: flex-end;
  gap: 16px;
  flex-wrap: wrap;
}
```

**Observation**: The layout is a simple flex row with `align-items: flex-end`. The refresh button sits inline with the selects. There is no special alignment or spacing logic to visually separate the button from the selects. On narrow screens, `flex-wrap: wrap` may cause the button to wrap to a new line without any visual grouping.

---

## Bug 3: Overview Tab - TrendChart Single Bar Width

### Files Found

| File Path | Description |
|---|---|
| `frontend/src/components/charts/TrendChart.tsx` | Chart.js bar+line combo chart |

### Findings

The bar dataset configuration (lines 85-106):

```typescript
barDatasets.map((ds, i) => {
  const colorPair = CHART_COLORS[i % CHART_COLORS.length]
  return {
    type: 'bar' as const,
    label: ds.label,
    data: ds.data,
    backgroundColor: ...,
    borderColor: ds.borderColor ?? colorPair.base,
    borderWidth: 1,
    borderRadius: 3,
    stack: 'endpoints',
    order: 1,
  }
})
```

**Root cause**: There is NO `barThickness`, `maxBarThickness`, `barPercentage`, or `categoryPercentage` setting anywhere in the chart config. Chart.js defaults to auto-sizing bars based on available space. When there is only a single data point (or very few bars), Chart.js will stretch the bar to fill the entire category width, making it appear excessively wide.

The chart options (lines 115-154) also do not set any bar-width constraints on the x-axis scale.

---

## Bug 4: Analysis Tab - Language Stats Duplicate Entries

### Files Found

| File Path | Description |
|---|---|
| `src/db.rs` (lines 574-648) | `get_lang_stats_filtered` and `get_lang_stats_by_days_filtered` |
| `src/api.rs` (lines 405-421) | `/api/lang-stats` endpoint handler |
| `frontend/src/types/index.ts` (lines 73-77) | `LangStat` type definition |

### Findings

The SQL query uses `UNION ALL` to produce two separate rows per language:

```sql
SELECT source_lang, SUM(source_chars), 0 AS target_chars
FROM translation_logs GROUP BY source_lang
UNION ALL
SELECT target_lang, 0 AS source_chars, SUM(target_chars)
FROM translation_logs GROUP BY target_lang
```

**Root cause**: This query intentionally produces TWO rows for the same language — one with `source_chars` populated (and `target_chars = 0`), and another with `target_chars` populated (and `source_chars = 0`). The API returns these raw rows directly without merging.

The frontend `LangStat` type expects:
```typescript
export interface LangStat {
  lang: string
  source_chars: number
  target_chars: number
}
```

The expectation is that each language has ONE row with both `source_chars` and `target_chars` filled. But the backend returns two rows per language (e.g., `{lang: "ZH", source_chars: 500, target_chars: 0}` and `{lang: "ZH", source_chars: 0, target_chars: 300}`).

**Fix options**:
1. (Backend) Rewrite SQL to use a single query with conditional aggregation: `SELECT lang, SUM(CASE WHEN role='source' THEN chars ELSE 0 END) AS source_chars, ...` — or use a subquery/CTE approach.
2. (Frontend) Merge rows client-side by grouping on `lang` and summing `source_chars` + `target_chars`.

---

## Bug 5: Logs Tab - Missing Endpoint and Latency Columns

### Files Found

| File Path | Description |
|---|---|
| `frontend/src/pages/Logs.tsx` (lines 66-89) | Table columns include endpoint and latency |
| `frontend/src/types/index.ts` (lines 31-43) | `RequestLog` type with `endpoint_name?` and `latency_ms?` |
| `src/db.rs` (lines 298-313) | SQL query for `get_requests_filtered` |
| `src/db.rs` (lines 1006-1018) | `map_request_log` function |
| `src/db.rs` (lines 1020-1031) | Rust `RequestLog` struct |

### Findings

The frontend table DOES render endpoint and latency columns (lines 82-83):
```tsx
<td>{log.endpoint_name ?? '-'}</td>
<td className={styles.mono}>{log.latency_ms != null ? `${log.latency_ms}ms` : '-'}</td>
```

The frontend type defines them as optional:
```typescript
endpoint_name?: string
latency_ms?: number
```

**Root cause**: The backend SQL query does NOT select `endpoint_name` or `latency_ms`:

```sql
SELECT id, chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
FROM translation_logs ORDER BY id DESC LIMIT ?1 OFFSET ?2
```

And the `map_request_log` function only maps 9 columns (id through created_at). The Rust `RequestLog` struct also lacks these fields:

```rust
pub struct RequestLog {
    pub id: i64,
    pub chars: i64,
    pub source_lang: String,
    pub target_lang: String,
    pub source_chars: i64,
    pub target_chars: i64,
    pub status: String,
    pub error_msg: Option<String>,
    pub created_at: String,
}
```

Despite the database table having `endpoint_name` and `latency_ms` columns (confirmed by the INSERT statement at line 183), the SELECT query and struct do not include them. The frontend shows `-` for both columns because the JSON response never contains these fields.

**Fix**: Add `endpoint_name` and `latency_ms` to:
1. The SQL SELECT in `get_requests_filtered` (both branches)
2. The `map_request_log` function
3. The Rust `RequestLog` struct

---

## Summary of Root Causes

| Bug | Root Cause | Fix Location |
|---|---|---|
| Auto refresh persistence | No localStorage/persist middleware | `useStatsStore.ts` |
| Refresh button layout | Simple flex row, no visual grouping | `Overview.module.scss` |
| TrendChart bar width | No `maxBarThickness` or `barPercentage` set | `TrendChart.tsx` |
| Lang stats duplicates | UNION ALL produces 2 rows per lang, no merge | `src/db.rs` SQL query |
| Missing endpoint/latency | SQL SELECT and struct omit these columns | `src/db.rs` struct + query |
