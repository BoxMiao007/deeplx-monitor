use crate::cache::TranslationCache;
use crate::config::Config;
use crate::db::Database;
use crate::upstream::LoadBalancer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// 应用全局共享状态
///
/// 通过 `Arc` 在多个 Axum handler 和后台任务之间共享。
/// 所有字段均为线程安全的智能指针，支持并发读写。
#[derive(Clone)]
pub struct AppState {
    /// 运行时配置，支持热重载（文件监听或 API 修改后写入）
    pub config: Arc<RwLock<Config>>,
    /// SQLite 数据库实例，内部使用 `Mutex<Connection>` 保证线程安全
    pub db: Arc<Database>,
    /// 上游翻译服务的健康状态（延迟、错误信息等）
    pub health: Arc<RwLock<HealthStatus>>,
    /// 复用的 HTTP 客户端，带连接池和超时配置
    pub http_client: reqwest::Client,
    /// 多上游负载均衡器（轮询 + 故障转移）
    pub load_balancer: Arc<LoadBalancer>,
    /// 翻译结果缓存（moka LRU + TTL）
    pub cache: Arc<TranslationCache>,
}

/// 上游翻译服务的健康检查状态
///
/// 通过定时探测或手动触发 `POST /api/health/check` 更新。
/// 序列化后返回给前端仪表盘展示。
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    /// 当前状态：`"healthy"` | `"unhealthy"` | `"unknown"`
    pub status: String,
    /// 最近一次探测的往返延迟（毫秒），未检测时为 None
    pub latency_ms: Option<u64>,
    /// 最近一次检查的时间戳（ISO 格式），未检测时为 None
    pub checked_at: Option<String>,
    /// 最近一次检查的错误信息，健康时为 None
    pub error: Option<String>,
}

impl Default for HealthStatus {
    /// 返回初始健康状态：状态为 `"unknown"`，其余字段为 None
    fn default() -> Self {
        Self {
            status: "unknown".to_string(),
            latency_ms: None,
            checked_at: None,
            error: None,
        }
    }
}

impl AppState {
    /// 根据配置和数据库实例构建应用状态
    ///
    /// # 参数
    /// - `config`: 从 `config.toml` 解析得到的完整配置
    /// - `db`: 已初始化的 SQLite 数据库实例
    ///
    /// # 初始化内容
    /// 1. 创建带 10 秒超时和连接池的 HTTP 客户端
    /// 2. 根据上游端点列表初始化负载均衡器
    /// 3. 根据缓存配置初始化翻译缓存
    /// 4. 将所有组件包装为 `Arc` 并组装为 `AppState`
    pub fn new(config: Config, db: Database) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(5)
            .build()
            .unwrap_or_default();

        let load_balancer = Arc::new(LoadBalancer::new(
            config.upstream.endpoints.clone(),
            config.upstream.max_failures,
        ));

        let cache = TranslationCache::new(
            config.cache.enabled,
            config.cache.ttl_secs,
            config.cache.max_entries,
            config.cache.max_memory_mb,
        );

        Self {
            config: Arc::new(RwLock::new(config)),
            db: Arc::new(db),
            health: Arc::new(RwLock::new(HealthStatus::default())),
            http_client,
            load_balancer,
            cache,
        }
    }
}
