use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub upstream: UpstreamConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub monitor: MonitorConfig,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub demo: DemoConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    #[serde(default)]
    pub endpoints: Vec<EndpointConfig>,
    #[serde(default = "default_max_failures")]
    pub max_failures: u32,
    #[serde(default = "default_probe_interval_secs")]
    pub probe_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EndpointConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub api_key: String,
}

fn default_max_failures() -> u32 {
    3
}

fn default_probe_interval_secs() -> u64 {
    60
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    #[serde(default = "default_proxy_host")]
    pub host: String,
    #[serde(default = "default_proxy_port")]
    pub port: u16,
}

fn default_proxy_host() -> String {
    "127.0.0.1".to_string()
}

fn default_proxy_port() -> u16 {
    5555
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitorConfig {
    #[serde(default)]
    pub auto_refresh_seconds: u64,
    #[serde(default = "default_max_log_entries")]
    pub max_log_entries: u32,
}

fn default_max_log_entries() -> u32 {
    10000
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DemoConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_demo_seed")]
    pub seed: u64,
}

fn default_demo_seed() -> u64 {
    20260511
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_source_lang")]
    pub source_lang: String,
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
}

fn default_source_lang() -> String {
    "EN".to_string()
}

fn default_target_lang() -> String {
    "ZH".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,
    #[serde(default = "default_cache_max_entries")]
    pub max_entries: u64,
    /// 最大内存限制（MB），0 表示不限制内存，使用条目数限制
    #[serde(default)]
    pub max_memory_mb: u64,
}

fn default_cache_ttl_secs() -> u64 {
    3600
}

fn default_cache_max_entries() -> u64 {
    10000
}

impl Default for Config {
    fn default() -> Self {
        Self {
            upstream: UpstreamConfig::default(),
            proxy: ProxyConfig::default(),
            monitor: MonitorConfig::default(),
            health_check: HealthCheckConfig::default(),
            demo: DemoConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            endpoints: vec![],
            max_failures: default_max_failures(),
            probe_interval_secs: default_probe_interval_secs(),
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            host: default_proxy_host(),
            port: default_proxy_port(),
        }
    }
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            auto_refresh_seconds: 0,
            max_log_entries: default_max_log_entries(),
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            source_lang: default_source_lang(),
            target_lang: default_target_lang(),
        }
    }
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            seed: default_demo_seed(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_secs: default_cache_ttl_secs(),
            max_entries: default_cache_max_entries(),
            max_memory_mb: 0,
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(ConfigError::IoError)?;
        toml::from_str(&content).map_err(ConfigError::ParseError)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self).map_err(ConfigError::SerializeError)?;
        fs::write(path, content).map_err(ConfigError::IoError)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(toml::de::Error),
    SerializeError(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
            ConfigError::SerializeError(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

/// 监听 config.toml 文件变更，自动热加载配置。
/// 使用 1 秒去抖动避免编辑器多次写入触发重复加载。
/// proxy.host / proxy.port 变更仅记录警告（需重启生效）。
/// 监听父目录而非文件本身，以支持文件不存在时后续创建的场景。
pub async fn watch_config_file(
    config_path: String,
    config: Arc<RwLock<Config>>,
    load_balancer: Arc<crate::upstream::LoadBalancer>,
    cache: Arc<crate::cache::TranslationCache>,
) {
    use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::time::Duration;
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel::<()>(1);

    // 使用标准 notify watcher，通过 channel 桥接到 tokio
    let path_for_watcher = std::path::PathBuf::from(&config_path);
    let canonical_path = path_for_watcher.canonicalize().unwrap_or_else(|_| path_for_watcher.clone());

    let canonical_path_for_handler = canonical_path.clone();
    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // 只关注写入/修改/创建事件
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_)
                ) {
                    // 检查事件是否与我们的配置文件相关
                    let is_our_file = event.paths.iter().any(|p| {
                        p == &canonical_path_for_handler
                            || p.file_name() == canonical_path_for_handler.file_name()
                    });
                    if is_our_file {
                        // 非阻塞发送，如果 channel 满了就丢弃（去抖动会处理）
                        let _ = tx.try_send(());
                    }
                }
            }
        },
        NotifyConfig::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("无法创建文件监听器: {}", e);
            return;
        }
    };

    // 监听父目录，这样即使文件不存在也能检测到后续创建
    let watch_path = canonical_path.parent().unwrap_or(std::path::Path::new("."));
    if let Err(e) = watcher.watch(watch_path, RecursiveMode::NonRecursive) {
        tracing::error!("无法监听配置目录 {:?}: {}", watch_path, e);
        return;
    }

    tracing::info!("已启动配置文件热加载监听: {}", config_path);

    // 保持 watcher 存活
    let _watcher = watcher;

    loop {
        // 等待文件变更通知
        if rx.recv().await.is_none() {
            tracing::warn!("配置文件监听 channel 已关闭");
            break;
        }

        // 去抖动：等待 1 秒，期间消耗所有额外通知
        tokio::time::sleep(Duration::from_secs(1)).await;
        while rx.try_recv().is_ok() {}

        // 读取并解析新配置
        match Config::load(&config_path) {
            Ok(new_config) => {
                let mut current = config.write().await;

                // 检查 proxy 地址是否变更（需重启生效）
                if new_config.proxy.host != current.proxy.host
                    || new_config.proxy.port != current.proxy.port
                {
                    tracing::warn!(
                        "检测到 proxy.host/port 变更 ({}:{} -> {}:{})，需重启服务才能生效",
                        current.proxy.host,
                        current.proxy.port,
                        new_config.proxy.host,
                        new_config.proxy.port
                    );
                }

                // 热加载 LoadBalancer 端点配置
                let endpoints = new_config.upstream.endpoints.clone();
                let max_failures = new_config.upstream.max_failures;

                // 热加载缓存配置
                let cache_enabled = new_config.cache.enabled;
                let cache_ttl = new_config.cache.ttl_secs;
                let cache_max = new_config.cache.max_entries;
                let cache_max_memory_mb = new_config.cache.max_memory_mb;

                *current = new_config;
                drop(current); // 释放 config 写锁后再 await LB reload

                load_balancer.reload(endpoints, max_failures).await;
                cache.reload(cache_enabled, cache_ttl, cache_max, cache_max_memory_mb);
                tracing::info!("配置文件已热加载: {}", config_path);
            }
            Err(e) => {
                tracing::warn!("配置文件解析失败，保留当前配置: {}", e);
            }
        }
    }
}
