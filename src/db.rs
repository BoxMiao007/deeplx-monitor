use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Mutex;

/// 翻译日志数据库，负责持久化所有翻译请求的明细和累计统计。
///
/// # 双表架构设计
///
/// - **`translation_logs`**：翻译请求明细表，每次翻译请求写入一行，包含语言对、字符数、
///   状态、延迟等信息。受日志清理策略影响，超过保留天数的记录会被定期删除。
///
/// - **`stats_anchor`**：永久累计统计表，记录总请求数、总字符数、缓存命中/未命中计数，
///   以及每个端点的请求数和成功率。此表不受日志清理影响，确保重启后统计数据不丢失。
///   支持全局行（`endpoint_name = ''`）和 per-endpoint 行。
///
/// # 线程安全
///
/// 内部使用 `Mutex<Connection>` 保证多线程环境下的安全访问。所有公开方法通过获取锁
/// 来串行化数据库操作。
pub struct Database {
    /// SQLite 连接，通过 Mutex 保证线程安全的串行访问
    conn: Mutex<Connection>,
}

impl Database {
    /// 创建并初始化数据库实例。
    ///
    /// 打开指定路径的 SQLite 数据库文件（不存在则自动创建），然后执行 `init()` 完成
    /// 表创建和数据迁移。
    ///
    /// # 参数
    /// - `path`：数据库文件路径
    ///
    /// # 返回
    /// 初始化完成的 `Database` 实例，或 SQLite 错误
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    /// 初始化数据库表结构并执行增量迁移。
    ///
    /// # 执行步骤
    ///
    /// 1. **设置 PRAGMA**：WAL 模式（并发读写）、NORMAL 同步（性能优先）、
    ///    5 秒忙等待超时、8MB 缓存。
    /// 2. **创建 `stats_anchor` 表**：永久累计统计（不受日志清理影响）。
    /// 3. **创建 `translation_logs` 表**：翻译请求明细日志。
    /// 4. **增量迁移 `translation_logs`**：逐列检查并添加 `source_chars`、`target_chars`、
    ///    `endpoint_name`、`latency_ms` 列（兼容旧版数据库）。
    /// 5. **修复 `source_chars` 文本值**：SQLite 的 `ALTER TABLE ADD COLUMN DEFAULT chars`
    ///    会将 "chars" 作为字面量文本存入旧行，此步骤将其修正为实际的 `chars` 整数值。
    /// 6. **创建索引**：加速按时间、语言、端点的查询。
    /// 7. **迁移 `stats_anchor` 为多行表**：从单行（CHECK id=1）重建为支持 per-endpoint
    ///    行的多行表，添加 `endpoint_name` 列和唯一索引。
    /// 8. **添加 `total_successes`/`latency_sum_ms` 列**：用于重启后恢复端点的成功率
    ///    和平均延迟统计，并从现有日志补种初始值。
    /// 9. **确保全局行存在**：`endpoint_name = ''` 的行存储全局累计。
    /// 10. **从日志补种 anchor**：若全局行计数为 0 但已有日志记录，从日志表回填累计值
    ///     （处理从旧版本升级的场景）。
    fn init(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;
             PRAGMA cache_size=-8000;",
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
            let cols: Vec<String> = stmt
                .query_map([], |row| row.get(1))?
                .filter_map(|r| r.ok())
                .collect();
            if !cols.iter().any(|c| c == "source_chars") {
                conn.execute("ALTER TABLE translation_logs ADD COLUMN source_chars INTEGER NOT NULL DEFAULT chars", [])?;
            }
            if !cols.iter().any(|c| c == "target_chars") {
                conn.execute("ALTER TABLE translation_logs ADD COLUMN target_chars INTEGER NOT NULL DEFAULT 0", [])?;
            }
            if !cols.iter().any(|c| c == "endpoint_name") {
                conn.execute("ALTER TABLE translation_logs ADD COLUMN endpoint_name TEXT NOT NULL DEFAULT ''", [])?;
            }
            if !cols.iter().any(|c| c == "latency_ms") {
                conn.execute(
                    "ALTER TABLE translation_logs ADD COLUMN latency_ms INTEGER",
                    [],
                )?;
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
            let anchor_cols: Vec<String> = stmt
                .query_map([], |row| row.get(1))?
                .filter_map(|r| r.ok())
                .collect();
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

        // 迁移：添加 total_successes 和 latency_sum_ms 列（用于重启后恢复端点统计）
        {
            let mut stmt = conn.prepare_cached("PRAGMA table_info(stats_anchor)")?;
            let anchor_cols: Vec<String> = stmt
                .query_map([], |row| row.get(1))?
                .filter_map(|r| r.ok())
                .collect();
            if !anchor_cols.iter().any(|c| c == "total_successes") {
                conn.execute("ALTER TABLE stats_anchor ADD COLUMN total_successes INTEGER NOT NULL DEFAULT 0", [])?;
                conn.execute(
                    "ALTER TABLE stats_anchor ADD COLUMN latency_sum_ms INTEGER NOT NULL DEFAULT 0",
                    [],
                )?;
                // 从现有日志补种
                conn.execute(
                    "UPDATE stats_anchor SET
                        total_successes = COALESCE((SELECT COUNT(*) FROM translation_logs WHERE status = 'success' AND translation_logs.endpoint_name = stats_anchor.endpoint_name), 0),
                        latency_sum_ms = COALESCE((SELECT SUM(latency_ms) FROM translation_logs WHERE status = 'success' AND latency_ms IS NOT NULL AND translation_logs.endpoint_name = stats_anchor.endpoint_name), 0)
                     WHERE endpoint_name != ''",
                    [],
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
        let (anchor_req, _): (i64, i64) = conn
            .prepare_cached(
                "SELECT total_requests, total_chars FROM stats_anchor WHERE endpoint_name = ''",
            )?
            .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        if anchor_req == 0 {
            let (log_count, log_chars): (i64, i64) = conn
                .prepare_cached(
                    "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs",
                )?
                .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            if log_count > 0 {
                conn.execute(
                    "UPDATE stats_anchor SET total_requests = ?1, total_chars = ?2 WHERE endpoint_name = ''",
                    params![log_count, log_chars],
                )?;
            }
        }

        // 创建 hourly_stats 汇总表：按 (小时桶, 端点) 维度聚合的图表数据源
        // 此表永久保留，不受 cleanup_old_logs 影响，确保日志清理后图表数据仍完整
        conn.execute(
            "CREATE TABLE IF NOT EXISTS hourly_stats (
                bucket_hour TEXT NOT NULL,
                endpoint_name TEXT NOT NULL DEFAULT '',
                request_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                source_chars_sum INTEGER NOT NULL DEFAULT 0,
                target_chars_sum INTEGER NOT NULL DEFAULT 0,
                latency_sum_ms INTEGER NOT NULL DEFAULT 0,
                latency_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (bucket_hour, endpoint_name)
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hourly_stats_bucket ON hourly_stats(bucket_hour)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hourly_stats_endpoint_bucket ON hourly_stats(endpoint_name, bucket_hour)",
            [],
        )?;

        // 创建 hourly_lang_stats 表：按 (小时桶, 语言, 角色) 聚合的语言维度数据源
        // role 区分 'source'（作为源语言）和 'target'（作为目标语言）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS hourly_lang_stats (
                bucket_hour TEXT NOT NULL,
                lang TEXT NOT NULL,
                role TEXT NOT NULL,
                endpoint_name TEXT NOT NULL DEFAULT '',
                request_count INTEGER NOT NULL DEFAULT 0,
                chars_sum INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (bucket_hour, lang, role, endpoint_name)
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hourly_lang_stats_bucket ON hourly_lang_stats(bucket_hour)",
            [],
        )?;

        // 迁移：若 hourly_stats 为空但 translation_logs 有数据，从现有日志全量补种
        // 同时生成全局行（endpoint_name = ''）和端点行
        let hourly_count: i64 = conn
            .prepare_cached("SELECT COUNT(*) FROM hourly_stats")?
            .query_row([], |row| row.get(0))?;
        let logs_exist: i64 = conn
            .prepare_cached("SELECT COUNT(*) FROM translation_logs LIMIT 1")?
            .query_row([], |row| row.get(0))?;
        if hourly_count == 0 && logs_exist > 0 {
            // 端点行
            conn.execute(
                "INSERT INTO hourly_stats (bucket_hour, endpoint_name, request_count, success_count, error_count, source_chars_sum, target_chars_sum, latency_sum_ms, latency_count)
                 SELECT strftime('%Y-%m-%d %H:00', created_at) AS bucket_hour,
                        endpoint_name,
                        COUNT(*) AS request_count,
                        SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success_count,
                        SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) AS error_count,
                        COALESCE(SUM(source_chars), 0) AS source_chars_sum,
                        COALESCE(SUM(target_chars), 0) AS target_chars_sum,
                        COALESCE(SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN latency_ms ELSE 0 END), 0) AS latency_sum_ms,
                        SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN 1 ELSE 0 END) AS latency_count
                 FROM translation_logs
                 GROUP BY strftime('%Y-%m-%d %H:00', created_at), endpoint_name",
                [],
            )?;
            // 全局行（endpoint_name = ''）：跨所有端点聚合
            conn.execute(
                "INSERT OR REPLACE INTO hourly_stats (bucket_hour, endpoint_name, request_count, success_count, error_count, source_chars_sum, target_chars_sum, latency_sum_ms, latency_count)
                 SELECT strftime('%Y-%m-%d %H:00', created_at) AS bucket_hour,
                        '' AS endpoint_name,
                        COUNT(*) AS request_count,
                        SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success_count,
                        SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) AS error_count,
                        COALESCE(SUM(source_chars), 0) AS source_chars_sum,
                        COALESCE(SUM(target_chars), 0) AS target_chars_sum,
                        COALESCE(SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN latency_ms ELSE 0 END), 0) AS latency_sum_ms,
                        SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN 1 ELSE 0 END) AS latency_count
                 FROM translation_logs
                 GROUP BY strftime('%Y-%m-%d %H:00', created_at)",
                [],
            )?;
        }

        // 迁移：若 hourly_lang_stats 为空但 translation_logs 有数据，从现有日志补种
        // 拆为 source 行和 target 行两部分插入
        let hourly_lang_count: i64 = conn
            .prepare_cached("SELECT COUNT(*) FROM hourly_lang_stats")?
            .query_row([], |row| row.get(0))?;
        if hourly_lang_count == 0 && logs_exist > 0 {
            // source 行（端点维度）
            conn.execute(
                "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
                 SELECT strftime('%Y-%m-%d %H:00', created_at), source_lang, 'source', endpoint_name,
                        COUNT(*), COALESCE(SUM(source_chars), 0)
                 FROM translation_logs
                 GROUP BY strftime('%Y-%m-%d %H:00', created_at), source_lang, endpoint_name",
                [],
            )?;
            // target 行（端点维度）
            conn.execute(
                "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
                 SELECT strftime('%Y-%m-%d %H:00', created_at), target_lang, 'target', endpoint_name,
                        COUNT(*), COALESCE(SUM(target_chars), 0)
                 FROM translation_logs
                 GROUP BY strftime('%Y-%m-%d %H:00', created_at), target_lang, endpoint_name",
                [],
            )?;
            // source 行（全局行：endpoint_name = ''）
            conn.execute(
                "INSERT OR REPLACE INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
                 SELECT strftime('%Y-%m-%d %H:00', created_at), source_lang, 'source', '',
                        COUNT(*), COALESCE(SUM(source_chars), 0)
                 FROM translation_logs
                 GROUP BY strftime('%Y-%m-%d %H:00', created_at), source_lang",
                [],
            )?;
            // target 行（全局行：endpoint_name = ''）
            conn.execute(
                "INSERT OR REPLACE INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
                 SELECT strftime('%Y-%m-%d %H:00', created_at), target_lang, 'target', '',
                        COUNT(*), COALESCE(SUM(target_chars), 0)
                 FROM translation_logs
                 GROUP BY strftime('%Y-%m-%d %H:00', created_at), target_lang",
                [],
            )?;
        }

        Ok(())
    }

    /// 记录一次翻译请求，同时更新累计统计（双写机制）。
    ///
    /// # 双写机制
    ///
    /// 1. **写入 `translation_logs`**：插入一条明细记录，包含完整的请求信息。
    /// 2. **更新全局 `stats_anchor`**：递增全局行的 `total_requests` 和 `total_chars`。
    /// 3. **更新端点 `stats_anchor`**：若 `endpoint_name` 非空，递增对应端点行的计数器；
    ///    若请求成功，还递增 `total_successes` 和累加 `latency_sum_ms`。
    ///
    /// 这种双写设计确保即使 `translation_logs` 被清理（按保留天数删除旧记录），
    /// `stats_anchor` 中的累计值仍然准确反映历史总量。
    ///
    /// # 参数
    /// - `source_lang`：源语言代码（如 "EN"、"ZH"）
    /// - `target_lang`：目标语言代码
    /// - `source_chars`：源文本字符数
    /// - `target_chars`：译文字符数
    /// - `status`：请求状态，"success" 或 "error"
    /// - `error_msg`：错误信息（仅 status="error" 时有值）
    /// - `endpoint_name`：处理该请求的上游端点名称
    /// - `latency_ms`：请求延迟（毫秒）
    ///
    /// # 返回
    /// 新插入记录的行 ID
    pub fn log_translation(
        &self,
        source_lang: &str,
        target_lang: &str,
        source_chars: i64,
        target_chars: i64,
        status: &str,
        error_msg: Option<&str>,
        endpoint_name: &str,
        latency_ms: Option<u64>,
    ) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, endpoint_name, latency_ms)
             VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6, ?7, ?8)",
            params![source_chars, source_lang, target_lang, target_chars, status, error_msg, endpoint_name, latency_ms],
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
            if status == "success" {
                let lat = latency_ms.unwrap_or(0) as i64;
                conn.execute(
                    "UPDATE stats_anchor SET total_successes = total_successes + 1, latency_sum_ms = latency_sum_ms + ?1 WHERE endpoint_name = ?2",
                    params![lat, endpoint_name],
                )?;
            }
        }

        // 三写 hourly_stats：按小时桶递增图表汇总数据（永久保留，不受日志清理影响）
        let bucket_hour = conn
            .prepare_cached("SELECT strftime('%Y-%m-%d %H:00', 'now', 'localtime')")?
            .query_row([], |row| row.get::<_, String>(0))?;
        let is_success = status == "success";
        let is_error = status == "error";
        let lat_val = if is_success {
            latency_ms.unwrap_or(0) as i64
        } else {
            0
        };
        let lat_cnt: i64 = if is_success && latency_ms.is_some() {
            1
        } else {
            0
        };

        // 全局行
        conn.execute(
            "INSERT INTO hourly_stats (bucket_hour, endpoint_name, request_count, success_count, error_count, source_chars_sum, target_chars_sum, latency_sum_ms, latency_count)
             VALUES (?1, '', 1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(bucket_hour, endpoint_name) DO UPDATE SET
                request_count = request_count + 1,
                success_count = success_count + ?2,
                error_count = error_count + ?3,
                source_chars_sum = source_chars_sum + ?4,
                target_chars_sum = target_chars_sum + ?5,
                latency_sum_ms = latency_sum_ms + ?6,
                latency_count = latency_count + ?7",
            params![bucket_hour, is_success as i64, is_error as i64, source_chars, target_chars, lat_val, lat_cnt],
        )?;

        // 端点行（如果 endpoint_name 非空）
        if !endpoint_name.is_empty() {
            conn.execute(
                "INSERT INTO hourly_stats (bucket_hour, endpoint_name, request_count, success_count, error_count, source_chars_sum, target_chars_sum, latency_sum_ms, latency_count)
                 VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(bucket_hour, endpoint_name) DO UPDATE SET
                    request_count = request_count + 1,
                    success_count = success_count + ?3,
                    error_count = error_count + ?4,
                    source_chars_sum = source_chars_sum + ?5,
                    target_chars_sum = target_chars_sum + ?6,
                    latency_sum_ms = latency_sum_ms + ?7,
                    latency_count = latency_count + ?8",
                params![bucket_hour, endpoint_name, is_success as i64, is_error as i64, source_chars, target_chars, lat_val, lat_cnt],
            )?;
        }

        // 三写 hourly_lang_stats：source_lang 和 target_lang 各一行
        // 全局行 source
        conn.execute(
            "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
             VALUES (?1, ?2, 'source', '', 1, ?3)
             ON CONFLICT(bucket_hour, lang, role, endpoint_name) DO UPDATE SET
                request_count = request_count + 1,
                chars_sum = chars_sum + ?3",
            params![bucket_hour, source_lang, source_chars],
        )?;
        // 全局行 target
        conn.execute(
            "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
             VALUES (?1, ?2, 'target', '', 1, ?3)
             ON CONFLICT(bucket_hour, lang, role, endpoint_name) DO UPDATE SET
                request_count = request_count + 1,
                chars_sum = chars_sum + ?3",
            params![bucket_hour, target_lang, target_chars],
        )?;
        // 端点行 source/target（如果 endpoint_name 非空）
        if !endpoint_name.is_empty() {
            conn.execute(
                "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
                 VALUES (?1, ?2, 'source', ?3, 1, ?4)
                 ON CONFLICT(bucket_hour, lang, role, endpoint_name) DO UPDATE SET
                    request_count = request_count + 1,
                    chars_sum = chars_sum + ?4",
                params![bucket_hour, source_lang, endpoint_name, source_chars],
            )?;
            conn.execute(
                "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
                 VALUES (?1, ?2, 'target', ?3, 1, ?4)
                 ON CONFLICT(bucket_hour, lang, role, endpoint_name) DO UPDATE SET
                    request_count = request_count + 1,
                    chars_sum = chars_sum + ?4",
                params![bucket_hour, target_lang, endpoint_name, target_chars],
            )?;
        }

        Ok(conn.last_insert_rowid())
    }

    /// 持久化缓存统计数据（命中数和未命中数）。
    ///
    /// 将当前内存中的缓存命中/未命中计数写入 `stats_anchor` 全局行，
    /// 以便服务重启后能恢复缓存统计显示。
    ///
    /// # 参数
    /// - `hits`：缓存命中次数
    /// - `misses`：缓存未命中次数
    pub fn save_cache_stats(&self, hits: u64, misses: u64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE stats_anchor SET cache_hits = ?1, cache_misses = ?2 WHERE endpoint_name = ''",
            params![hits as i64, misses as i64],
        )?;
        Ok(())
    }

    /// 加载持久化的缓存统计数据。
    ///
    /// 从 `stats_anchor` 全局行读取上次保存的缓存命中/未命中计数，用于服务启动时
    /// 恢复缓存统计。读取失败（表不存在或行缺失）时返回 (0, 0)。
    ///
    /// # 返回
    /// `(命中次数, 未命中次数)` 元组
    pub fn load_cache_stats(&self) -> (u64, u64) {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .prepare_cached(
                "SELECT cache_hits, cache_misses FROM stats_anchor WHERE endpoint_name = ''",
            )
            .and_then(|mut stmt| {
                stmt.query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
            });
        match result {
            Ok((hits, misses)) => (hits as u64, misses as u64),
            Err(_) => (0, 0),
        }
    }

    /// 获取全局累计统计（总请求数和总字符数）。
    ///
    /// 从 `stats_anchor` 全局行（`endpoint_name = ''`）读取，该值不受日志清理影响，
    /// 始终反映服务启动以来的历史总量。
    ///
    /// # 返回
    /// `(总请求数, 总字符数)` 元组
    pub fn get_current_log_totals(&self) -> SqliteResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT total_requests, total_chars FROM stats_anchor WHERE endpoint_name = ''",
        )?;
        stmt.query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
    }

    /// 获取指定端点的累计统计（总请求数和总字符数）。
    ///
    /// 从 `stats_anchor` 中对应端点行读取。若端点不存在则返回 (0, 0)。
    ///
    /// # 参数
    /// - `endpoint_name`：端点名称
    ///
    /// # 返回
    /// `(总请求数, 总字符数)` 元组
    pub fn get_endpoint_totals(&self, endpoint_name: &str) -> SqliteResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT total_requests, total_chars FROM stats_anchor WHERE endpoint_name = ?1",
        )?;
        stmt.query_row(params![endpoint_name], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .or(Ok((0, 0)))
    }

    /// 加载所有端点的持久化统计，用于重启后恢复 LoadBalancer 计数器。
    ///
    /// 读取 `stats_anchor` 中所有 `endpoint_name != ''` 的行，返回每个端点的
    /// 总请求数、总成功数和延迟累计值。LoadBalancer 据此恢复请求占比和平均延迟显示。
    ///
    /// # 返回
    /// `Vec<(端点名称, 总请求数, 总成功数, 延迟累计毫秒)>`
    pub fn load_endpoint_stats(&self) -> Vec<(String, u64, u64, u64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare_cached(
            "SELECT endpoint_name, total_requests, total_successes, latency_sum_ms FROM stats_anchor WHERE endpoint_name != ''"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? as u64,
                row.get::<_, i64>(2)? as u64,
                row.get::<_, i64>(3)? as u64,
            ))
        });
        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// 查询指定时间范围内的请求统计（请求数和字符数），支持按端点过滤。
    ///
    /// 从 `hourly_stats` 汇总表按 `bucket_hour` 时间范围聚合统计。该表永久保留，
    /// 不受日志清理影响，因此结果在日志清理后仍然完整。
    ///
    /// # 参数
    /// - `days`：向前查询的天数（从当前时间起算）
    /// - `endpoint`：可选的端点名称过滤，`None` 或空字符串表示查询全局行
    ///
    /// # 返回
    /// `(请求数, 字符数)` 元组
    pub fn get_period_stats_filtered(
        &self,
        days: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let mut stmt = conn.prepare_cached(
            "SELECT COALESCE(SUM(request_count), 0), COALESCE(SUM(source_chars_sum), 0) FROM hourly_stats
             WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
               AND endpoint_name = ?2"
        )?;
        stmt.query_row(params![days, ep_filter], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
    }

    /// 分页查询翻译请求日志，支持按端点和状态过滤。
    ///
    /// 结果按 `id DESC` 排序（最新的在前），支持动态组合 WHERE 条件。
    ///
    /// # 参数
    /// - `page`：页码（从 1 开始）
    /// - `page_size`：每页记录数
    /// - `endpoint`：可选的端点名称过滤
    /// - `status`：可选的状态过滤（"success" 或 "error"）
    ///
    /// # 返回
    /// `(当前页日志列表, 符合条件的总记录数)` 元组
    pub fn get_requests_filtered(
        &self,
        page: u32,
        page_size: u32,
        endpoint: Option<&str>,
        status: Option<&str>,
    ) -> SqliteResult<(Vec<RequestLog>, i64)> {
        let conn = self.conn.lock().unwrap();
        let offset = (page - 1) * page_size;
        let ep_filter = endpoint.unwrap_or("");
        let st_filter = status.unwrap_or("");

        let mut where_clauses = Vec::new();
        let mut count_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if !ep_filter.is_empty() {
            where_clauses.push("endpoint_name = ?");
            count_params.push(Box::new(ep_filter.to_string()));
        }
        if !st_filter.is_empty() {
            where_clauses.push("status = ?");
            count_params.push(Box::new(st_filter.to_string()));
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM translation_logs{}", where_sql);
        let total: i64 = {
            let mut stmt = conn.prepare(&count_sql)?;
            let params_ref: Vec<&dyn rusqlite::types::ToSql> =
                count_params.iter().map(|p| p.as_ref()).collect();
            stmt.query_row(params_ref.as_slice(), |row| row.get(0))?
        };

        let query_sql = format!(
            "SELECT id, chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at, endpoint_name, latency_ms
             FROM translation_logs{} ORDER BY id DESC LIMIT ? OFFSET ?",
            where_sql
        );

        let mut query_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if !ep_filter.is_empty() {
            query_params.push(Box::new(ep_filter.to_string()));
        }
        if !st_filter.is_empty() {
            query_params.push(Box::new(st_filter.to_string()));
        }
        query_params.push(Box::new(page_size));
        query_params.push(Box::new(offset));

        let mut logs = Vec::new();
        let mut stmt = conn.prepare(&query_sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            query_params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_ref.as_slice(), map_request_log)?;
        for row in rows {
            logs.push(row?);
        }

        Ok((logs, total))
    }

    /// 清理超过指定保留天数的旧日志记录。
    ///
    /// # 清理策略
    ///
    /// 仅删除 `translation_logs` 表中 `created_at` 早于保留期限的记录。
    /// **不影响 `stats_anchor` 表**——累计统计（总请求数、总字符数等）保持不变，
    /// 确保仪表盘上的历史总量数据在清理后依然准确。
    ///
    /// 此方法由后台定时任务调用，清理频率和保留天数由 `config.toml` 中
    /// `[monitor].log_retention_days` 配置。
    ///
    /// # 参数
    /// - `retention_days`：保留天数，超过此天数的日志将被删除
    ///
    /// # 返回
    /// 被删除的记录数
    pub fn cleanup_old_logs(&self, retention_days: u32) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();

        let deleted = conn.execute(
            "DELETE FROM translation_logs WHERE created_at < datetime('now', '-' || ?1 || ' days', 'localtime')",
            params![retention_days],
        )?;

        Ok(deleted as i64)
    }

    /// 用演示数据替换当前数据库内容（用于 Demo 模式展示仪表盘效果）。
    ///
    /// # 演示数据生成策略
    ///
    /// 1. **清空现有数据**：删除所有日志和端点统计行，重置自增序列。
    /// 2. **生成 30 天历史数据**：
    ///    - 模拟真实流量模式：凌晨低谷、上午高峰、下午平稳、晚间次高峰（`hourly_weights`）
    ///    - 流量逐日增长（`day_multiplier`），周末减少 40%
    ///    - 10 种语言对，EN→ZH 占 40%、ZH→EN 占 25%，其余分散
    ///    - 多端点差异化性能特征（快速/中等/较慢）
    ///    - 错误率随时段和端点变化（高峰 5%、凌晨 1%、慢端点错误更多）
    /// 3. **生成当天密集数据**：更细粒度的当日数据，使实时图表有内容展示。
    /// 4. **同步累计计数器**：重建 `stats_anchor` 全局行和各端点行，确保与日志一致。
    /// 5. **模拟缓存统计**：设置 35% 的缓存命中率。
    ///
    /// # 参数
    /// - `seed`：随机数种子，相同种子生成相同数据（确定性）
    /// - `endpoint_names`：端点名称列表，用于分配请求到不同端点
    pub fn replace_with_demo_data(&self, seed: u64, endpoint_names: &[String]) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM translation_logs", [])?;
        conn.execute(
            "DELETE FROM sqlite_sequence WHERE name = 'translation_logs'",
            [],
        )?;
        conn.execute(
            "UPDATE stats_anchor SET cache_hits = 0, cache_misses = 0 WHERE endpoint_name = ''",
            [],
        )?;

        // 语言对及其典型字符数范围 (source_min, source_max, target_ratio)
        let lang_profiles: &[(&str, &str, i64, i64, f64)] = &[
            ("EN", "ZH", 60, 800, 0.45), // 英译中：中文更短
            ("ZH", "EN", 30, 400, 2.2),  // 中译英：英文更长
            ("JA", "ZH", 40, 500, 0.85), // 日译中：长度相近
            ("KO", "ZH", 35, 450, 0.9),  // 韩译中
            ("FR", "ZH", 80, 900, 0.42), // 法译中
            ("DE", "ZH", 90, 1000, 0.4), // 德译中
            ("ES", "EN", 70, 750, 0.95), // 西译英
            ("RU", "ZH", 60, 600, 0.5),  // 俄译中
            ("EN", "JA", 60, 800, 1.1),  // 英译日
            ("ZH", "JA", 30, 400, 1.3),  // 中译日
        ];
        let error_messages = [
            "Upstream timeout after 10s",
            "HTTP 429 Too Many Requests",
            "TLS handshake failed",
            "Connection reset by peer",
            "Invalid JSON response from upstream",
            "DNS resolution failed",
        ];
        // 每小时流量权重（模拟真实使用模式：凌晨低谷，上午高峰，下午平稳，晚间次高峰）
        let hourly_weights: [u32; 24] = [
            2, 1, 1, 1, 2, 3, 5, 8, 12, 15, 14, 13, 11, 12, 13, 14, 12, 10, 9, 8, 7, 5, 4, 3,
        ];

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let now = now - (now % 60);
        let mut rng = SimpleRng::new(seed);
        let ep_count = endpoint_names.len().max(1);

        // 端点性能特征：(基础延迟, 延迟波动, 错误率倍数)
        let ep_profiles: Vec<(u64, u64, u32)> = (0..ep_count)
            .map(|i| match i % 3 {
                0 => (180, 120, 1), // 快速稳定
                1 => (350, 200, 2), // 中等
                _ => (500, 300, 3), // 较慢，错误多
            })
            .collect();

        let mut insert_stmt = conn.prepare(
            "INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at, endpoint_name, latency_ms)
             VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6, datetime(?7, 'unixepoch', 'localtime'), ?8, ?9)"
        )?;

        // 生成 30 天数据，流量逐渐增长
        for day_offset in 0..30_i64 {
            let day_start = now - (29 - day_offset) * 86_400;
            // 流量随时间增长：早期少，近期多
            let day_multiplier = 1.0 + (day_offset as f64 / 30.0) * 2.5;
            // 周末流量减少 40%
            let weekday = ((day_start / 86_400) % 7) as usize;
            let weekend_factor = if weekday == 5 || weekday == 6 {
                0.6
            } else {
                1.0
            };

            for hour in 0..24_u32 {
                let base_count =
                    (hourly_weights[hour as usize] as f64 * day_multiplier * weekend_factor) as u32;
                let count = base_count + rng.next_usize(3) as u32;

                for _ in 0..count {
                    let minute = rng.next_usize(60) as i64;
                    let second = rng.next_usize(60) as i64;
                    let created_at = day_start + (hour as i64) * 3600 + minute * 60 + second;

                    // 语言对选择：EN→ZH 占 40%，ZH→EN 占 25%，其余分散
                    let pair_idx = {
                        let r = rng.next_usize(100);
                        if r < 40 {
                            0
                        } else if r < 65 {
                            1
                        } else if r < 75 {
                            2
                        } else if r < 82 {
                            3
                        } else if r < 88 {
                            4
                        } else {
                            5 + rng.next_usize(lang_profiles.len() - 5)
                        }
                    };
                    let (src, tgt, src_min, src_max, tgt_ratio) =
                        lang_profiles[pair_idx % lang_profiles.len()];

                    let source_chars =
                        src_min + (rng.next_usize((src_max - src_min) as usize) as i64);
                    let target_chars = std::cmp::max(
                        10,
                        (source_chars as f64
                            * tgt_ratio
                            * (0.85 + rng.next_usize(30) as f64 / 100.0))
                            as i64,
                    );

                    // 端点选择：第一个端点承担更多流量
                    let ep_idx = if ep_count <= 1 {
                        0
                    } else {
                        let r = rng.next_usize(100);
                        if r < 55 {
                            0
                        } else if r < 85 && ep_count > 1 {
                            1
                        } else {
                            rng.next_usize(ep_count)
                        }
                    };
                    let ep_name = if endpoint_names.is_empty() {
                        ""
                    } else {
                        &endpoint_names[ep_idx]
                    };
                    let (base_lat, lat_var, err_mult) = ep_profiles[ep_idx];

                    // 延迟：基础 + 随机波动 + 文本长度影响
                    let text_factor = (source_chars as u64) / 200;
                    let latency =
                        base_lat + rng.next_usize(lat_var as usize) as u64 + text_factor * 30;

                    // 错误率：基础 3%，高峰时段 5%，凌晨 1%
                    let base_error_pct = if (9..=17).contains(&hour) {
                        5
                    } else if hour <= 5 {
                        1
                    } else {
                        3
                    };
                    let error_pct = base_error_pct * err_mult;
                    let is_error = rng.next_usize(100) < error_pct as usize;

                    let (status, error_msg, final_latency): (&str, Option<&str>, u64) = if is_error
                    {
                        let err_idx = rng.next_usize(error_messages.len());
                        let err_latency = if err_idx == 0 {
                            10000
                        } else {
                            latency + 2000 + rng.next_usize(3000) as u64
                        };
                        ("error", Some(error_messages[err_idx]), err_latency)
                    } else {
                        ("success", None, latency)
                    };

                    insert_stmt.execute(params![
                        source_chars,
                        src,
                        tgt,
                        target_chars,
                        status,
                        error_msg,
                        created_at,
                        ep_name,
                        final_latency
                    ])?;
                }
            }
        }

        // 今天的密集数据（更细粒度）
        let today_start = now - (now % 86_400);
        let current_hour = ((now - today_start) / 3600) as u32;
        for hour in 0..=current_hour {
            let extra = (hourly_weights[hour as usize] as f64 * 1.8) as u32;
            for _ in 0..extra {
                let minute = rng.next_usize(60) as i64;
                let second = rng.next_usize(60) as i64;
                let created_at = today_start + (hour as i64) * 3600 + minute * 60 + second;
                if created_at > now {
                    continue;
                }

                let pair_idx = rng.next_usize(lang_profiles.len());
                let (src, tgt, src_min, src_max, tgt_ratio) = lang_profiles[pair_idx];
                let source_chars = src_min + (rng.next_usize((src_max - src_min) as usize) as i64);
                let target_chars = std::cmp::max(10, (source_chars as f64 * tgt_ratio) as i64);

                let ep_idx = if ep_count <= 1 {
                    0
                } else {
                    rng.next_usize(ep_count)
                };
                let ep_name = if endpoint_names.is_empty() {
                    ""
                } else {
                    &endpoint_names[ep_idx]
                };
                let (base_lat, lat_var, _) = ep_profiles[ep_idx];
                let latency = base_lat + rng.next_usize(lat_var as usize) as u64;

                let is_error = rng.next_usize(100) < 4;
                let (status, error_msg, final_latency): (&str, Option<&str>, u64) = if is_error {
                    (
                        "error",
                        Some(error_messages[rng.next_usize(error_messages.len())]),
                        latency + 5000,
                    )
                } else {
                    ("success", None, latency)
                };

                insert_stmt.execute(params![
                    source_chars,
                    src,
                    tgt,
                    target_chars,
                    status,
                    error_msg,
                    created_at,
                    ep_name,
                    final_latency
                ])?;
            }
        }

        drop(insert_stmt);

        // 同步累计计数器
        let (count, chars): (i64, i64) = conn
            .prepare_cached(
                "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs",
            )?
            .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        conn.execute(
            "UPDATE stats_anchor SET total_requests = ?1, total_chars = ?2 WHERE endpoint_name = ''",
            params![count, chars],
        )?;
        // 清除旧端点行并重建
        conn.execute("DELETE FROM stats_anchor WHERE endpoint_name != ''", [])?;
        for ep in endpoint_names {
            let (ep_count_val, ep_chars): (i64, i64) = conn.prepare(
                "SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs WHERE endpoint_name = ?1"
            )?.query_row(params![ep], |row| Ok((row.get(0)?, row.get(1)?)))?;
            conn.execute(
                "INSERT OR IGNORE INTO stats_anchor (total_requests, total_chars, anchor_date, endpoint_name) VALUES (?1, ?2, '', ?3)",
                params![ep_count_val, ep_chars, ep],
            )?;
        }

        // 设置缓存统计（模拟 35% 命中率）
        let cache_hits = count * 35 / 100;
        let cache_misses = count - cache_hits;
        conn.execute(
            "UPDATE stats_anchor SET cache_hits = ?1, cache_misses = ?2 WHERE endpoint_name = ''",
            params![cache_hits, cache_misses],
        )?;

        // 清空并从 translation_logs 重建汇总表（与 init() 中的迁移补种逻辑相同）
        // 演示数据生成完毕后，图表查询读取的 hourly_stats / hourly_lang_stats 必须保持一致
        conn.execute("DELETE FROM hourly_stats", [])?;
        conn.execute("DELETE FROM hourly_lang_stats", [])?;

        // hourly_stats 端点行
        conn.execute(
            "INSERT INTO hourly_stats (bucket_hour, endpoint_name, request_count, success_count, error_count, source_chars_sum, target_chars_sum, latency_sum_ms, latency_count)
             SELECT strftime('%Y-%m-%d %H:00', created_at) AS bucket_hour,
                    endpoint_name,
                    COUNT(*) AS request_count,
                    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success_count,
                    SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) AS error_count,
                    COALESCE(SUM(source_chars), 0) AS source_chars_sum,
                    COALESCE(SUM(target_chars), 0) AS target_chars_sum,
                    COALESCE(SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN latency_ms ELSE 0 END), 0) AS latency_sum_ms,
                    SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN 1 ELSE 0 END) AS latency_count
             FROM translation_logs
             GROUP BY strftime('%Y-%m-%d %H:00', created_at), endpoint_name",
            [],
        )?;
        // hourly_stats 全局行（endpoint_name = ''）：跨所有端点聚合
        conn.execute(
            "INSERT OR REPLACE INTO hourly_stats (bucket_hour, endpoint_name, request_count, success_count, error_count, source_chars_sum, target_chars_sum, latency_sum_ms, latency_count)
             SELECT strftime('%Y-%m-%d %H:00', created_at) AS bucket_hour,
                    '' AS endpoint_name,
                    COUNT(*) AS request_count,
                    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success_count,
                    SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) AS error_count,
                    COALESCE(SUM(source_chars), 0) AS source_chars_sum,
                    COALESCE(SUM(target_chars), 0) AS target_chars_sum,
                    COALESCE(SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN latency_ms ELSE 0 END), 0) AS latency_sum_ms,
                    SUM(CASE WHEN status = 'success' AND latency_ms IS NOT NULL THEN 1 ELSE 0 END) AS latency_count
             FROM translation_logs
             GROUP BY strftime('%Y-%m-%d %H:00', created_at)",
            [],
        )?;

        // hourly_lang_stats source 行（端点维度）
        conn.execute(
            "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
             SELECT strftime('%Y-%m-%d %H:00', created_at), source_lang, 'source', endpoint_name,
                    COUNT(*), COALESCE(SUM(source_chars), 0)
             FROM translation_logs
             GROUP BY strftime('%Y-%m-%d %H:00', created_at), source_lang, endpoint_name",
            [],
        )?;
        // hourly_lang_stats target 行（端点维度）
        conn.execute(
            "INSERT INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
             SELECT strftime('%Y-%m-%d %H:00', created_at), target_lang, 'target', endpoint_name,
                    COUNT(*), COALESCE(SUM(target_chars), 0)
             FROM translation_logs
             GROUP BY strftime('%Y-%m-%d %H:00', created_at), target_lang, endpoint_name",
            [],
        )?;
        // hourly_lang_stats source 行（全局行：endpoint_name = ''）
        conn.execute(
            "INSERT OR REPLACE INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
             SELECT strftime('%Y-%m-%d %H:00', created_at), source_lang, 'source', '',
                    COUNT(*), COALESCE(SUM(source_chars), 0)
             FROM translation_logs
             GROUP BY strftime('%Y-%m-%d %H:00', created_at), source_lang",
            [],
        )?;
        // hourly_lang_stats target 行（全局行：endpoint_name = ''）
        conn.execute(
            "INSERT OR REPLACE INTO hourly_lang_stats (bucket_hour, lang, role, endpoint_name, request_count, chars_sum)
             SELECT strftime('%Y-%m-%d %H:00', created_at), target_lang, 'target', '',
                    COUNT(*), COALESCE(SUM(target_chars), 0)
             FROM translation_logs
             GROUP BY strftime('%Y-%m-%d %H:00', created_at), target_lang",
            [],
        )?;

        Ok(())
    }

    /// 查询按小时聚合的统计数据，支持按端点过滤。
    ///
    /// 从 `hourly_stats` 汇总表直接读取每小时的请求数、字符数、成功数和平均延迟。
    /// 用于仪表盘的小时级趋势图表。
    ///
    /// # 参数
    /// - `hours`：向前查询的小时数
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 按时间升序排列的小时统计列表
    pub fn get_hourly_stats_filtered(
        &self,
        hours: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<Vec<HourlyStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let mut stmt = conn.prepare_cached(
            "SELECT bucket_hour,
                    request_count,
                    source_chars_sum,
                    success_count,
                    COALESCE(latency_sum_ms * 1.0 / NULLIF(latency_count, 0), 0.0) as avg_latency
             FROM hourly_stats
             WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' hours', 'localtime'))
               AND endpoint_name = ?2
             ORDER BY bucket_hour"
        )?;
        let rows = stmt.query_map(params![hours, ep_filter], |row| {
            Ok(HourlyStat {
                hour: row.get(0)?,
                count: row.get(1)?,
                chars: row.get(2)?,
                successes: row.get(3)?,
                avg_latency_ms: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    /// 查询语言使用统计（全量），支持按端点过滤。
    ///
    /// 从 `hourly_lang_stats` 汇总表读取每种语言作为源语言（role='source'）和
    /// 目标语言（role='target'）时的字符数总量，按总字符数降序排列。
    ///
    /// # 参数
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 按总字符数降序排列的语言统计列表
    pub fn get_lang_stats_filtered(&self, endpoint: Option<&str>) -> SqliteResult<Vec<LangStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let query = r#"
            SELECT lang,
                   COALESCE(SUM(CASE WHEN role = 'source' THEN chars_sum ELSE 0 END), 0) AS source_chars,
                   COALESCE(SUM(CASE WHEN role = 'target' THEN chars_sum ELSE 0 END), 0) AS target_chars
            FROM hourly_lang_stats
            WHERE endpoint_name = ?1
            GROUP BY lang
            ORDER BY (source_chars + target_chars) DESC
        "#;
        let mut stmt = conn.prepare_cached(query)?;
        let rows = stmt.query_map(params![ep_filter], |row| {
            Ok(LangStat {
                lang: row.get(0)?,
                source_chars: row.get(1)?,
                target_chars: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// 查询指定天数内的语言使用统计，支持按端点过滤。
    ///
    /// 从 `hourly_lang_stats` 汇总表读取，增加了时间范围限制。
    ///
    /// # 参数
    /// - `days`：向前查询的天数
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 按总字符数降序排列的语言统计列表
    pub fn get_lang_stats_by_days_filtered(
        &self,
        days: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<Vec<LangStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let query = r#"
            SELECT lang,
                   COALESCE(SUM(CASE WHEN role = 'source' THEN chars_sum ELSE 0 END), 0) AS source_chars,
                   COALESCE(SUM(CASE WHEN role = 'target' THEN chars_sum ELSE 0 END), 0) AS target_chars
            FROM hourly_lang_stats
            WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
              AND endpoint_name = ?2
            GROUP BY lang
            ORDER BY (source_chars + target_chars) DESC
        "#;
        let mut stmt = conn.prepare_cached(query)?;
        let rows = stmt.query_map(params![days, ep_filter], |row| {
            Ok(LangStat {
                lang: row.get(0)?,
                source_chars: row.get(1)?,
                target_chars: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// 查询最近 24 小时内各语言的每小时使用量。
    ///
    /// 从 `hourly_lang_stats` 汇总表读取全局行（endpoint_name=''），按小时和语言聚合
    /// source/target 两种角色的请求计数。用于仪表盘的语言趋势堆叠图。
    /// 结果按 `(hour, lang)` 排序。
    ///
    /// # 返回
    /// 每小时每语言的使用次数列表
    pub fn get_lang_hourly_stats(&self) -> SqliteResult<Vec<LangHourlyUsage>> {
        let conn = self.conn.lock().unwrap();
        let query = r#"
            SELECT bucket_hour AS hour, lang, SUM(request_count) AS count
            FROM hourly_lang_stats
            WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-24 hours', 'localtime'))
              AND endpoint_name = ''
            GROUP BY bucket_hour, lang
            ORDER BY bucket_hour, lang
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

    /// 查询指定天数内各语言的每日使用量。
    ///
    /// 从 `hourly_lang_stats` 汇总表读取全局行，按天和语言聚合请求计数。
    /// 用于仪表盘的语言日趋势图。字段名复用 `LangHourlyUsage`，其中 `hour` 字段
    /// 实际存储日期字符串。
    ///
    /// # 参数
    /// - `days`：向前查询的天数
    ///
    /// # 返回
    /// 每日每语言的使用次数列表
    pub fn get_lang_daily_stats(&self, days: u32) -> SqliteResult<Vec<LangHourlyUsage>> {
        let conn = self.conn.lock().unwrap();
        let query = r#"
            SELECT substr(bucket_hour, 1, 10) AS hour, lang, SUM(request_count) AS count
            FROM hourly_lang_stats
            WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
              AND endpoint_name = ''
            GROUP BY substr(bucket_hour, 1, 10), lang
            ORDER BY hour, lang
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

    /// 查询按天聚合的统计数据，支持按端点过滤。
    ///
    /// 从 `hourly_stats` 汇总表按天分桶（`substr(bucket_hour, 1, 10)`），统计每天的
    /// 请求数、字符数、成功数和平均延迟。用于仪表盘的日级趋势图表。
    ///
    /// # 参数
    /// - `days`：向前查询的天数
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 按日期升序排列的每日统计列表
    pub fn get_daily_stats_filtered(
        &self,
        days: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<Vec<DailyStat>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let mut stmt = conn.prepare_cached(
            "SELECT substr(bucket_hour, 1, 10) as day,
                    SUM(request_count) as count,
                    SUM(source_chars_sum) as chars,
                    SUM(success_count) as successes,
                    COALESCE(SUM(latency_sum_ms) * 1.0 / NULLIF(SUM(latency_count), 0), 0.0) as avg_latency
             FROM hourly_stats
             WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
               AND endpoint_name = ?2
             GROUP BY day
             ORDER BY day"
        )?;
        let rows = stmt.query_map(params![days, ep_filter], |row| {
            Ok(DailyStat {
                day: row.get(0)?,
                count: row.get(1)?,
                chars: row.get(2)?,
                successes: row.get(3)?,
                avg_latency_ms: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    /// 查询按星期几×小时聚合的热力图数据，支持按端点过滤。
    ///
    /// 从 `hourly_stats` 汇总表读取，X 轴为星期几（周日~周六），Y 轴为小时（0~23），
    /// 值为该时段的请求数。用于仪表盘的"一周流量热力图"展示。
    ///
    /// # 参数
    /// - `days`：向前查询的天数（决定数据覆盖范围）
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 热力图单元格列表，每个单元格包含 (星期名, 小时, 请求数)
    pub fn get_heatmap_by_weekday_filtered(
        &self,
        days: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<Vec<HeatmapCell>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let mut stmt = conn.prepare_cached(
            "SELECT CAST(strftime('%w', bucket_hour) AS INTEGER) as weekday,
                    CAST(strftime('%H', bucket_hour) AS INTEGER) as hour,
                    SUM(request_count) as count
             FROM hourly_stats
             WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
               AND endpoint_name = ?2
             GROUP BY weekday, hour
             ORDER BY weekday, hour"
        )?;
        let rows = stmt.query_map(params![days, ep_filter], |row| {
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

    /// 查询按日期×小时聚合的热力图数据，支持按端点过滤。
    ///
    /// 从 `hourly_stats` 汇总表读取，X 轴为具体日期（`YYYY-MM-DD`），Y 轴为小时（0~23），
    /// 值为该时段的请求数。用于仪表盘的"日期流量热力图"展示。
    ///
    /// # 参数
    /// - `days`：向前查询的天数
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 热力图单元格列表，每个单元格包含 (日期, 小时, 请求数)
    pub fn get_heatmap_by_date_filtered(
        &self,
        days: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<Vec<HeatmapCell>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let mut stmt = conn.prepare_cached(
            "SELECT substr(bucket_hour, 1, 10) as day,
                    CAST(strftime('%H', bucket_hour) AS INTEGER) as hour,
                    SUM(request_count) as count
             FROM hourly_stats
             WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
               AND endpoint_name = ?2
             GROUP BY day, hour
             ORDER BY day, hour"
        )?;
        let rows = stmt.query_map(params![days, ep_filter], |row| {
            Ok(HeatmapCell {
                x: row.get(0)?,
                y: row.get(1)?,
                count: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// 查询按小时聚合的错误趋势数据，支持按端点过滤。
    ///
    /// 从 `hourly_stats` 汇总表读取每小时的总请求数、错误数和错误率。
    /// 用于仪表盘的"错误趋势"折线图。
    ///
    /// # 参数
    /// - `days`：向前查询的天数
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 按时间升序排列的错误趋势数据点列表
    pub fn get_error_trend_hourly_filtered(
        &self,
        days: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<Vec<ErrorTrendPoint>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<ErrorTrendPoint> {
            let total: i64 = row.get(1)?;
            let errors: i64 = row.get(2)?;
            let error_rate = if total > 0 {
                errors as f64 / total as f64
            } else {
                0.0
            };
            Ok(ErrorTrendPoint {
                time: row.get(0)?,
                total,
                errors,
                error_rate,
            })
        };
        let mut stmt = conn.prepare_cached(
            "SELECT bucket_hour,
                    SUM(request_count) as total,
                    SUM(error_count) as errors
             FROM hourly_stats
             WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
               AND endpoint_name = ?2
             GROUP BY bucket_hour
             ORDER BY bucket_hour"
        )?;
        let rows = stmt.query_map(params![days, ep_filter], map_row)?;
        rows.collect()
    }

    /// 查询按天聚合的错误趋势数据，支持按端点过滤。
    ///
    /// 从 `hourly_stats` 汇总表按天分桶，统计每天的总请求数、错误数和错误率。
    /// 适用于较长时间范围的错误趋势展示。
    ///
    /// # 参数
    /// - `days`：向前查询的天数
    /// - `endpoint`：可选的端点名称过滤
    ///
    /// # 返回
    /// 按时间升序排列的错误趋势数据点列表
    pub fn get_error_trend_daily_filtered(
        &self,
        days: u32,
        endpoint: Option<&str>,
    ) -> SqliteResult<Vec<ErrorTrendPoint>> {
        let conn = self.conn.lock().unwrap();
        let ep_filter = endpoint.unwrap_or("");
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<ErrorTrendPoint> {
            let total: i64 = row.get(1)?;
            let errors: i64 = row.get(2)?;
            let error_rate = if total > 0 {
                errors as f64 / total as f64
            } else {
                0.0
            };
            Ok(ErrorTrendPoint {
                time: row.get(0)?,
                total,
                errors,
                error_rate,
            })
        };
        let mut stmt = conn.prepare_cached(
            "SELECT substr(bucket_hour, 1, 10) as time_bucket,
                    SUM(request_count) as total,
                    SUM(error_count) as errors
             FROM hourly_stats
             WHERE bucket_hour >= strftime('%Y-%m-%d %H:00', datetime('now', '-' || ?1 || ' days', 'localtime'))
               AND endpoint_name = ?2
             GROUP BY time_bucket
             ORDER BY time_bucket"
        )?;
        let rows = stmt.query_map(params![days, ep_filter], map_row)?;
        rows.collect()
    }

    /// 导出翻译日志，支持按时间范围、语言和状态过滤。
    ///
    /// 动态构建 WHERE 条件，最多返回 10000 条记录（防止内存溢出）。
    /// 用于数据导出功能（CSV/JSON 下载）。
    ///
    /// # 参数
    /// - `start`：起始时间（ISO 格式字符串），`None` 表示不限
    /// - `end`：结束时间（ISO 格式字符串），`None` 表示不限
    /// - `lang`：语言过滤（匹配源语言或目标语言），`None` 表示不限
    /// - `status`：状态过滤（"success" 或 "error"），`None` 表示不限
    ///
    /// # 返回
    /// 符合条件的日志列表（按 id DESC 排序，最多 10000 条）
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
        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params.as_slice(), map_request_log)?;
        rows.collect()
    }

    /// 查询时间线数据，将指定天数的时间范围均匀划分为 672 个时间桶。
    ///
    /// # 时间桶算法
    ///
    /// 1. 将 `days` 天的总秒数均匀划分为 672 个桶（`total_blocks`）。
    ///    672 = 7天 × 24小时 × 4（每小时4个15分钟段），适合在 UI 上展示为紧凑的活动条。
    /// 2. 使用 SQL 的 `GROUP BY` 按桶编号聚合，桶编号 = `(now - created_at) / bucket_seconds`。
    /// 3. 将每条记录的时间戳映射到对应的桶索引（0 = 最早，671 = 最新）。
    /// 4. 返回非空桶的 `(index, count)` 列表，前端据此渲染活动密度条。
    ///
    /// # 参数
    /// - `days`：时间范围天数
    ///
    /// # 返回
    /// 非空时间桶列表，每个桶包含索引和请求数
    pub fn get_timeline_data(&self, days: u32) -> SqliteResult<Vec<TimelineBlock>> {
        let conn = self.conn.lock().unwrap();
        let total_blocks: u32 = 672;
        let total_seconds = days as f64 * 86400.0;
        let bucket_seconds = total_seconds / total_blocks as f64;

        let mut stmt = conn.prepare_cached(
            "SELECT CAST(strftime('%s', created_at) AS REAL) as ts,
                    COUNT(*) as count
             FROM translation_logs
             WHERE created_at >= datetime('now', '-' || ?1 || ' days', 'localtime')
             GROUP BY CAST((CAST(strftime('%s', 'now', 'localtime') AS REAL) - CAST(strftime('%s', created_at) AS REAL)) / ?2 AS INTEGER)
             ORDER BY ts"
        )?;

        let now_ts: f64 = conn.query_row(
            "SELECT CAST(strftime('%s', 'now', 'localtime') AS REAL)",
            [],
            |row| row.get(0),
        )?;
        let window_start = now_ts - total_seconds;

        let rows = stmt.query_map(params![days, bucket_seconds], |row| {
            let ts: f64 = row.get(0)?;
            let count: i64 = row.get(1)?;
            let offset = ts - window_start;
            let index = ((offset / bucket_seconds) as i64).clamp(0, total_blocks as i64 - 1) as u32;
            Ok(TimelineBlock { index, count })
        })?;
        rows.collect()
    }
}

/// 将 SQLite 行映射为 `RequestLog` 结构体的辅助函数。
///
/// 期望列顺序：id, chars, source_lang, target_lang, source_chars, target_chars,
/// status, error_msg, created_at, endpoint_name, latency_ms。
/// 其中 `endpoint_name` 和 `latency_ms` 使用 `.ok()` 容错（兼容旧查询不含这些列的情况）。
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
        endpoint_name: row.get(9).ok(),
        latency_ms: row.get(10).ok(),
    })
}

/// 翻译请求日志记录，对应 `translation_logs` 表的一行。
///
/// 用于分页查询结果和数据导出，包含单次翻译请求的完整信息。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RequestLog {
    /// 记录唯一 ID（自增主键）
    pub id: i64,
    /// 字符数（历史兼容字段，与 source_chars 相同）
    pub chars: i64,
    /// 源语言代码（如 "EN"、"ZH"、"JA"）
    pub source_lang: String,
    /// 目标语言代码
    pub target_lang: String,
    /// 源文本字符数
    pub source_chars: i64,
    /// 译文字符数
    pub target_chars: i64,
    /// 请求状态："success" 或 "error"
    pub status: String,
    /// 错误信息（仅 status="error" 时有值）
    pub error_msg: Option<String>,
    /// 创建时间（本地时间，格式 "YYYY-MM-DD HH:MM:SS"）
    pub created_at: String,
    /// 处理该请求的上游端点名称（旧记录可能为 None）
    pub endpoint_name: Option<String>,
    /// 请求延迟（毫秒，旧记录可能为 None）
    pub latency_ms: Option<i64>,
}

/// 小时级聚合统计，用于仪表盘的小时趋势图表。
#[derive(Debug, Clone, serde::Serialize)]
pub struct HourlyStat {
    /// 小时桶标识（格式 "YYYY-MM-DD HH:00"）
    pub hour: String,
    /// 该小时内的请求总数
    pub count: i64,
    /// 该小时内的源字符总数
    pub chars: i64,
    /// 该小时内的成功请求数
    pub successes: i64,
    /// 该小时内的平均延迟（毫秒）
    pub avg_latency_ms: f64,
}

/// 语言使用统计，用于仪表盘的语言分布图表。
#[derive(Debug, Clone, serde::Serialize)]
pub struct LangStat {
    /// 语言代码
    pub lang: String,
    /// 该语言作为源语言时的字符数总和
    pub source_chars: i64,
    /// 该语言作为目标语言时的字符数总和
    pub target_chars: i64,
}

/// 语言时间序列使用量，用于堆叠图（小时或日粒度）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct LangHourlyUsage {
    /// 时间桶标识（小时粒度时为 "YYYY-MM-DD HH:00"，日粒度时为 "YYYY-MM-DD"）
    pub hour: String,
    /// 语言代码
    pub lang: String,
    /// 该时段该语言的使用次数
    pub count: i64,
}

/// 日级聚合统计，用于仪表盘的日趋势图表。
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyStat {
    /// 日期标识（格式 "YYYY-MM-DD"）
    pub day: String,
    /// 该日的请求总数
    pub count: i64,
    /// 该日的源字符总数
    pub chars: i64,
    /// 该日的成功请求数
    pub successes: i64,
    /// 该日的平均延迟（毫秒）
    pub avg_latency_ms: f64,
}

/// 热力图单元格，用于二维分布展示（如星期×小时、日期×小时）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeatmapCell {
    /// X 轴标识（如星期名 "周一" 或日期字符串）
    pub x: String,
    /// Y 轴值（如小时 0~23）
    pub y: u32,
    /// 该单元格的请求数
    pub count: i64,
}

/// 错误趋势数据点，用于错误率折线图。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorTrendPoint {
    /// 时间桶标识（小时或日粒度）
    pub time: String,
    /// 该时段总请求数
    pub total: i64,
    /// 该时段错误请求数
    pub errors: i64,
    /// 错误率（errors / total，total=0 时为 0.0）
    pub error_rate: f64,
}

/// 时间线活动块，用于紧凑的活动密度条展示。
///
/// 时间范围被均匀划分为 672 个桶，每个桶对应一个 `TimelineBlock`。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineBlock {
    /// 桶索引（0 = 最早，671 = 最新）
    pub index: u32,
    /// 该桶内的请求数
    pub count: i64,
}

/// 轻量级确定性伪随机数生成器（用于演示数据生成，避免引入 rand 依赖）。
///
/// # 算法
///
/// 线性同余生成器（Linear Congruential Generator, LCG），公式：
/// `state = state * 6364136223846793005 + 1`
///
/// 乘数和增量参数来自 Knuth 的 MMIX 推荐值，具有完整的 2^64 周期。
/// 不适用于密码学场景，但对演示数据生成的随机性要求完全足够。
///
/// # 确定性
///
/// 相同的 seed 总是产生相同的序列，确保演示数据可复现。
struct SimpleRng {
    /// 当前状态（即上一次生成的随机数）
    state: u64,
}

impl SimpleRng {
    /// 创建新的 RNG 实例。
    ///
    /// # 参数
    /// - `seed`：随机数种子，最小为 1（避免全零状态退化）
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    /// 生成下一个 u64 随机数（LCG 迭代一步）。
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    /// 生成 [0, modulo) 范围内的随机 usize。
    ///
    /// # 参数
    /// - `modulo`：上界（不含），结果在 [0, modulo) 范围内
    fn next_usize(&mut self, modulo: usize) -> usize {
        (self.next_u64() % modulo as u64) as usize
    }
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
        db.log_translation("EN", "ZH", 100, 80, "success", None, "", None)
            .unwrap();
        let (req, chars) = db.get_current_log_totals().unwrap();
        assert_eq!(req, 1);
        assert_eq!(chars, 100);

        db.log_translation("EN", "ZH", 50, 40, "success", None, "", None)
            .unwrap();
        let (req, chars) = db.get_current_log_totals().unwrap();
        assert_eq!(req, 2);
        assert_eq!(chars, 150);
    }

    #[test]
    fn test_cleanup_deletes_old_logs_only() {
        let db = temp_db();
        {
            let conn = db.conn.lock().unwrap();
            // 插入 5 条 10 天前的旧记录
            for _ in 0..5 {
                conn.execute(
                    "INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at, endpoint_name, latency_ms)
                     VALUES (10, 'EN', 'ZH', 10, 8, 'success', NULL, datetime('now', '-10 days', 'localtime'), '', NULL)",
                    [],
                ).unwrap();
            }
            // 插入 3 条今天的新记录
            for _ in 0..3 {
                conn.execute(
                    "INSERT INTO translation_logs (chars, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at, endpoint_name, latency_ms)
                     VALUES (20, 'EN', 'ZH', 20, 16, 'success', NULL, datetime('now', 'localtime'), '', NULL)",
                    [],
                ).unwrap();
            }
        }

        // 保留最近 7 天 → 10 天前的 5 条应被删除，今天的 3 条保留
        let deleted = db.cleanup_old_logs(7).unwrap();
        assert_eq!(deleted, 5);

        let conn = db.conn.lock().unwrap();
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM translation_logs", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(remaining, 3, "应保留 3 条今天的日志");
    }

    #[test]
    fn test_cleanup_does_not_decrease_totals() {
        let db = temp_db();
        for i in 0..20 {
            db.log_translation("EN", "ZH", 10 + i, 8, "success", None, "", None)
                .unwrap();
        }
        let (req_before, chars_before) = db.get_current_log_totals().unwrap();
        assert_eq!(req_before, 20);

        // 把其中 15 条手动改为 40 天前
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE translation_logs SET created_at = datetime('now', '-40 days', 'localtime')
                 WHERE id IN (SELECT id FROM translation_logs ORDER BY id ASC LIMIT 15)",
                [],
            )
            .unwrap();
        }

        // 保留 30 天 → 删除超过 30 天的 15 条
        let deleted = db.cleanup_old_logs(30).unwrap();
        assert_eq!(deleted, 15);

        let (req_after, chars_after) = db.get_current_log_totals().unwrap();
        assert_eq!(req_after, req_before, "总请求数不应因清理而减少");
        assert_eq!(chars_after, chars_before, "总字符数不应因清理而减少");
    }

    #[test]
    fn test_cleanup_no_op_when_all_logs_within_retention() {
        let db = temp_db();
        for _ in 0..5 {
            db.log_translation("EN", "ZH", 10, 8, "success", None, "", None)
                .unwrap();
        }
        // 全部是今天的记录，保留 30 天 → 不删除任何记录
        let deleted = db.cleanup_old_logs(30).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_get_period_stats() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "", None)
            .unwrap();
        db.log_translation("EN", "ZH", 200, 160, "error", Some("timeout"), "", None)
            .unwrap();

        let (count, chars) = db.get_period_stats_filtered(1, None).unwrap();
        assert_eq!(count, 2);
        assert_eq!(chars, 300);
    }

    #[test]
    fn test_get_requests_pagination() {
        let db = temp_db();
        for i in 0..10 {
            db.log_translation("EN", "ZH", i + 1, 0, "success", None, "", None)
                .unwrap();
        }

        let (items, total) = db.get_requests_filtered(1, 5, None, None).unwrap();
        assert_eq!(total, 10);
        assert_eq!(items.len(), 5);
        // 按 id DESC 排序，第一页应该是最新的
        assert_eq!(items[0].id, 10);
        assert_eq!(items[4].id, 6);

        let (items, _) = db.get_requests_filtered(2, 5, None, None).unwrap();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].id, 5);
    }

    #[test]
    fn test_error_logging() {
        let db = temp_db();
        db.log_translation(
            "EN",
            "ZH",
            50,
            0,
            "error",
            Some("upstream timeout"),
            "",
            None,
        )
        .unwrap();

        let (items, _) = db.get_requests_filtered(1, 50, None, None).unwrap();
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
        db.log_translation("EN", "ZH", 100, 80, "success", None, "", None)
            .unwrap();
        let (req_before, _) = db.get_current_log_totals().unwrap();
        assert_eq!(req_before, 1);

        // demo 数据替换后，anchor 应该反映新的日志数量
        db.replace_with_demo_data(12345, &["ep1".to_string(), "ep2".to_string()])
            .unwrap();
        let (req_after, chars_after) = db.get_current_log_totals().unwrap();
        assert!(req_after > 0, "demo 数据后 anchor 应该 > 0");

        // 验证 anchor 与实际日志一致
        let conn = db.conn.lock().unwrap();
        let (log_count, log_chars): (i64, i64) = conn
            .prepare_cached("SELECT COUNT(*), COALESCE(SUM(source_chars), 0) FROM translation_logs")
            .unwrap()
            .query_row([], |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())))
            .unwrap();
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
        db.log_translation("EN", "ZH", 100, 80, "success", None, "", None)
            .unwrap();
        db.log_translation("JA", "ZH", 50, 40, "error", Some("fail"), "", None)
            .unwrap();
        db.log_translation("EN", "ZH", 200, 160, "success", None, "", None)
            .unwrap();

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
        db.log_translation("EN", "ZH", 100, 80, "success", None, "", None)
            .unwrap();
        let data = db.get_heatmap_by_weekday_filtered(30, None).unwrap();
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
            db.log_translation("EN", "ZH", *chars, 0, "success", None, "", None)
                .unwrap();
        }

        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT chars, source_chars FROM translation_logs ORDER BY id")
            .unwrap();
        let rows: Vec<(i64, i64)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0).unwrap(), row.get::<_, i64>(1).unwrap()))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        for (i, (chars_col, source_chars_col)) in rows.iter().enumerate() {
            assert_eq!(
                chars_col,
                source_chars_col,
                "第 {} 行: chars={} != source_chars={}, 新行两列应相等",
                i + 1,
                chars_col,
                source_chars_col
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
            conn.execute(
                "ALTER TABLE translation_logs ADD COLUMN target_chars INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .unwrap();

            // 验证旧行确实有文本值
            let val: String = conn
                .prepare("SELECT typeof(source_chars) FROM translation_logs LIMIT 1")
                .unwrap()
                .query_row([], |row| row.get(0))
                .unwrap();
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
            assert_eq!(
                type_name, "integer",
                "迁移后 source_chars 应为 integer 类型"
            );
            assert_eq!(chars, source_chars, "迁移后 source_chars 应等于 chars");
        }

        // 验证 SUM(source_chars) 现在返回正确结果
        let sum: i64 = conn
            .prepare("SELECT COALESCE(SUM(source_chars), 0) FROM translation_logs")
            .unwrap()
            .query_row([], |row| row.get(0))
            .unwrap();
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
                db.log_translation("EN", "ZH", i + 1, 0, "success", None, "", None)
                    .unwrap();
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
        let (period_req, period_chars) = db.get_period_stats_filtered(365, None).unwrap();
        assert!(period_req > 0, "365天内应有请求");
        assert!(period_chars > 0, "365天内应有字符");

        // 验证分页查询
        let (items, total) = db.get_requests_filtered(1, 10, None, None).unwrap();
        assert!(total > 0);
        assert!(!items.is_empty());
        // 验证 source_chars 字段是正确的整数
        for item in &items {
            assert!(
                item.source_chars > 0,
                "source_chars 应为正整数, got {}",
                item.source_chars
            );
        }

        // 验证图表查询
        let daily = db.get_daily_stats_filtered(365, None).unwrap();
        assert!(!daily.is_empty(), "应有每日统计");
        for d in &daily {
            assert!(d.chars > 0, "每日字符数应 > 0");
        }

        // 验证热力图
        let heatmap = db.get_heatmap_by_weekday_filtered(365, None).unwrap();
        assert!(!heatmap.is_empty());

        std::mem::forget(dir);
    }

    #[test]
    fn test_log_translation_updates_hourly_stats() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "ep1", Some(200))
            .unwrap();
        db.log_translation("EN", "ZH", 50, 40, "error", Some("timeout"), "ep1", None)
            .unwrap();

        let conn = db.conn.lock().unwrap();
        // 全局行应有 2 条请求
        let (req, succ, err, src_chars): (i64, i64, i64, i64) = conn
            .prepare("SELECT request_count, success_count, error_count, source_chars_sum FROM hourly_stats WHERE endpoint_name = ''")
            .unwrap()
            .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .unwrap();
        assert_eq!(req, 2);
        assert_eq!(succ, 1);
        assert_eq!(err, 1);
        assert_eq!(src_chars, 150);

        // 端点行也应有 2 条
        let ep_req: i64 = conn
            .prepare("SELECT request_count FROM hourly_stats WHERE endpoint_name = 'ep1'")
            .unwrap()
            .query_row([], |row| row.get(0))
            .unwrap();
        assert_eq!(ep_req, 2);
    }

    #[test]
    fn test_log_translation_updates_lang_stats() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "ep1", Some(100))
            .unwrap();

        let conn = db.conn.lock().unwrap();
        // source 全局行
        let src_count: i64 = conn
            .prepare("SELECT request_count FROM hourly_lang_stats WHERE lang = 'EN' AND role = 'source' AND endpoint_name = ''")
            .unwrap()
            .query_row([], |row| row.get(0))
            .unwrap();
        assert_eq!(src_count, 1);

        // target 全局行
        let tgt_count: i64 = conn
            .prepare("SELECT request_count FROM hourly_lang_stats WHERE lang = 'ZH' AND role = 'target' AND endpoint_name = ''")
            .unwrap()
            .query_row([], |row| row.get(0))
            .unwrap();
        assert_eq!(tgt_count, 1);
    }

    #[test]
    fn test_cleanup_does_not_affect_hourly_stats() {
        let db = temp_db();
        db.log_translation("EN", "ZH", 100, 80, "success", None, "", Some(150))
            .unwrap();

        // 手动把日志改为 40 天前
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE translation_logs SET created_at = datetime('now', '-40 days', 'localtime')",
                [],
            )
            .unwrap();
        }

        // 清理日志（保留 30 天）
        let deleted = db.cleanup_old_logs(30).unwrap();
        assert_eq!(deleted, 1);

        // hourly_stats 不受影响
        let conn = db.conn.lock().unwrap();
        let req: i64 = conn
            .prepare("SELECT SUM(request_count) FROM hourly_stats WHERE endpoint_name = ''")
            .unwrap()
            .query_row([], |row| row.get(0))
            .unwrap();
        assert_eq!(req, 1, "hourly_stats 不应因日志清理而减少");
    }

    #[test]
    fn test_get_daily_stats_after_log_cleanup() {
        let db = temp_db();
        // 插入 3 条记录
        for _ in 0..3 {
            db.log_translation("EN", "ZH", 50, 40, "success", None, "", Some(100))
                .unwrap();
        }

        // 把日志推到 40 天前，再清理
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE translation_logs SET created_at = datetime('now', '-40 days', 'localtime')",
                [],
            )
            .unwrap();
        }
        let deleted = db.cleanup_old_logs(30).unwrap();
        assert_eq!(deleted, 3, "3 条 40 天前的日志应被清理");

        // 日志列表应为空
        let (items, _) = db.get_requests_filtered(1, 50, None, None).unwrap();
        assert!(items.is_empty(), "日志应已被清空");

        // 但图表数据仍完整（从 hourly_stats 查询，不受日志清理影响）
        // 注意：hourly_stats 中的 bucket_hour 仍是日志写入时的"今天"，所以查询"最近 1 天"应能命中
        let daily = db.get_daily_stats_filtered(1, None).unwrap();
        assert!(!daily.is_empty(), "清理日志后图表数据应仍存在");
        let total_req: i64 = daily.iter().map(|d| d.count).sum();
        assert_eq!(total_req, 3, "图表应显示 3 条请求");
    }
}
