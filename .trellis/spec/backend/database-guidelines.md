# Database Guidelines

> Database patterns and conventions for this project.

---

## Overview

<!--
Document your project's database conventions here.

Questions to answer:
- What ORM/query library do you use?
- How are migrations managed?
- What are the naming conventions for tables/columns?
- How do you handle transactions?
-->

(To be filled by the team)

---

## Query Patterns

<!-- How should queries be written? Batch operations? -->

### Scenario: Explicit time-range queries for dashboard stats

#### 1. Scope / Trigger
- Trigger: dashboard and analysis views now distinguish `today` from `24h`.
- The API layer forwards an explicit `range` value (`today`, `24h`, `7d`, `30d`, `90d`, `all`) instead of relying on `days=1` to carry two meanings.
- DB helpers must preserve both the local-midnight window and the rolling 24-hour window.

#### 2. Signatures
- `Database.get_today_stats_filtered(endpoint: Option<&str>) -> SqliteResult<(i64, i64)>`
- `Database.get_period_stats_filtered(days: u32, endpoint: Option<&str>) -> SqliteResult<(i64, i64)>`
- `Database.get_today_hourly_stats_filtered(endpoint: Option<&str>) -> SqliteResult<Vec<HourlyStat>>`
- `Database.get_today_daily_stats_filtered(endpoint: Option<&str>) -> SqliteResult<Vec<DailyStat>>`
- `Database.get_today_lang_stats_filtered(endpoint: Option<&str>) -> SqliteResult<Vec<LangStat>>`
- `Database.get_timeline_data_today() -> SqliteResult<Vec<TimelineBlock>>`

#### 3. Contracts
- `today` means local midnight inclusive through "now".
- `24h` means the last rolling 24 hours from "now".
- `days=N` remains backward compatible for rolling N-day queries.
- `all` maps to aggregate totals, while charts still limit daily output to the existing 90-day window.
- Empty endpoint filters (`None` or `""`) must read the global rows.

#### 4. Validation & Error Matrix
- Unknown `range` value -> handled by the API layer fallback, not by the DB helpers.
- No matching rows -> return an empty collection or `(0, 0)`, never an error.
- Endpoint not present in aggregated rows -> return empty results for that endpoint.

#### 5. Good/Base/Bad Cases
- Good: `today` excludes yesterday 23:00 rows.
- Base: `24h` includes rows within the last 24 hours even if they started yesterday.
- Bad: using `datetime('now', '-1 days', 'localtime')` to represent `today`.

#### 6. Tests Required
- Add a regression test that seeds `hourly_stats` with yesterday 23:00 and today rows, then asserts `get_today_stats_filtered()` only returns the today row.
- Add a regression test that seeds `hourly_lang_stats` and asserts `get_today_lang_stats_filtered()` excludes yesterday's bucket.
- Add a regression test that seeds `translation_logs` and asserts `get_timeline_data_today()` only includes today entries.

#### 7. Wrong vs Correct

##### Wrong
```sql
WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-1 days', 'localtime'))
```

##### Correct
```sql
WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', 'localtime', 'start of day'))
```

---

## Migrations

<!-- How to create and run migrations -->

(To be filled by the team)

---

## Naming Conventions

<!-- Table names, column names, index names -->

(To be filled by the team)

---

## Common Mistakes

<!-- Database-related mistakes your team has made -->

(To be filled by the team)
