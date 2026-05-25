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
            if !cols.iter().any(|c| c == "endpoint_name") {
                conn.execute("ALTER TABLE translation_logs ADD COLUMN endpoint_name TEXT NOT NULL DEFAULT ''", [])?;
            }
        }

        // 迁移：修复 ALTER TABLE DEFAULT chars 产生的文本值（SQLite 将其视为字面量 "chars"）
        conn.execute(
            "UPDATE translation_logs SET source_chars = chars WHERE typeof(source_chars) != 'integer'",
            [],
        )?;

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

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_endpoint_name ON translation_logs(endpoint_name)",
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

        // 迁移：将 stats_anchor 从单行表（CHECK id=1）重建为多行表（支持 per-endpoint 行）
        // 检查是否有 endpoint_name 列，如果没有则需要重建
        {
            let mut stmt = conn.prepare_cached("PRAGMA table_info(stats_anchor)")?;
            let anchor_cols: Vec<String> = stmt.query_map([], |row| row.get(1))?.filter_map(|r| r.ok()).collect();
            if !anchor_cols.iter().any(|c| c == "endpoint_name") {
                // 读取现有数据
                let (total_req, total_chars, cache_hits, cache_misses): (i64, i64, i64, i64) = conn
                    .prepare_cached("SELECT total_requests, total_chars, cache_hits, cache_misses FROM stats_anchor WHERE id = 1")
                    .and_then(|mut s| s.query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))))
                    .unwrap_or((0, 0, 0, 0));

                // 重建表（去掉 CHECK 约束，添加 endpoint_name 列）
                conn.execute("DROP TABLE stats_anchor", [])?;
                conn.execute(
                    "CREATE TABLE stats_anchor (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        total_requests INTEGER NOT NULL DEFAULT 0,
                        total_chars INTEGER NOT NULL DEFAULT 0,
                        anchor_date TEXT NOT NULL DEFAULT '',
                        cache_hits INTEGER NOT NULL DEFAULT 0,
                        cache_misses INTEGER NOT NULL DEFAULT 0,
                        endpoint_name TEXT NOT NULL DEFAULT ''
                    )",
                    [],
                )?;
                conn.execute(
                    "CREATE UNIQUE INDEX IF NOT EXISTS idx_stats_anchor_endpoint ON stats_anchor(endpoint_name)",
                    [],
                )?;
                // 恢复全局行
                conn.execute(
                    "INSERT INTO stats_anchor (total_requests, total_chars, anchor_date, cache_hits, cache_misses, endpoint_name)
                     VALUES (?1, ?2, '', ?3, ?4, '')",
                    params![total_req, total_chars, cache_hits, cache_misses],
                )?;
            }
        }

        // 确保全局行存在
        conn.execute(
            "INSERT OR IGNORE INTO stats_anchor (total_requests, total_chars, anchor_date, endpoint_name)
             VALUES (0, 0, '', '')",
            [],
        )?;

        // 迁移：若 stats_anchor 全局行计数为 0 但已有日志，从现有日志补种累计值
        let (anchor_req, _): (i64, i64) = conn.prepare_cached(
            "SELECT total_requests, total_chars FROM stats_anchor WHERE endpoint_name = ''"
        )?.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        if anchor_req == 0 {
            let (log_count, log_chars): (i64, i64) = conn.prepare_cached(
                "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs"
            )?.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            if log_count > 0 {
                conn.execute(
                    "UPDATE stats_anchor SET total_requests = ?1, total_chars = ?2 WHERE endpoint_name = ''",
                    params![log_count, log_chars],
                )?;
            }
        }

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
        endpoint_name: &str,
    ) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, endpoint_name)
             VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6, ?7)",
            params![source_chars, source_lang, target_lang, target_chars, status, error_msg, endpoint_name],
        )?;
        // 更新全局行
        conn.execute(
            "UPDATE stats_anchor SET total_requests = total_requests + 1, total_chars = total_chars + ?1 WHERE endpoint_name = ''",
            params![source_chars],
        )?;
        // 更新端点行（如果 endpoint_name 非空）
        if !endpoint_name.is_empty() {
            conn.execute(
                "INSERT OR IGNORE INTO stats_anchor (total_requests, total_chars, anchor_date, endpoint_name) VALUES (0, 0, '', ?1)",
                params![endpoint_name],
            )?;
            conn.execute(
                "UPDATE stats_anchor SET total_requests = total_requests + 1, total_chars = total_chars + ?1 WHERE endpoint_name = ?2",
                params![source_chars, endpoint_name],
            )?;
        }
        Ok(conn.last_insert_rowid())
    }


    pub fn save_cache_stats(&self, hits: u64, misses: u64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE stats_anchor SET cache_hits = ?1, cache_misses = ?2 WHERE endpoint_name = ''",
            params![hits as i64, misses as i64],
        )?;
        Ok(())
    }

    pub fn load_cache_stats(&self) -> (u64, u64) {
        let conn = self.conn.lock().unwrap();
        let result = conn.prepare_cached(
            "SELECT cache_hits, cache_misses FROM stats_anchor WHERE endpoint_name = ''",
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
            "SELECT total_requests, total_chars FROM stats_anchor WHERE endpoint_name = ''",
        )?;
        stmt.query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
    }

    /// 获取指定端点的累计统计
    pub fn get_endpoint_totals(&self, endpoint_name: &str) -> SqliteResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT total_requests, total_chars FROM stats_anchor WHERE endpoint_name = ?1",
        )?;
        stmt.query_row(params![endpoint_name], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
            .or(Ok((0, 0)))
    }

    pub fn get_period_stats(&self, days: u32) -> SqliteResult<(i64, i64)> {
        self.get_period_stats_filtered(days, None)
    }

    pub fn get_period_stats_filtered(&self, days: u32, endpoint: Option<&str>) -> SqliteResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        if ep_filter.is_empty() {
            let mut stmt = conn.prepare_cached(
                "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')"
            )?;
            stmt.query_row(params![days], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2"
            )?;
            stmt.query_row(params![days, ep_filter], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        }
    }

    pub fn get_requests(
        &self,
        page: u32,
        page_size: u32,
    ) -> SqliteResult<(Vec<RequestLog>, i64)> {
        self.get_requests_filtered(page, page_size, None)
    }

    pub fn get_requests_filtered(
        &self,
        page: u32,
        page_size: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<(Vec<RequestLog>, i64)> {
        let conn = self.conn.lock().unwrap();
        let offset = (page - 1) * page_size;
        let ep_filter = endpoint.unwrap_or("");

        let total: i64 = if ep_filter.is_empty() {
            conn.prepare_cached("SELECT COUNT(*) FROM translation_logs")?
                .query_row([], |row| row.get(0))?
        } else {
            conn.prepare_cached("SELECT COUNT(*) FROM translation_logs WHERE endpoint_name = ?1")?
                .query_row(params![ep_filter], |row| row.get(0))?
        };

        let mut logs = Vec::new();
        if ep_filter.is_empty() {
            let mut stmt = conn.prepare_cached(
                "SELECT id, chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
                 FROM translation_logs ORDER BY id DESC LIMIT ?1 OFFSET ?2"
            )?;
            let rows = stmt.query_map(params![page_size, offset], map_request_log)?;
            for row in rows { logs.push(row?); }
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT id, chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
                 FROM translation_logs WHERE endpoint_name = ?3 ORDER BY id DESC LIMIT ?1 OFFSET ?2"
            )?;
            let rows = stmt.query_map(params![page_size, offset, ep_filter], map_request_log)?;
            for row in rows { logs.push(row?); }
        };

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
            "UPDATE stats_anchor SET cache_hits = 0, cache_misses = 0 WHERE endpoint_name = ''",
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

        // 同步累计计数器
        let (count, chars): (i64, i64) = conn.prepare_cached(
            "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs"
        )?.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        conn.execute(
            "UPDATE stats_anchor SET total_requests = ?1, total_chars = ?2 WHERE endpoint_name = ''",
            params![count, chars],
        )?;
        // 清除端点行（demo 数据不含 endpoint_name）
        conn.execute("DELETE FROM stats_anchor WHERE endpoint_name != ''", [])?;

        Ok(())
    }

    pub fn get_hourly_stats(&self, hours: u32) -> SqliteResult<Vec<HourlyStat>> {
        self.get_hourly_stats_filtered(hours, None)
    }

    pub fn get_hourly_stats_filtered(&self, hours: u32, endpoint: Option<&str>) -> SqliteResult<Vec<HourlyStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        if ep_filter.is_empty() {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d %H:00', created_at) as hour,
                        COUNT(*) as count,
                        COALESCE(SUM(source_chars), 0) as chars
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' hours', 'localtime')
                 GROUP BY hour
                 ORDER BY hour"
            )?;
            let rows = stmt.query_map(params![hours], |row| {
                Ok(HourlyStat { hour: row.get(0)?, count: row.get(1)?, chars: row.get(2)? })
            })?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d %H:00', created_at) as hour,
                        COUNT(*) as count,
                        COALESCE(SUM(source_chars), 0) as chars
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' hours', 'localtime') AND endpoint_name = ?2
                 GROUP BY hour
                 ORDER BY hour"
            )?;
            let rows = stmt.query_map(params![hours, ep_filter], |row| {
                Ok(HourlyStat { hour: row.get(0)?, count: row.get(1)?, chars: row.get(2)? })
            })?;
            rows.collect()
        }
    }

    pub fn get_lang_stats(&self) -> SqliteResult<Vec<LangStat>> {
        self.get_lang_stats_filtered(None)
    }

    pub fn get_lang_stats_filtered(&self, endpoint: Option<&str>) -> SqliteResult<Vec<LangStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        if ep_filter.is_empty() {
            let query = r#"
                SELECT source_lang, SUM(source_chars), 0 AS target_chars
                FROM translation_logs GROUP BY source_lang
                UNION ALL
                SELECT target_lang, 0 AS source_chars, SUM(target_chars)
                FROM translation_logs GROUP BY target_lang
            "#;
            let mut stmt = conn.prepare_cached(query)?;
            let rows = stmt.query_map([], |row| {
                Ok(LangStat { lang: row.get(0)?, source_chars: row.get(1)?, target_chars: row.get(2)? })
            })?;
            rows.collect()
        } else {
            let query = r#"
                SELECT source_lang, SUM(source_chars), 0 AS target_chars
                FROM translation_logs WHERE endpoint_name = ?1 GROUP BY source_lang
                UNION ALL
                SELECT target_lang, 0 AS source_chars, SUM(target_chars)
                FROM translation_logs WHERE endpoint_name = ?1 GROUP BY target_lang
            "#;
            let mut stmt = conn.prepare_cached(query)?;
            let rows = stmt.query_map(params![ep_filter], |row| {
                Ok(LangStat { lang: row.get(0)?, source_chars: row.get(1)?, target_chars: row.get(2)? })
            })?;
            rows.collect()
        }
    }

    pub fn get_lang_stats_by_days(&self, days: u32) -> SqliteResult<Vec<LangStat>> {
        self.get_lang_stats_by_days_filtered(days, None)
    }

    pub fn get_lang_stats_by_days_filtered(&self, days: u32, endpoint: Option<&str>) -> SqliteResult<Vec<LangStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        if ep_filter.is_empty() {
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
                Ok(LangStat { lang: row.get(0)?, source_chars: row.get(1)?, target_chars: row.get(2)? })
            })?;
            rows.collect()
        } else {
            let query = r#"
                SELECT source_lang, SUM(source_chars), 0 AS target_chars
                FROM translation_logs
                WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2
                GROUP BY source_lang
                UNION ALL
                SELECT target_lang, 0 AS source_chars, SUM(target_chars)
                FROM translation_logs
                WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2
                GROUP BY target_lang
            "#;
            let mut stmt = conn.prepare_cached(query)?;
            let rows = stmt.query_map(params![days, ep_filter], |row| {
                Ok(LangStat { lang: row.get(0)?, source_chars: row.get(1)?, target_chars: row.get(2)? })
            })?;
            rows.collect()
        }
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
        self.get_daily_stats_filtered(days, None)
    }

    pub fn get_daily_stats_filtered(&self, days: u32, endpoint: Option<&str>) -> SqliteResult<Vec<DailyStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        if ep_filter.is_empty() {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d', created_at) as day,
                        COUNT(*) as count,
                        COALESCE(SUM(source_chars), 0) as chars
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
                 GROUP BY day
                 ORDER BY day"
            )?;
            let rows = stmt.query_map(params![days], |row| {
                Ok(DailyStat { day: row.get(0)?, count: row.get(1)?, chars: row.get(2)? })
            })?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d', created_at) as day,
                        COUNT(*) as count,
                        COALESCE(SUM(source_chars), 0) as chars
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2
                 GROUP BY day
                 ORDER BY day"
            )?;
            let rows = stmt.query_map(params![days, ep_filter], |row| {
                Ok(DailyStat { day: row.get(0)?, count: row.get(1)?, chars: row.get(2)? })
            })?;
            rows.collect()
        }
    }

    pub fn get_heatmap_by_weekday(&self, days: u32) -> SqliteResult<Vec<HeatmapCell>> {
        self.get_heatmap_by_weekday_filtered(days, None)
    }

    pub fn get_heatmap_by_weekday_filtered(&self, days: u32, endpoint: Option<&str>) -> SqliteResult<Vec<HeatmapCell>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        if ep_filter.is_empty() {
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
                Ok(HeatmapCell { x: weekday_names[weekday as usize].to_string(), y: hour, count })
            })?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT CAST(strftime('%w', created_at) AS INTEGER) as weekday,
                        CAST(strftime('%H', created_at) AS INTEGER) as hour,
                        COUNT(*) as count
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2
                 GROUP BY weekday, hour
                 ORDER BY weekday, hour"
            )?;
            let rows = stmt.query_map(params![days, ep_filter], |row| {
                let weekday: u32 = row.get(0)?;
                let hour: u32 = row.get(1)?;
                let count: i64 = row.get(2)?;
                let weekday_names = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
                Ok(HeatmapCell { x: weekday_names[weekday as usize].to_string(), y: hour, count })
            })?;
            rows.collect()
        }
    }

    pub fn get_heatmap_by_date(&self, days: u32) -> SqliteResult<Vec<HeatmapCell>> {
        self.get_heatmap_by_date_filtered(days, None)
    }

    pub fn get_heatmap_by_date_filtered(&self, days: u32, endpoint: Option<&str>) -> SqliteResult<Vec<HeatmapCell>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        if ep_filter.is_empty() {
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
                Ok(HeatmapCell { x: row.get(0)?, y: row.get(1)?, count: row.get(2)? })
            })?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d', created_at) as day,
                        CAST(strftime('%H', created_at) AS INTEGER) as hour,
                        COUNT(*) as count
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2
                 GROUP BY day, hour
                 ORDER BY day, hour"
            )?;
            let rows = stmt.query_map(params![days, ep_filter], |row| {
                Ok(HeatmapCell { x: row.get(0)?, y: row.get(1)?, count: row.get(2)? })
            })?;
            rows.collect()
        }
    }

    pub fn get_error_trend_hourly(&self, days: u32) -> SqliteResult<Vec<ErrorTrendPoint>> {
        self.get_error_trend_hourly_filtered(days, None)
    }

    pub fn get_error_trend_hourly_filtered(&self, days: u32, endpoint: Option<&str>) -> SqliteResult<Vec<ErrorTrendPoint>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<ErrorTrendPoint> {
            let total: i64 = row.get(1)?;
            let errors: i64 = row.get(2)?;
            let error_rate = if total > 0 { errors as f64 / total as f64 } else { 0.0 };
            Ok(ErrorTrendPoint { time: row.get(0)?, total, errors, error_rate })
        };
        if ep_filter.is_empty() {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d %H:00', created_at) as time_bucket,
                        COUNT(*) as total,
                        SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
                 GROUP BY time_bucket
                 ORDER BY time_bucket"
            )?;
            let rows = stmt.query_map(params![days], map_row)?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d %H:00', created_at) as time_bucket,
                        COUNT(*) as total,
                        SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2
                 GROUP BY time_bucket
                 ORDER BY time_bucket"
            )?;
            let rows = stmt.query_map(params![days, ep_filter], map_row)?;
            rows.collect()
        }
    }

    pub fn get_error_trend_daily(&self, days: u32) -> SqliteResult<Vec<ErrorTrendPoint>> {
        self.get_error_trend_daily_filtered(days, None)
    }

    pub fn get_error_trend_daily_filtered(&self, days: u32, endpoint: Option<&str>) -> SqliteResult<Vec<ErrorTrendPoint>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<ErrorTrendPoint> {
            let total: i64 = row.get(1)?;
            let errors: i64 = row.get(2)?;
            let error_rate = if total > 0 { errors as f64 / total as f64 } else { 0.0 };
            Ok(ErrorTrendPoint { time: row.get(0)?, total, errors, error_rate })
        };
        if ep_filter.is_empty() {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d', created_at) as time_bucket,
                        COUNT(*) as total,
                        SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
                 GROUP BY time_bucket
                 ORDER BY time_bucket"
            )?;
            let rows = stmt.query_map(params![days], map_row)?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare_cached(
                "SELECT strftime('%Y-%m-%d', created_at) as time_bucket,
                        COUNT(*) as total,
                        SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors
                 FROM translation_logs
                 WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime') AND endpoint_name = ?2
                 GROUP BY time_bucket
                 ORDER BY time_bucket"
            )?;
            let rows = stmt.query_map(params![days, ep_filter], map_row)?;
            rows.collect()
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> Database {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(&path).unwrap();
        std::mem::forget(dir);
        db
    }

    #[test]
    fn test_init_creates_tables() {
        let db = temp_db();
        let totals = db.get_current_log_totals().unwrap();
        assert_eq!(totals, (0, 0));
    }

    #[test]
    fn test_log_translation_increments_anchor() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "").unwrap();
        let (req, chars) = db.get_current_log_totals().unwrap();
        assert_eq!(req, 1);
        assert_eq!(chars, 100);

        db.log_translation("EN", "ZH", 50, 40, "success", None, "").unwrap();
        let (req, chars) = db.get_current_log_totals().unwrap();
        assert_eq!(req, 2);
        assert_eq!(chars, 150);
    }

    #[test]
    fn test_cleanup_does_not_decrease_totals() {
        let db = temp_db();
        for i in 0..20 {
            db.log_translation("EN", "ZH", 10 + i, 8, "success", None, "").unwrap();
        }
        let (req_before, chars_before) = db.get_current_log_totals().unwrap();
        assert_eq!(req_before, 20);

        let deleted = db.cleanup_old_logs(5).unwrap();
        assert_eq!(deleted, 15);

        let (req_after, chars_after) = db.get_current_log_totals().unwrap();
        assert_eq!(req_after, req_before, "总请求数不应因清理而减少");
        assert_eq!(chars_after, chars_before, "总字符数不应因清理而减少");
    }

    #[test]
    fn test_cleanup_no_op_when_under_limit() {
        let db = temp_db();
        for _ in 0..5 {
            db.log_translation("EN", "ZH", 10, 8, "success", None, "").unwrap();
        }
        let deleted = db.cleanup_old_logs(10).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_get_period_stats() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "").unwrap();
        db.log_translation("EN", "ZH", 200, 160, "error", Some("timeout"), "").unwrap();

        let (count, chars) = db.get_period_stats(1).unwrap();
        assert_eq!(count, 2);
        assert_eq!(chars, 300);
    }

    #[test]
    fn test_get_requests_pagination() {
        let db = temp_db();
        for i in 0..10 {
            db.log_translation("EN", "ZH", i + 1, 0, "success", None, "").unwrap();
        }

        let (items, total) = db.get_requests(1, 5).unwrap();
        assert_eq!(total, 10);
        assert_eq!(items.len(), 5);
        // 按 id DESC 排序，第一页应该是最新的
        assert_eq!(items[0].id, 10);
        assert_eq!(items[4].id, 6);

        let (items, _) = db.get_requests(2, 5).unwrap();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].id, 5);
    }

    #[test]
    fn test_error_logging() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 50, 0, "error", Some("upstream timeout"), "").unwrap();

        let (items, _) = db.get_requests(1, 50).unwrap();
        assert_eq!(items[0].status, "error");
        assert_eq!(items[0].error_msg.as_deref(), Some("upstream timeout"));
    }

    #[test]
    fn test_cache_stats_persistence() {
        let db = temp_db();
        db.save_cache_stats(42, 13).unwrap();
        let (hits, misses) = db.load_cache_stats();
        assert_eq!(hits, 42);
        assert_eq!(misses, 13);
    }

    #[test]
    fn test_replace_with_demo_data_syncs_anchor() {
        let db = temp_db();
        // 先插入一些正常数据
        db.log_translation("EN", "ZH", 100, 80, "success", None, "").unwrap();
        let (req_before, _) = db.get_current_log_totals().unwrap();
        assert_eq!(req_before, 1);

        // demo 数据替换后，anchor 应该反映新的日志数量
        db.replace_with_demo_data(12345).unwrap();
        let (req_after, chars_after) = db.get_current_log_totals().unwrap();
        assert!(req_after > 0, "demo 数据后 anchor 应该 > 0");

        // 验证 anchor 与实际日志一致
        let conn = db.conn.lock().unwrap();
        let (log_count, log_chars): (i64, i64) = conn.prepare_cached(
            "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs"
        ).unwrap().query_row([], |row| Ok((row.get(0).unwrap(), row.get(1).unwrap()))).unwrap();
        assert_eq!(req_after, log_count);
        assert_eq!(chars_after, log_chars);
    }

    #[test]
    fn test_migration_seeds_anchor_from_existing_logs() {
        // 模拟旧数据库：有日志但 anchor 为 0
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("migrate.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE translation_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    chars INTEGER NOT NULL,
                    source_lang TEXT NOT NULL,
                    target_lang TEXT NOT NULL,
                    source_chars INTEGER NOT NULL DEFAULT 0,
                    target_chars INTEGER NOT NULL DEFAULT 0,
                    status TEXT NOT NULL,
                    error_msg TEXT,
                    created_at TEXT DEFAULT (datetime('now', 'localtime'))
                );
                INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status) VALUES (100, 'EN', 'ZH', 100, 80, 'success');
                INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status) VALUES (200, 'EN', 'ZH', 200, 160, 'success');
                INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status) VALUES (50, 'JA', 'ZH', 50, 40, 'success');
                CREATE TABLE stats_anchor (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    total_requests INTEGER NOT NULL DEFAULT 0,
                    total_chars INTEGER NOT NULL DEFAULT 0,
                    anchor_date TEXT NOT NULL
                );
                INSERT INTO stats_anchor (id, total_requests, total_chars, anchor_date) VALUES (1, 0, 0, '');"
            ).unwrap();
        }

        // 打开数据库触发 init() 迁移
        let db = Database::new(&path).unwrap();
        let (req, chars) = db.get_current_log_totals().unwrap();
        assert_eq!(req, 3, "迁移应从日志补种 anchor");
        assert_eq!(chars, 350, "迁移应从日志补种字符总数");
        std::mem::forget(dir);
    }

    #[test]
    fn test_anchor_not_reseeded_when_nonzero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no_reseed.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE translation_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    chars INTEGER NOT NULL,
                    source_lang TEXT NOT NULL,
                    target_lang TEXT NOT NULL,
                    source_chars INTEGER NOT NULL DEFAULT 0,
                    target_chars INTEGER NOT NULL DEFAULT 0,
                    status TEXT NOT NULL,
                    error_msg TEXT,
                    created_at TEXT DEFAULT (datetime('now', 'localtime'))
                );
                INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status) VALUES (100, 'EN', 'ZH', 100, 80, 'success');
                CREATE TABLE stats_anchor (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    total_requests INTEGER NOT NULL DEFAULT 0,
                    total_chars INTEGER NOT NULL DEFAULT 0,
                    anchor_date TEXT NOT NULL
                );
                INSERT INTO stats_anchor (id, total_requests, total_chars, anchor_date) VALUES (1, 999, 88888, '');"
            ).unwrap();
        }

        let db = Database::new(&path).unwrap();
        let (req, chars) = db.get_current_log_totals().unwrap();
        assert_eq!(req, 999, "已有非零 anchor 不应被覆盖");
        assert_eq!(chars, 88888);
        std::mem::forget(dir);
    }

    #[test]
    fn test_export_with_filters() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "").unwrap();
        db.log_translation("JA", "ZH", 50, 40, "error", Some("fail"), "").unwrap();
        db.log_translation("EN", "ZH", 200, 160, "success", None, "").unwrap();

        let logs = db.get_export_logs(None, None, Some("JA"), None).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].source_lang, "JA");

        let logs = db.get_export_logs(None, None, None, Some("error")).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].status, "error");
    }

    #[test]
    fn test_heatmap_by_weekday() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "").unwrap();
        let data = db.get_heatmap_by_weekday(30).unwrap();
        assert!(!data.is_empty());
        let weekday_names = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
        assert!(weekday_names.contains(&data[0].x.as_str()));
    }

    // ===== 问题证明测试 =====

    #[test]
    fn proof_chars_column_equals_source_chars_for_new_rows() {
        // 证明: INSERT 语句中 chars 和 source_chars 使用同一个参数 ?1
        // init() 中的迁移确保旧行也被修复: UPDATE ... SET source_chars = chars WHERE typeof != 'integer'
        let db = temp_db();
        let test_cases: Vec<i64> = vec![1, 50, 100, 999, 12345];
        for chars in &test_cases {
            db.log_translation("EN", "ZH", *chars, 0, "success", None, "").unwrap();
        }

        let conn = db.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT chars, source_chars FROM translation_logs ORDER BY id").unwrap();
        let rows: Vec<(i64, i64)> = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0).unwrap(), row.get::<_, i64>(1).unwrap()))
        }).unwrap().filter_map(|r| r.ok()).collect();

        for (i, (chars_col, source_chars_col)) in rows.iter().enumerate() {
            assert_eq!(
                chars_col, source_chars_col,
                "第 {} 行: chars={} != source_chars={}, 新行两列应相等",
                i + 1, chars_col, source_chars_col
            );
            assert_eq!(*chars_col, test_cases[i]);
        }
    }

    #[test]
    fn test_migration_fixes_old_source_chars_text_values() {
        // 验证: init() 中的迁移能修复旧数据库中 source_chars 为文本 "chars" 的行
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old_schema.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("
                CREATE TABLE translation_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    chars INTEGER NOT NULL,
                    source_lang TEXT NOT NULL,
                    target_lang TEXT NOT NULL,
                    status TEXT NOT NULL,
                    error_msg TEXT,
                    created_at TEXT DEFAULT (datetime('now', 'localtime'))
                );
                INSERT INTO translation_logs (chars, source_lang, target_lang, status) VALUES (100, 'EN', 'ZH', 'success');
                INSERT INTO translation_logs (chars, source_lang, target_lang, status) VALUES (200, 'JA', 'ZH', 'success');
                INSERT INTO translation_logs (chars, source_lang, target_lang, status) VALUES (50, 'ZH', 'EN', 'success');
            ").unwrap();
            // 模拟旧的 ALTER TABLE 行为：source_chars 得到文本 "chars"
            conn.execute("ALTER TABLE translation_logs ADD COLUMN source_chars INTEGER NOT NULL DEFAULT chars", []).unwrap();
            conn.execute("ALTER TABLE translation_logs ADD COLUMN target_chars INTEGER NOT NULL DEFAULT 0", []).unwrap();

            // 验证旧行确实有文本值
            let val: String = conn.prepare("SELECT typeof(source_chars) FROM translation_logs LIMIT 1")
                .unwrap().query_row([], |row| row.get(0)).unwrap();
            assert_eq!(val, "text", "旧行的 source_chars 应为文本类型");
        }

        // 打开数据库触发 init() 迁移
        let db = Database::new(&path).unwrap();

        // 验证迁移后 source_chars 被修复为正确的整数值
        let conn = db.conn.lock().unwrap();
        let rows: Vec<(i64, i64, String)> = conn.prepare(
            "SELECT chars, source_chars, typeof(source_chars) FROM translation_logs ORDER BY id"
        ).unwrap().query_map([], |row| {
            Ok((row.get(0).unwrap(), row.get(1).unwrap(), row.get(2).unwrap()))
        }).unwrap().filter_map(|r| r.ok()).collect();

        for (chars, source_chars, type_name) in &rows {
            assert_eq!(type_name, "integer", "迁移后 source_chars 应为 integer 类型");
            assert_eq!(chars, source_chars, "迁移后 source_chars 应等于 chars");
        }

        // 验证 SUM(source_chars) 现在返回正确结果
        let sum: i64 = conn.prepare("SELECT COALESCE(SUM(source_chars), 0) FROM translation_logs")
            .unwrap().query_row([], |row| row.get(0)).unwrap();
        assert_eq!(sum, 350, "SUM(source_chars) 应为 100+200+50=350");
        std::mem::forget(dir);
    }

    #[test]
    fn test_concurrent_log_and_read() {
        use std::sync::Arc;
        use std::thread;

        let db = Arc::new(temp_db());
        let mut handles = vec![];

        for i in 0..10 {
            let db = Arc::clone(&db);
            handles.push(thread::spawn(move || {
                db.log_translation("EN", "ZH", i + 1, 0, "success", None, "").unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let (req, chars) = db.get_current_log_totals().unwrap();
        assert_eq!(req, 10);
        // chars = 1+2+3+...+10 = 55
        assert_eq!(chars, 55);
    }

    #[test]
    fn test_open_real_old_database() {
        let src = std::path::Path::new("deeplx-monitor-旧.db");
        if !src.exists() {
            eprintln!("跳过: 旧数据库文件不存在");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join("old_copy.db");
        std::fs::copy(src, &tmp).unwrap();

        let db = Database::new(&tmp).expect("Database::new() 应能打开旧数据库");

        // 验证累计计数器
        let (req, chars) = db.get_current_log_totals().unwrap();
        assert!(req > 0, "旧数据库 anchor 应有请求数, got {}", req);
        assert!(chars > 0, "旧数据库 anchor 应有字符数, got {}", chars);

        // 验证 SUM(source_chars) 查询正常工作
        let (period_req, period_chars) = db.get_period_stats(365).unwrap();
        assert!(period_req > 0, "365天内应有请求");
        assert!(period_chars > 0, "365天内应有字符");

        // 验证分页查询
        let (items, total) = db.get_requests(1, 10).unwrap();
        assert!(total > 0);
        assert!(!items.is_empty());
        // 验证 source_chars 字段是正确的整数
        for item in &items {
            assert!(item.source_chars > 0, "source_chars 应为正整数, got {}", item.source_chars);
        }

        // 验证图表查询
        let daily = db.get_daily_stats(365).unwrap();
        assert!(!daily.is_empty(), "应有每日统计");
        for d in &daily {
            assert!(d.chars > 0, "每日字符数应 > 0");
        }

        // 验证热力图
        let heatmap = db.get_heatmap_by_weekday(365).unwrap();
        assert!(!heatmap.is_empty());

        std::mem::forget(dir);
    }
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
