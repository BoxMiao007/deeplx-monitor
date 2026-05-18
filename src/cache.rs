use arc_swap::ArcSwap;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, serde::Serialize)]
pub struct CacheStats {
    pub enabled: bool,
    pub hits: u64,
    pub misses: u64,
    pub size: u64,
    pub max_entries: u64,
    pub ttl_secs: u64,
    pub hit_rate: f64,
    pub max_memory_mb: u64,
    pub estimated_memory_bytes: u64,
}

#[derive(Clone)]
pub struct CachedTranslation {
    pub response_json: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CacheHitEntry {
    pub source_lang: String,
    pub target_lang: String,
    pub text_preview: String,
    pub timestamp: String,
}

const MAX_HIT_LOG: usize = 100;

pub struct TranslationCache {
    cache: ArcSwap<moka::sync::Cache<String, CachedTranslation>>,
    hits: AtomicU64,
    misses: AtomicU64,
    enabled: AtomicBool,
    max_entries: AtomicU64,
    ttl_secs: AtomicU64,
    max_memory_mb: AtomicU64,
    hit_log: Mutex<VecDeque<CacheHitEntry>>,
}

impl TranslationCache {
    pub fn new(enabled: bool, ttl_secs: u64, max_entries: u64, max_memory_mb: u64) -> Arc<Self> {
        let cache = Self::build_cache(ttl_secs, max_entries, max_memory_mb);

        Arc::new(Self {
            cache: ArcSwap::from_pointee(cache),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            enabled: AtomicBool::new(enabled),
            max_entries: AtomicU64::new(max_entries),
            ttl_secs: AtomicU64::new(ttl_secs),
            max_memory_mb: AtomicU64::new(max_memory_mb),
            hit_log: Mutex::new(VecDeque::with_capacity(MAX_HIT_LOG)),
        })
    }

    /// 构建 moka cache，根据 max_memory_mb 决定使用内存限制还是条目数限制
    fn build_cache(ttl_secs: u64, max_entries: u64, max_memory_mb: u64) -> moka::sync::Cache<String, CachedTranslation> {
        let builder = moka::sync::Cache::builder()
            .time_to_live(Duration::from_secs(ttl_secs));

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
            builder
                .max_capacity(max_entries)
                .build()
        }
    }

    /// 热加载缓存配置。重建 moka cache（已有缓存条目会丢失，可接受）。
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

    /// 清除缓存：重建 moka cache 并重置命中/未命中计数器。
    pub fn clear(&self) {
        let max_entries = self.max_entries.load(Ordering::Relaxed);
        let ttl_secs = self.ttl_secs.load(Ordering::Relaxed);
        let max_memory_mb = self.max_memory_mb.load(Ordering::Relaxed);

        let new_cache = Self::build_cache(ttl_secs, max_entries, max_memory_mb);

        self.cache.store(Arc::new(new_cache));
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);

        tracing::info!("缓存已清除，计数器已重置");
    }

    pub fn get(&self, text: &str, source_lang: &str, target_lang: &str) -> Option<CachedTranslation> {
        if !self.enabled.load(Ordering::Relaxed) {
            return None;
        }
        let key = Self::make_key(text, source_lang, target_lang);
        let cache_guard = self.cache.load();
        match cache_guard.get(&key) {
            Some(entry) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
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

    pub fn insert(&self, text: &str, source_lang: &str, target_lang: &str, response: serde_json::Value) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let key = Self::make_key(text, source_lang, target_lang);
        let cache_guard = self.cache.load();
        cache_guard.insert(key, CachedTranslation { response_json: response });
    }

    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 { hits as f64 / total as f64 } else { 0.0 };

        let cache_guard = self.cache.load();
        let size = cache_guard.entry_count();
        let estimated_memory_bytes = cache_guard.weighted_size();

        CacheStats {
            enabled: self.enabled.load(Ordering::Relaxed),
            hits,
            misses,
            size,
            max_entries: self.max_entries.load(Ordering::Relaxed),
            ttl_secs: self.ttl_secs.load(Ordering::Relaxed),
            hit_rate,
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
