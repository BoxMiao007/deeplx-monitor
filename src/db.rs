use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;
             PRAGMA cache_size=-8000;"
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS stats_anchor (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                total_requests INTEGER NOT NULL DEFAULT 0,
                total_chars INTEGER NOT NULL DEFAULT 0,
                anchor_date TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS translation_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chars INTEGER NOT NULL,
                source_lang TEXT NOT NULL,
                target_lang TEXT NOT NULL,
                status TEXT NOT NULL,
                error_msg TEXT,
                created_at TEXT DEFAULT (datetime('now', 'localtime'))
            )",
            [],
        )?;

        {
            let mut stmt = conn.prepare_cached("PRAGMA table_info(translation_logs)")?;
            let cols: Vec<String> = stmt.query_map([], |row| row.get(1))?.filter_map(|r| r.ok()).collect();
            if !cols.iter().any(|c| c == "source_chars") {
                conn.execute("ALTER TABLE translation_logs ADD COLUMN source_chars INTEGER NOT NULL DEFAULT chars", [])?;
            }
            if !cols.iter().any(|c| c == "target_chars") {
                conn.execute("ALTER TABLE translation_logs ADD COLUMN target_chars INTEGER NOT NULL DEFAULT 0", [])?;
            }
        }

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON translation_logs(created_at)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_source_lang_created ON translation_logs(source_lang, created_at)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_target_lang_created ON translation_logs(target_lang, created_at)",
            [],
        )?;

        // 缓存统计持久化列（重启后恢复命中/未命中计数）
        let _ = conn.execute(
            "ALTER TABLE stats_anchor ADD COLUMN cache_hits INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE stats_anchor ADD COLUMN cache_misses INTEGER NOT NULL DEFAULT 0",
            [],
        );

        conn.execute(
            "INSERT OR IGNORE INTO stats_anchor (id, total_requests, total_chars, anchor_date)
             VALUES (1, 0, 0, '')",
            [],
        )?;

        Ok(())
    }

    pub fn log_translation(
        &self,
        source_lang: &str,
        target_lang: &str,
        source_chars: i64,
        target_chars: i64,
        status: &str,
        error_msg: Option<&str>,
    ) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status, error_msg)
             VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6)",
            params![source_chars, source_lang, target_lang, target_chars, status, error_msg],
        )?;
        Ok(conn.last_insert_rowid())
    }


    pub fn save_cache_stats(&self, hits: u64, misses: u64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE stats_anchor SET cache_hits = ?1, cache_misses = ?2 WHERE id = 1",
            params![hits as i64, misses as i64],
        )?;
        Ok(())
    }

    pub fn load_cache_stats(&self) -> (u64, u64) {
        let conn = self.conn.lock().unwrap();
        let result = conn.prepare_cached(
            "SELECT cache_hits, cache_misses FROM stats_anchor WHERE id = 1",
        )
        .and_then(|mut stmt| {
            stmt.query_row([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
        });
        match result {
            Ok((hits, misses)) => (hits as u64, misses as u64),
            Err(_) => (0, 0),
        }
    }

    pub fn get_current_log_totals(&self) -> SqliteResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT COUNT(*), COALESCE(SUM(chars), 0) FROM translation_logs",
        )?;
        stmt.query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
    }

    pub fn get_period_stats(&self, days: u32) -> SqliteResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT COUNT(*), COALESCE(SUM(chars), 0) FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')"
        )?;
        stmt.query_row(params![days], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
    }

    pub fn get_requests(
        &self,
        period_days: Option<u32>,
        page: u32,
        page_size: u32,
    ) -> SqliteResult<(Vec<RequestLog>, i64)> {
        let conn = self.conn.lock().unwrap();
        let offset = (page - 1) * page_size;

        let (count_sql, data_sql) = if period_days.is_some() {
            (
                "SELECT COUNT(*), COALESCE(SUM(chars), 0) FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')",
                "SELECT id, chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
                 ORDER BY id DESC LIMIT ?2 OFFSET ?3",
            )
        } else {
            (
                "SELECT COUNT(*), COALESCE(SUM(chars), 0) FROM translation_logs",
                "SELECT id, chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
                 FROM translation_logs ORDER BY id DESC LIMIT ?1 OFFSET ?2",
            )
        };

        let total = if let Some(days) = period_days {
            let mut stmt = conn.prepare_cached(count_sql)?;
            stmt.query_row(params![days], |row| row.get::<_, i64>(0))?
        } else {
            let mut stmt = conn.prepare_cached(count_sql)?;
            stmt.query_row([], |row| row.get::<_, i64>(0))?
        };

        let mut logs = Vec::new();
        if let Some(days) = period_days {
            let mut stmt = conn.prepare_cached(data_sql)?;
            let rows = stmt.query_map(params![days, page_size, offset], map_request_log)?;
            for row in rows { logs.push(row?); }
        } else {
            let mut stmt = conn.prepare_cached(data_sql)?;
            let rows = stmt.query_map(params![page_size, offset], map_request_log)?;
            for row in rows { logs.push(row?); }
        }

        Ok((logs, total))
    }

    pub fn cleanup_old_logs(&self, max_entries: u32) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();

        let total_count: i64 = conn.prepare_cached(
            "SELECT COUNT(*) FROM translation_logs"
        )?.query_row([], |row| row.get(0))?;

        let to_delete = total_count - max_entries as i64;
        if to_delete <= 0 {
            return Ok(0);
        }

        conn.execute(
            "DELETE FROM translation_logs WHERE id IN (SELECT id FROM translation_logs ORDER BY id ASC LIMIT ?1)",
            params![to_delete],
        )?;

        Ok(to_delete)
    }

    pub fn replace_with_demo_data(&self, seed: u64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM translation_logs", [])?;
        conn.execute("DELETE FROM sqlite_sequence WHERE name = 'translation_logs'", [])?;
        conn.execute(
            "UPDATE stats_anchor SET cache_hits = 0, cache_misses = 0 WHERE id = 1",
            [],
        )?;

        let lang_pairs = [
            ("EN", "ZH"), ("JA", "ZH"), ("ZH", "EN"), ("KO", "ZH"),
            ("FR", "EN"), ("DE", "ZH"), ("ES", "EN"), ("RU", "ZH"),
            ("PT", "EN"), ("IT", "ZH"), ("TR", "EN"), ("AR", "ZH"),
        ];
        let error_messages = [
            "Upstream timeout after 10s",
            "HTTP 429 Too Many Requests",
            "TLS handshake failed",
            "Invalid JSON response from upstream",
            "Synthetic upstream error for UI QA",
        ];

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let now = now - (now % 3600);
        let mut rng = SimpleRng::new(seed);
        let mut rows = Vec::new();

        for day_offset in 0..365_i64 {
            let day_start = now - (364 - day_offset) * 86_400;
            let samples_per_day = if day_offset >= 300 { 10 } else if day_offset >= 200 { 7 } else { 5 };
            for sample_idx in 0..samples_per_day {
                let pair = lang_pairs[((day_offset + sample_idx as i64) as usize) % lang_pairs.len()];
                let hour = ((sample_idx as i64 * 3 + day_offset) % 24) * 3600;
                let minute = ((sample_idx as i64 * 11) % 60) * 60;
                let created_at = day_start + hour + minute;
                let source_chars = 80 + ((day_offset * 37 + sample_idx as i64 * 19) % 620);
                let target_chars = std::cmp::max(40, source_chars * (85 + ((sample_idx as i64 % 5) * 6)) / 100);
                let is_error = sample_idx == 0 && day_offset % 7 == 0;
                let status = if is_error { "error" } else { "success" };
                let error_msg = if is_error {
                    Some(error_messages[rng.next_usize(error_messages.len())])
                } else {
                    None
                };
                rows.push((pair.0, pair.1, source_chars, target_chars, status, error_msg, created_at));
            }
        }

        let recent_langs = [("EN", "ZH"), ("JA", "ZH"), ("ZH", "EN"), ("DE", "ZH")];
        for hour_offset in 0..24_i64 {
            let created_at = now - (23 - hour_offset) * 3600;
            let pair = recent_langs[(hour_offset as usize) % recent_langs.len()];
            let source_chars = 120 + (hour_offset * 23) % 520;
            let target_chars = source_chars + (hour_offset % 7) * 9;
            let is_error = hour_offset % 6 == 0;
            let status = if is_error { "error" } else { "success" };
            let error_msg = if is_error {
                Some("Synthetic upstream error for UI QA")
            } else {
                None
            };
            rows.push((pair.0, pair.1, source_chars, target_chars, status, error_msg, created_at));
        }

        for (source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at) in rows {
            conn.execute(
                "INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at)
                 VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6, datetime(?7, 'unixepoch', 'localtime'))",
                params![source_chars, source_lang, target_lang, target_chars, status, error_msg, created_at],
            )?;
        }

        Ok(())
    }

    pub fn get_hourly_stats(&self, hours: u32) -> SqliteResult<Vec<HourlyStat>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT strftime('%Y-%m-%d %H:00', created_at) as hour,
                    COUNT(*) as count,
                    COALESCE(SUM(chars), 0) as chars
             FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' hours', 'localtime')
             GROUP BY hour
             ORDER BY hour"
        )?;
        let rows = stmt.query_map(params![hours], |row| {
            Ok(HourlyStat {
                hour: row.get(0)?,
                count: row.get(1)?,
                chars: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_lang_stats(&self) -> SqliteResult<Vec<LangStat>> {
        let conn = self.conn.lock().unwrap();
        let query = r#"
            SELECT source_lang, SUM(source_chars), 0 AS target_chars
            FROM translation_logs GROUP BY source_lang
            UNION ALL
            SELECT target_lang, 0 AS source_chars, SUM(target_chars)
            FROM translation_logs GROUP BY target_lang
        "#;
        let mut stmt = conn.prepare_cached(query)?;
        let rows = stmt.query_map([], |row| {
            Ok(LangStat {
                lang: row.get(0)?,
                source_chars: row.get(1)?,
                target_chars: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_lang_stats_by_days(&self, days: u32) -> SqliteResult<Vec<LangStat>> {
        let conn = self.conn.lock().unwrap();
        let query = r#"
            SELECT source_lang, SUM(source_chars), 0 AS target_chars
            FROM translation_logs
            WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
            GROUP BY source_lang
            UNION ALL
            SELECT target_lang, 0 AS source_chars, SUM(target_chars)
            FROM translation_logs
            WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
            GROUP BY target_lang
        "#;
        let mut stmt = conn.prepare_cached(query)?;
        let rows = stmt.query_map(params![days], |row| {
            Ok(LangStat {
                lang: row.get(0)?,
                source_chars: row.get(1)?,
                target_chars: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_lang_hourly_stats(&self) -> SqliteResult<Vec<LangHourlyUsage>> {
        let conn = self.conn.lock().unwrap();
        let query = r#"
            SELECT hour, lang, SUM(cnt) AS count
            FROM (
                SELECT
                    strftime('%Y-%m-%d %H:00', created_at) AS hour,
                    source_lang AS lang,
                    COUNT(*) AS cnt
                FROM translation_logs
                WHERE created_at >= datetime('now', '-24 hours', 'localtime')
                GROUP BY hour, source_lang
                UNION ALL
                SELECT
                    strftime('%Y-%m-%d %H:00', created_at) AS hour,
                    target_lang AS lang,
                    COUNT(*) AS cnt
                FROM translation_logs
                WHERE created_at >= datetime('now', '-24 hours', 'localtime')
                GROUP BY hour, target_lang
            )
            GROUP BY hour, lang
            ORDER BY hour, lang
        "#;
        let mut stmt = conn.prepare_cached(query)?;
        let rows = stmt.query_map([], |row| {
            Ok(LangHourlyUsage {
                hour: row.get(0)?,
                lang: row.get(1)?,
                count: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_lang_daily_stats(&self, days: u32) -> SqliteResult<Vec<LangHourlyUsage>> {
        let conn = self.conn.lock().unwrap();
        let query = r#"
            SELECT day AS hour, lang, SUM(cnt) AS count
            FROM (
                SELECT
                    strftime('%Y-%m-%d', created_at) AS day,
                    source_lang AS lang,
                    COUNT(*) AS cnt
                FROM translation_logs
                WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
                GROUP BY day, source_lang
                UNION ALL
                SELECT
                    strftime('%Y-%m-%d', created_at) AS day,
                    target_lang AS lang,
                    COUNT(*) AS cnt
                FROM translation_logs
                WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
                GROUP BY day, target_lang
            )
            GROUP BY day, lang
            ORDER BY day, lang
        "#;
        let mut stmt = conn.prepare_cached(query)?;
        let rows = stmt.query_map(params![days], |row| {
            Ok(LangHourlyUsage {
                hour: row.get(0)?,
                lang: row.get(1)?,
                count: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_daily_stats(&self, days: u32) -> SqliteResult<Vec<DailyStat>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT strftime('%Y-%m-%d', created_at) as day,
                    COUNT(*) as count,
                    COALESCE(SUM(chars), 0) as chars
             FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
             GROUP BY day
             ORDER BY day"
        )?;
        let rows = stmt.query_map(params![days], |row| {
            Ok(DailyStat {
                day: row.get(0)?,
                count: row.get(1)?,
                chars: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_heatmap_by_weekday(&self, days: u32) -> SqliteResult<Vec<HeatmapCell>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT CAST(strftime('%w', created_at) AS INTEGER) as weekday,
                    CAST(strftime('%H', created_at) AS INTEGER) as hour,
                    COUNT(*) as count
             FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
             GROUP BY weekday, hour
             ORDER BY weekday, hour"
        )?;
        let rows = stmt.query_map(params![days], |row| {
            let weekday: u32 = row.get(0)?;
            let hour: u32 = row.get(1)?;
            let count: i64 = row.get(2)?;
            let weekday_names = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
            Ok(HeatmapCell {
                x: weekday_names[weekday as usize].to_string(),
                y: hour,
                count,
            })
        })?;
        rows.collect()
    }

    pub fn get_heatmap_by_date(&self, days: u32) -> SqliteResult<Vec<HeatmapCell>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT strftime('%Y-%m-%d', created_at) as day,
                    CAST(strftime('%H', created_at) AS INTEGER) as hour,
                    COUNT(*) as count
             FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
             GROUP BY day, hour
             ORDER BY day, hour"
        )?;
        let rows = stmt.query_map(params![days], |row| {
            Ok(HeatmapCell {
                x: row.get(0)?,
                y: row.get(1)?,
                count: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_error_trend_hourly(&self, days: u32) -> SqliteResult<Vec<ErrorTrendPoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT strftime('%Y-%m-%d %H:00', created_at) as time_bucket,
                    COUNT(*) as total,
                    SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors
             FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
             GROUP BY time_bucket
             ORDER BY time_bucket"
        )?;
        let rows = stmt.query_map(params![days], |row| {
            let total: i64 = row.get(1)?;
            let errors: i64 = row.get(2)?;
            let error_rate = if total > 0 { errors as f64 / total as f64 } else { 0.0 };
            Ok(ErrorTrendPoint {
                time: row.get(0)?,
                total,
                errors,
                error_rate,
            })
        })?;
        rows.collect()
    }

    pub fn get_error_trend_daily(&self, days: u32) -> SqliteResult<Vec<ErrorTrendPoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT strftime('%Y-%m-%d', created_at) as time_bucket,
                    COUNT(*) as total,
                    SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors
             FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
             GROUP BY time_bucket
             ORDER BY time_bucket"
        )?;
        let rows = stmt.query_map(params![days], |row| {
            let total: i64 = row.get(1)?;
            let errors: i64 = row.get(2)?;
            let error_rate = if total > 0 { errors as f64 / total as f64 } else { 0.0 };
            Ok(ErrorTrendPoint {
                time: row.get(0)?,
                total,
                errors,
                error_rate,
            })
        })?;
        rows.collect()
    }

    pub fn get_export_logs(
        &self,
        start: Option<&str>,
        end: Option<&str>,
        lang: Option<&str>,
        status: Option<&str>,
    ) -> SqliteResult<Vec<RequestLog>> {
        let conn = self.conn.lock().unwrap();
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(s) = start {
            conditions.push(format!("created_at >= ?{}", param_values.len() + 1));
            param_values.push(Box::new(s.to_string()));
        }
        if let Some(e) = end {
            conditions.push(format!("created_at <= ?{}", param_values.len() + 1));
            param_values.push(Box::new(e.to_string()));
        }
        if let Some(l) = lang {
            conditions.push(format!(
                "(source_lang = ?{0} OR target_lang = ?{0})",
                param_values.len() + 1
            ));
            param_values.push(Box::new(l.to_string()));
        }
        if let Some(st) = status {
            conditions.push(format!("status = ?{}", param_values.len() + 1));
            param_values.push(Box::new(st.to_string()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
             FROM translation_logs {} ORDER BY id DESC LIMIT 10000",
            where_clause
        );

        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params.as_slice(), map_request_log)?;
        rows.collect()
    }
}

fn map_request_log(row: &rusqlite::Row) -> rusqlite::Result<RequestLog> {
    Ok(RequestLog {
        id: row.get(0)?,
        chars: row.get(1)?,
        source_lang: row.get(2)?,
        target_lang: row.get(3)?,
        source_chars: row.get(4)?,
        target_chars: row.get(5)?,
        status: row.get(6)?,
        error_msg: row.get(7)?,
        created_at: row.get(8)?,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct HourlyStat {
    pub hour: String,
    pub count: i64,
    pub chars: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LangStat {
    pub lang: String,
    pub source_chars: i64,
    pub target_chars: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LangHourlyUsage {
    pub hour: String,
    pub lang: String,
    pub count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyStat {
    pub day: String,
    pub count: i64,
    pub chars: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HeatmapCell {
    pub x: String,
    pub y: u32,
    pub count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorTrendPoint {
    pub time: String,
    pub total: i64,
    pub errors: i64,
    pub error_rate: f64,
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        self.state
    }

    fn next_usize(&mut self, modulo: usize) -> usize {
        (self.next_u64() % modulo as u64) as usize
    }
}
