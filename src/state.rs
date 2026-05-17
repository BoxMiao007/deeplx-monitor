use crate::cache::TranslationCache;
use crate::config::Config;
use crate::db::Database;
use crate::upstream::LoadBalancer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub db: Arc<Database>,
    pub health: Arc<RwLock<HealthStatus>>,
    pub http_client: reqwest::Client,
    pub load_balancer: Arc<LoadBalancer>,
    pub cache: Arc<TranslationCache>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub checked_at: Option<String>,
    pub error: Option<String>,
}

impl Default for HealthStatus {
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
