use arc_swap::ArcSwap;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 缓存统计信息快照
///
/// 由 `TranslationCache::stats()` 生成，包含累计和当日的命中/未命中计数、
/// 命中率、当前缓存大小及配置参数。用于 `/api/cache/stats` 接口返回给前端。
#[derive(Clone, Debug, serde::Serialize)]
pub struct CacheStats {
    /// 缓存是否启用
    pub enabled: bool,
    /// 累计命中次数（自启动或上次清除以来）
    pub hits: u64,
    /// 累计未命中次数
    pub misses: u64,
    /// 今日命中次数（每日零点自动重置）
    pub today_hits: u64,
    /// 今日未命中次数
    pub today_misses: u64,
    /// 当前缓存中的条目数量
    pub size: u64,
    /// 配置的最大条目数（当 max_memory_mb == 0 时生效）
    pub max_entries: u64,
    /// 缓存条目的存活时间（秒）
    pub ttl_secs: u64,
    /// 累计命中率 = hits / (hits + misses)，范围 [0.0, 1.0]
    pub hit_rate: f64,
    /// 今日命中率
    pub today_hit_rate: f64,
    /// 配置的最大内存限制（MB），为 0 表示使用条目数限制
    pub max_memory_mb: u64,
    /// moka 估算的当前缓存占用字节数（基于 weigher 计算）
    pub estimated_memory_bytes: u64,
}

/// 缓存中存储的翻译结果
///
/// 包含完整的 JSON 响应体，命中时直接返回给客户端，无需再次请求上游。
#[derive(Clone)]
pub struct CachedTranslation {
    /// 上游返回的完整 JSON 响应（包含 code、data、alternatives 等字段）
    pub response_json: serde_json::Value,
}

/// 缓存命中日志条目
///
/// 每次缓存命中时记录一条，用于前端展示最近的命中记录。
/// 通过环形缓冲区（VecDeque）维护，最多保留 `MAX_HIT_LOG` 条。
#[derive(Clone, Debug, serde::Serialize)]
pub struct CacheHitEntry {
    /// 源语言代码（如 "EN"、"ZH"）
    pub source_lang: String,
    /// 目标语言代码
    pub target_lang: String,
    /// 原文预览（截取前 50 个字符）
    pub text_preview: String,
    /// 命中时间戳（ISO 8601 格式）
    pub timestamp: String,
}

/// 命中日志的最大保留条数（环形缓冲区容量）
const MAX_HIT_LOG: usize = 100;

/// 获取当前日期字符串（格式 "YYYY-MM-DD"）
///
/// 从 `chrono_now()` 返回的 ISO 8601 时间戳中提取日期部分，
/// 用于判断是否需要重置当日计数器。
fn today_str() -> String {
    crate::utils::chrono_now()
        .split('T')
        .next()
        .unwrap_or("")
        .to_string()
}

/// 翻译缓存核心结构
///
/// 设计要点：
/// - 使用 `ArcSwap` 包装 moka Cache，支持热加载时原子替换整个缓存实例，
///   无需持有写锁即可切换到新配置的缓存。
/// - 原子计数器（AtomicU64/AtomicBool）用于无锁的命中/未命中统计，
///   避免在高并发翻译请求路径上产生锁竞争。
/// - 命中日志（hit_log）使用 Mutex<VecDeque> 作为环形缓冲区，
///   仅在命中时写入，频率相对较低，锁竞争可接受。
/// - 支持两种容量限制模式：基于内存大小（max_memory_mb > 0）或基于条目数。
pub struct TranslationCache {
    /// moka 并发缓存实例，通过 ArcSwap 实现热加载时的无锁替换
    cache: ArcSwap<moka::sync::Cache<String, CachedTranslation>>,
    /// 累计命中次数（跨日累计，重启后从数据库恢复）
    hits: AtomicU64,
    /// 累计未命中次数
    misses: AtomicU64,
    /// 今日命中次数（每日零点自动重置）
    today_hits: AtomicU64,
    /// 今日未命中次数
    today_misses: AtomicU64,
    /// 当前日期字符串，用于检测日期切换并重置当日计数器
    today_date: Mutex<String>,
    /// 缓存是否启用（禁用时 get/insert 直接跳过）
    enabled: AtomicBool,
    /// 最大条目数配置（当 max_memory_mb == 0 时作为 moka 的 max_capacity）
    max_entries: AtomicU64,
    /// 缓存条目存活时间（秒），超时后自动淘汰
    ttl_secs: AtomicU64,
    /// 最大内存限制（MB），大于 0 时启用基于权重的内存淘汰策略
    max_memory_mb: AtomicU64,
    /// 最近命中日志的环形缓冲区，容量为 MAX_HIT_LOG
    hit_log: Mutex<VecDeque<CacheHitEntry>>,
}

impl TranslationCache {
    /// 创建新的翻译缓存实例
    ///
    /// # 参数
    /// - `enabled`: 是否启用缓存
    /// - `ttl_secs`: 缓存条目存活时间（秒）
    /// - `max_entries`: 最大条目数（当 max_memory_mb == 0 时生效）
    /// - `max_memory_mb`: 最大内存限制（MB），为 0 表示使用条目数限制
    ///
    /// # 返回
    /// 返回 `Arc<Self>`，便于在多个异步任务间共享
    pub fn new(enabled: bool, ttl_secs: u64, max_entries: u64, max_memory_mb: u64) -> Arc<Self> {
        let cache = Self::build_cache(ttl_secs, max_entries, max_memory_mb);

        Arc::new(Self {
            cache: ArcSwap::from_pointee(cache),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            today_hits: AtomicU64::new(0),
            today_misses: AtomicU64::new(0),
            today_date: Mutex::new(today_str()),
            enabled: AtomicBool::new(enabled),
            max_entries: AtomicU64::new(max_entries),
            ttl_secs: AtomicU64::new(ttl_secs),
            max_memory_mb: AtomicU64::new(max_memory_mb),
            hit_log: Mutex::new(VecDeque::with_capacity(MAX_HIT_LOG)),
        })
    }

    /// 构建 moka cache 实例，根据 max_memory_mb 决定使用内存限制还是条目数限制
    ///
    /// 两种模式：
    /// 1. **内存限制模式**（max_memory_mb > 0）：使用 weigher 函数估算每个条目的
    ///    字节大小（将 response_json 序列化为字符串后取长度），moka 根据总权重
    ///    自动淘汰条目，使总内存不超过 max_memory_mb * 1024 * 1024 字节。
    /// 2. **条目数限制模式**（max_memory_mb == 0）：直接以 max_entries 作为
    ///    max_capacity，moka 按 LRU 策略淘汰超出的条目。
    ///
    /// # 参数
    /// - `ttl_secs`: 条目存活时间（秒），超时后自动过期
    /// - `max_entries`: 最大条目数（仅在条目数限制模式下使用）
    /// - `max_memory_mb`: 最大内存（MB），大于 0 时启用内存限制模式
    fn build_cache(
        ttl_secs: u64,
        max_entries: u64,
        max_memory_mb: u64,
    ) -> moka::sync::Cache<String, CachedTranslation> {
        let builder = moka::sync::Cache::builder().time_to_live(Duration::from_secs(ttl_secs));

        if max_memory_mb > 0 {
            // 使用内存限制：weigher 估算每个条目的字节大小
            builder
                .weigher(|_key: &String, value: &CachedTranslation| -> u32 {
                    serde_json::to_string(&value.response_json)
                        .map(|s| s.len() as u32)
                        .unwrap_or(256)
                })
                .max_capacity(max_memory_mb * 1024 * 1024)
                .build()
        } else {
            // 使用条目数限制
            builder.max_capacity(max_entries).build()
        }
    }

    /// 热加载缓存配置
    ///
    /// 当配置文件变更或通过 Web UI 修改缓存配置时调用。
    /// 会重建整个 moka cache 实例（已有缓存条目会丢失，这是可接受的行为，
    /// 因为配置变更可能改变了 TTL 或容量，旧条目的语义可能不再正确）。
    ///
    /// # 参数
    /// - `enabled`: 是否启用缓存
    /// - `ttl_secs`: 新的 TTL（秒）
    /// - `max_entries`: 新的最大条目数
    /// - `max_memory_mb`: 新的内存限制（MB）
    pub fn reload(&self, enabled: bool, ttl_secs: u64, max_entries: u64, max_memory_mb: u64) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.ttl_secs.store(ttl_secs, Ordering::Relaxed);
        self.max_entries.store(max_entries, Ordering::Relaxed);
        self.max_memory_mb.store(max_memory_mb, Ordering::Relaxed);

        let new_cache = Self::build_cache(ttl_secs, max_entries, max_memory_mb);
        self.cache.store(Arc::new(new_cache));

        tracing::info!(
            "缓存已热加载: enabled={}, ttl={}s, max_entries={}, max_memory_mb={}",
            enabled,
            ttl_secs,
            max_entries,
            max_memory_mb
        );
    }

    /// 清除缓存：重建 moka cache 并重置所有命中/未命中计数器
    ///
    /// 通过重建而非调用 moka 的 invalidate_all() 来确保计数器与缓存状态一致。
    /// 清除后 size、hits、misses、today_hits、today_misses 全部归零。
    pub fn clear(&self) {
        let max_entries = self.max_entries.load(Ordering::Relaxed);
        let ttl_secs = self.ttl_secs.load(Ordering::Relaxed);
        let max_memory_mb = self.max_memory_mb.load(Ordering::Relaxed);

        let new_cache = Self::build_cache(ttl_secs, max_entries, max_memory_mb);

        self.cache.store(Arc::new(new_cache));
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.today_hits.store(0, Ordering::Relaxed);
        self.today_misses.store(0, Ordering::Relaxed);

        tracing::info!("缓存已清除，计数器已重置");
    }

    /// 检查是否跨日，若日期已切换则重置当日计数器
    ///
    /// 逻辑：获取当前日期字符串，与 `today_date` 对比。若不同，说明已过零点，
    /// 将 `today_date` 更新为新日期，并将 `today_hits` 和 `today_misses` 归零。
    /// 在 `get()` 和 `stats()` 中调用，确保当日统计数据的准确性。
    fn maybe_reset_today(&self) {
        let today = today_str();
        if let Ok(mut date) = self.today_date.lock() {
            if *date != today {
                *date = today;
                self.today_hits.store(0, Ordering::Relaxed);
                self.today_misses.store(0, Ordering::Relaxed);
            }
        }
    }

    /// 查询缓存，返回命中的翻译结果
    ///
    /// 流程：
    /// 1. 检查缓存是否启用，未启用直接返回 None
    /// 2. 调用 `maybe_reset_today()` 检查日期切换
    /// 3. 根据 text + source_lang + target_lang 生成 SHA256 缓存键
    /// 4. 查询 moka cache：
    ///    - 命中：累加 hits/today_hits 计数器，记录命中日志，返回 Some
    ///    - 未命中：累加 misses/today_misses 计数器，返回 None
    ///
    /// # 参数
    /// - `text`: 待翻译的原文
    /// - `source_lang`: 源语言代码（如 "EN"）
    /// - `target_lang`: 目标语言代码（如 "ZH"）
    ///
    /// # 返回
    /// 命中时返回 `Some(CachedTranslation)`，未命中或缓存禁用时返回 `None`
    pub fn get(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Option<CachedTranslation> {
        if !self.enabled.load(Ordering::Relaxed) {
            return None;
        }
        self.maybe_reset_today();
        let key = Self::make_key(text, source_lang, target_lang);
        let cache_guard = self.cache.load();
        match cache_guard.get(&key) {
            Some(entry) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                self.today_hits.fetch_add(1, Ordering::Relaxed);
                // 记录命中日志
                let preview: String = text.chars().take(50).collect();
                let log_entry = CacheHitEntry {
                    source_lang: source_lang.to_string(),
                    target_lang: target_lang.to_string(),
                    text_preview: preview,
                    timestamp: crate::utils::chrono_now(),
                };
                if let Ok(mut log) = self.hit_log.lock() {
                    if log.len() >= MAX_HIT_LOG {
                        log.pop_front();
                    }
                    log.push_back(log_entry);
                }
                Some(entry)
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                self.today_misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// 获取缓存命中日志（最近 100 条，按时间倒序）
    pub fn hit_log(&self) -> Vec<CacheHitEntry> {
        if let Ok(log) = self.hit_log.lock() {
            log.iter().rev().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 将翻译结果写入缓存
    ///
    /// 缓存禁用时直接跳过，不执行任何操作。
    ///
    /// # 参数
    /// - `text`: 原文（用于生成缓存键）
    /// - `source_lang`: 源语言代码
    /// - `target_lang`: 目标语言代码
    /// - `response`: 上游返回的完整 JSON 响应体
    pub fn insert(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        response: serde_json::Value,
    ) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let key = Self::make_key(text, source_lang, target_lang);
        let cache_guard = self.cache.load();
        cache_guard.insert(
            key,
            CachedTranslation {
                response_json: response,
            },
        );
    }

    /// 生成缓存统计信息快照
    ///
    /// 会先调用 `maybe_reset_today()` 确保当日数据准确，然后读取所有原子计数器
    /// 和 moka cache 的实时状态，组装为 `CacheStats` 返回。
    pub fn stats(&self) -> CacheStats {
        self.maybe_reset_today();
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        let today_hits = self.today_hits.load(Ordering::Relaxed);
        let today_misses = self.today_misses.load(Ordering::Relaxed);
        let today_total = today_hits + today_misses;
        let today_hit_rate = if today_total > 0 {
            today_hits as f64 / today_total as f64
        } else {
            0.0
        };

        let cache_guard = self.cache.load();
        let size = cache_guard.entry_count();
        let estimated_memory_bytes = cache_guard.weighted_size();

        CacheStats {
            enabled: self.enabled.load(Ordering::Relaxed),
            hits,
            misses,
            today_hits,
            today_misses,
            size,
            max_entries: self.max_entries.load(Ordering::Relaxed),
            ttl_secs: self.ttl_secs.load(Ordering::Relaxed),
            hit_rate,
            today_hit_rate,
            max_memory_mb: self.max_memory_mb.load(Ordering::Relaxed),
            estimated_memory_bytes,
        }
    }

    /// 从数据库恢复缓存统计计数器（启动时调用）
    pub fn restore_stats(&self, hits: u64, misses: u64) {
        self.hits.store(hits, Ordering::Relaxed);
        self.misses.store(misses, Ordering::Relaxed);
    }

    /// 获取当前命中/未命中原始值（用于持久化到数据库）
    pub fn get_raw_stats(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }

    /// 生成缓存键：使用 SHA256 哈希 text + source_lang + target_lang
    ///
    /// 哈希策略说明：
    /// - 将 source_lang 纳入哈希，防止同一文本在不同源语言下的缓存污染
    ///   （例如 "gift" 在 EN→ZH 和 DE→ZH 中含义不同）
    /// - 使用 "|" 作为分隔符，避免字段拼接时产生歧义
    /// - SHA256 输出为 64 字符十六进制字符串，作为 moka cache 的 key
    ///
    /// # 参数
    /// - `text`: 原文
    /// - `source_lang`: 源语言代码
    /// - `target_lang`: 目标语言代码
    ///
    /// # 返回
    /// 64 字符的十六进制 SHA256 哈希字符串
    fn make_key(text: &str, source_lang: &str, target_lang: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hasher.update(b"|");
        hasher.update(source_lang.as_bytes());
        hasher.update(b"|");
        hasher.update(target_lang.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache(enabled: bool) -> Arc<TranslationCache> {
        TranslationCache::new(enabled, 3600, 1000, 0)
    }

    #[test]
    fn test_insert_and_get() {
        let cache = make_cache(true);
        let json = serde_json::json!({"code": 200, "data": "你好"});
        cache.insert("hello", "EN", "ZH", json.clone());

        let result = cache.get("hello", "EN", "ZH");
        assert!(result.is_some());
        assert_eq!(result.unwrap().response_json, json);
    }

    #[test]
    fn test_disabled_cache_returns_none() {
        let cache = make_cache(false);
        let json = serde_json::json!({"code": 200, "data": "你好"});
        cache.insert("hello", "EN", "ZH", json);

        let result = cache.get("hello", "EN", "ZH");
        assert!(result.is_none(), "禁用缓存时应返回 None");
    }

    #[test]
    fn test_different_lang_pair_is_different_key() {
        let cache = make_cache(true);
        let json_en_zh = serde_json::json!({"data": "你好"});
        let json_en_ja = serde_json::json!({"data": "こんにちは"});
        cache.insert("hello", "EN", "ZH", json_en_zh.clone());
        cache.insert("hello", "EN", "JA", json_en_ja.clone());

        assert_eq!(
            cache.get("hello", "EN", "ZH").unwrap().response_json,
            json_en_zh
        );
        assert_eq!(
            cache.get("hello", "EN", "JA").unwrap().response_json,
            json_en_ja
        );
    }

    #[test]
    fn test_hit_miss_counting() {
        let cache = make_cache(true);
        let json = serde_json::json!({"data": "test"});
        cache.insert("hello", "EN", "ZH", json);

        cache.get("hello", "EN", "ZH"); // hit
        cache.get("hello", "EN", "ZH"); // hit
        cache.get("world", "EN", "ZH"); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_clear_resets_everything() {
        let cache = make_cache(true);
        let json = serde_json::json!({"data": "test"});
        cache.insert("hello", "EN", "ZH", json);
        cache.get("hello", "EN", "ZH");

        cache.clear();

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.size, 0);
        assert!(cache.get("hello", "EN", "ZH").is_none());
    }

    #[test]
    fn test_reload_rebuilds_cache() {
        let cache = make_cache(true);
        let json = serde_json::json!({"data": "test"});
        cache.insert("hello", "EN", "ZH", json);

        cache.reload(true, 7200, 2000, 0);

        // 旧条目应该丢失
        assert!(cache.get("hello", "EN", "ZH").is_none());
        assert_eq!(cache.stats().ttl_secs, 7200);
        assert_eq!(cache.stats().max_entries, 2000);
    }

    #[test]
    fn test_restore_stats() {
        let cache = make_cache(true);
        cache.restore_stats(100, 50);
        let stats = cache.stats();
        assert_eq!(stats.hits, 100);
        assert_eq!(stats.misses, 50);
    }

    #[test]
    fn test_get_raw_stats() {
        let cache = make_cache(true);
        let json = serde_json::json!({"data": "test"});
        cache.insert("hello", "EN", "ZH", json);
        cache.get("hello", "EN", "ZH");
        cache.get("world", "EN", "ZH");

        let (hits, misses) = cache.get_raw_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[test]
    fn test_hit_log_records_entries() {
        let cache = make_cache(true);
        let json = serde_json::json!({"data": "test"});
        cache.insert("hello world", "EN", "ZH", json);
        cache.get("hello world", "EN", "ZH");

        let log = cache.hit_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].source_lang, "EN");
        assert_eq!(log[0].target_lang, "ZH");
        assert_eq!(log[0].text_preview, "hello world");
    }

    #[test]
    fn test_hit_log_max_capacity() {
        let cache = make_cache(true);
        for i in 0..150 {
            let text = format!("text_{}", i);
            let json = serde_json::json!({"data": text});
            cache.insert(&text, "EN", "ZH", json);
            cache.get(&text, "EN", "ZH");
        }

        let log = cache.hit_log();
        assert_eq!(
            log.len(),
            MAX_HIT_LOG,
            "命中日志应限制在 {} 条",
            MAX_HIT_LOG
        );
    }

    #[test]
    fn test_memory_based_cache() {
        let cache = TranslationCache::new(true, 3600, 1000, 1); // 1MB limit
        let stats = cache.stats();
        assert_eq!(stats.max_memory_mb, 1);
    }
}
