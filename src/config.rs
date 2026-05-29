use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 应用程序顶层配置结构体，对应 `config.toml` 文件。
/// 包含上游端点、代理服务器、监控面板、健康检查、演示模式和缓存六大配置模块。
/// 所有字段均有默认值，支持部分配置文件加载（缺失字段自动填充默认值）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// 上游 DeepLX 端点配置（负载均衡、故障转移相关）
    #[serde(default)]
    pub upstream: UpstreamConfig,
    /// 代理服务器监听地址配置（host + port，变更需重启）
    #[serde(default)]
    pub proxy: ProxyConfig,
    /// 监控面板配置（自动刷新间隔、日志保留策略）
    #[serde(default)]
    pub monitor: MonitorConfig,
    /// 健康检查配置（测试翻译使用的源语言和目标语言）
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    /// 演示模式配置（启用后生成模拟数据用于展示）
    #[serde(default)]
    pub demo: DemoConfig,
    /// 翻译缓存配置（LRU 缓存策略，支持内存或条目数限制）
    #[serde(default)]
    pub cache: CacheConfig,
}

/// 上游端点配置，管理所有 DeepLX 翻译服务端点。
/// 支持多端点轮询（round-robin）负载均衡和自动故障转移。
/// 当某端点连续失败次数达到 `max_failures` 时，将被标记为不可用，
/// 后台探测任务每隔 `probe_interval_secs` 秒检测并恢复。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    /// 上游端点列表，LoadBalancer 按顺序轮询分发请求
    #[serde(default)]
    pub endpoints: Vec<EndpointConfig>,
    /// 端点连续失败次数阈值，超过后标记为不可用，默认 3 次
    #[serde(default = "default_max_failures")]
    pub max_failures: u32,
    /// 后台健康探测间隔（秒），对不可用端点定期尝试恢复，默认 60 秒
    #[serde(default = "default_probe_interval_secs")]
    pub probe_interval_secs: u64,
}

/// 单个上游端点的配置信息。
/// 每个端点对应一个 DeepLX 翻译服务实例。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EndpointConfig {
    /// 端点名称，用于日志和 UI 展示的可读标识
    #[serde(default)]
    pub name: String,
    /// 端点 URL，完整的翻译 API 地址（如 `http://host:port/translate`）
    #[serde(default)]
    pub url: String,
    /// API 密钥，若端点需要认证则填写，空字符串表示无需认证
    #[serde(default)]
    pub api_key: String,
}

/// 端点最大连续失败次数默认值：3 次
fn default_max_failures() -> u32 {
    3
}

/// 后台探测间隔默认值：60 秒
fn default_probe_interval_secs() -> u64 {
    60
}

/// 代理服务器监听配置。
/// 注意：`host` 和 `port` 的变更需要重启服务才能生效，热加载时仅记录警告。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// 监听地址，默认 "127.0.0.1"（仅本地访问），设为 "0.0.0.0" 可对外暴露
    #[serde(default = "default_proxy_host")]
    pub host: String,
    /// 监听端口，默认 5555
    #[serde(default = "default_proxy_port")]
    pub port: u16,
}

/// 代理监听地址默认值："127.0.0.1"
fn default_proxy_host() -> String {
    "127.0.0.1".to_string()
}

/// 代理监听端口默认值：5555
fn default_proxy_port() -> u16 {
    5555
}

/// 监控面板配置，控制前端自动刷新行为和日志清理策略。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitorConfig {
    /// 前端自动刷新间隔（秒），0 表示禁用自动刷新
    #[serde(default)]
    pub auto_refresh_seconds: u64,
    /// 日志保留天数，超过此天数的日志明细会被滑动窗口式清理。
    /// 注意：清理仅影响"日志"标签页的明细列表，所有图表（趋势/热力图/错误趋势/语言统计）
    /// 数据源自永久保留的 `hourly_stats` / `hourly_lang_stats` 汇总表，不受日志清理影响。
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u32,
}

/// 日志保留天数默认值：30 天。图表数据由汇总表永久保留，不依赖此值。
fn default_log_retention_days() -> u32 {
    30
}

/// 演示模式配置。
/// 启用后系统会基于种子值生成模拟翻译数据，用于展示和测试 UI。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DemoConfig {
    /// 是否启用演示模式，默认 false
    #[serde(default)]
    pub enabled: bool,
    /// 随机数种子，确保演示数据可复现，默认 20260511
    #[serde(default = "default_demo_seed")]
    pub seed: u64,
}

/// 演示模式随机种子默认值：20260511
fn default_demo_seed() -> u64 {
    20260511
}

/// 健康检查配置，定义测试翻译请求使用的语言对。
/// 用于 `POST /api/health/check` 端点主动探测上游可用性。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthCheckConfig {
    /// 健康检查翻译的源语言，默认 "EN"
    #[serde(default = "default_source_lang")]
    pub source_lang: String,
    /// 健康检查翻译的目标语言，默认 "ZH"
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
}

/// 健康检查源语言默认值："EN"（英语）
fn default_source_lang() -> String {
    "EN".to_string()
}

/// 健康检查目标语言默认值："ZH"（中文）
fn default_target_lang() -> String {
    "ZH".to_string()
}

/// 翻译缓存配置，基于 moka 实现 LRU 缓存。
/// 支持两种容量限制模式：
/// - **条目数限制**：`max_memory_mb = 0` 时生效，按 `max_entries` 限制缓存条目数量
/// - **内存限制**：`max_memory_mb > 0` 时生效，按内存用量（MB）限制缓存大小，忽略 `max_entries`
///
/// 两种模式均配合 TTL 过期策略，超过 `ttl_secs` 的缓存条目自动失效。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// 是否启用翻译缓存，默认 false
    #[serde(default)]
    pub enabled: bool,
    /// 缓存条目存活时间（秒），过期后自动淘汰，默认 3600（1 小时）
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,
    /// 最大缓存条目数，仅在 `max_memory_mb = 0` 时生效，默认 10000
    #[serde(default = "default_cache_max_entries")]
    pub max_entries: u64,
    /// 最大内存限制（MB），0 表示不限制内存，使用条目数限制
    #[serde(default)]
    pub max_memory_mb: u64,
}

/// 缓存 TTL 默认值：3600 秒（1 小时）
fn default_cache_ttl_secs() -> u64 {
    3600
}

/// 缓存最大条目数默认值：10000 条
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
            log_retention_days: default_log_retention_days(),
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
    /// 从指定路径加载并解析 TOML 配置文件。
    ///
    /// # 参数
    /// - `path`: 配置文件路径（通常为 `config.toml`）
    ///
    /// # 返回值
    /// - `Ok(Config)`: 解析成功的配置实例，缺失字段使用默认值填充
    /// - `Err(ConfigError::IoError)`: 文件读取失败（不存在、权限不足等）
    /// - `Err(ConfigError::ParseError)`: TOML 格式解析失败
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(ConfigError::IoError)?;
        toml::from_str(&content).map_err(ConfigError::ParseError)
    }

    /// 将当前配置序列化为 TOML 格式并写入指定路径。
    /// 使用 `toml::to_string_pretty` 生成人类可读的格式化输出。
    ///
    /// # 参数
    /// - `path`: 目标文件路径
    ///
    /// # 返回值
    /// - `Ok(())`: 写入成功
    /// - `Err(ConfigError::SerializeError)`: 序列化失败
    /// - `Err(ConfigError::IoError)`: 文件写入失败
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self).map_err(ConfigError::SerializeError)?;
        fs::write(path, content).map_err(ConfigError::IoError)
    }
}

/// 配置操作错误类型，涵盖文件 I/O、解析和序列化三类错误。
#[derive(Debug)]
pub enum ConfigError {
    /// 文件 I/O 错误（读取或写入失败），如文件不存在、权限不足
    IoError(std::io::Error),
    /// TOML 反序列化错误，配置文件格式不合法或字段类型不匹配
    ParseError(toml::de::Error),
    /// TOML 序列化错误，将 Config 结构体转为 TOML 字符串时失败
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
    use notify::{
        Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    };
    use std::time::Duration;
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel::<()>(1);

    // 使用标准 notify watcher，通过 channel 桥接到 tokio
    let path_for_watcher = std::path::PathBuf::from(&config_path);
    let canonical_path = path_for_watcher
        .canonicalize()
        .unwrap_or_else(|_| path_for_watcher.clone());

    let canonical_path_for_handler = canonical_path.clone();
    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // 只关注写入/修改/创建事件
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.proxy.host, "127.0.0.1");
        assert_eq!(config.proxy.port, 5555);
        assert_eq!(config.upstream.max_failures, 3);
        assert_eq!(config.upstream.probe_interval_secs, 60);
        assert_eq!(config.monitor.log_retention_days, 30);
        assert_eq!(config.cache.ttl_secs, 3600);
        assert_eq!(config.cache.max_entries, 10000);
        assert!(!config.cache.enabled);
        assert!(!config.demo.enabled);
    }

    #[test]
    fn test_load_save_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_config.toml");

        let mut config = Config::default();
        config.proxy.port = 9999;
        config.upstream.endpoints.push(EndpointConfig {
            name: "test".to_string(),
            url: "http://localhost:1188/translate".to_string(),
            api_key: "secret".to_string(),
        });
        config.cache.enabled = true;
        config.cache.ttl_secs = 7200;

        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();

        assert_eq!(loaded.proxy.port, 9999);
        assert_eq!(loaded.upstream.endpoints.len(), 1);
        assert_eq!(loaded.upstream.endpoints[0].name, "test");
        assert_eq!(
            loaded.upstream.endpoints[0].url,
            "http://localhost:1188/translate"
        );
        assert_eq!(loaded.upstream.endpoints[0].api_key, "secret");
        assert!(loaded.cache.enabled);
        assert_eq!(loaded.cache.ttl_secs, 7200);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = Config::load("/nonexistent/path/config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_partial_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("partial.toml");
        fs::write(
            &path,
            r#"
[proxy]
port = 8080
"#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.proxy.port, 8080);
        assert_eq!(config.proxy.host, "127.0.0.1"); // default
        assert_eq!(config.upstream.max_failures, 3); // default
    }

    #[test]
    fn test_load_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.toml");
        fs::write(&path, "this is not valid toml [[[").unwrap();

        let result = Config::load(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not found",
        ));
        assert!(err.to_string().contains("IO error"));
    }
}
