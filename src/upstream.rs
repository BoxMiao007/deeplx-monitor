use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::config::EndpointConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct EndpointStatus {
    pub name: String,
    pub url: String,
    pub healthy: bool,
    pub consecutive_failures: u32,
    pub total_requests: u64,
    pub total_successes: u64,
    pub avg_latency_ms: u64,
}

struct EndpointState {
    config: EndpointConfig,
    healthy: RwLock<bool>,
    consecutive_failures: RwLock<u32>,
    total_requests: AtomicU64,
    total_successes: AtomicU64,
    latency_sum_ms: AtomicU64,
}

pub struct LoadBalancer {
    endpoints: RwLock<Vec<Arc<EndpointState>>>,
    current_index: AtomicUsize,
    max_failures: RwLock<u32>,
    last_known_good: RwLock<Option<usize>>,
}

pub struct SelectedEndpoint {
    url: String,
    api_key: String,
    index: usize,
}

impl SelectedEndpoint {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

impl LoadBalancer {
    pub fn new(endpoints: Vec<EndpointConfig>, max_failures: u32) -> Self {
        let states: Vec<Arc<EndpointState>> = endpoints
            .into_iter()
            .enumerate()
            .map(|(i, config)| {
                let name = if config.name.is_empty() {
                    format!("endpoint-{}", i + 1)
                } else {
                    config.name.clone()
                };
                Arc::new(EndpointState {
                    config: EndpointConfig {
                        name,
                        ..config
                    },
                    healthy: RwLock::new(true),
                    consecutive_failures: RwLock::new(0),
                    total_requests: AtomicU64::new(0),
                    total_successes: AtomicU64::new(0),
                    latency_sum_ms: AtomicU64::new(0),
                })
            })
            .collect();

        Self {
            endpoints: RwLock::new(states),
            current_index: AtomicUsize::new(0),
            max_failures: RwLock::new(max_failures),
            last_known_good: RwLock::new(None),
        }
    }

    /// 热加载：用新的端点配置替换当前端点列表
    pub async fn reload(&self, new_endpoints: Vec<EndpointConfig>, max_failures: u32) {
        let states: Vec<Arc<EndpointState>> = new_endpoints
            .into_iter()
            .enumerate()
            .map(|(i, config)| {
                let name = if config.name.is_empty() {
                    format!("endpoint-{}", i + 1)
                } else {
                    config.name.clone()
                };
                Arc::new(EndpointState {
                    config: EndpointConfig { name, ..config },
                    healthy: RwLock::new(true),
                    consecutive_failures: RwLock::new(0),
                    total_requests: AtomicU64::new(0),
                    total_successes: AtomicU64::new(0),
                    latency_sum_ms: AtomicU64::new(0),
                })
            })
            .collect();

        *self.max_failures.write().await = max_failures;
        *self.endpoints.write().await = states;
        *self.last_known_good.write().await = None;
        self.current_index.store(0, Ordering::Relaxed);
        tracing::info!("LoadBalancer 已重新加载端点配置");
    }

    pub async fn endpoint_count(&self) -> usize {
        self.endpoints.read().await.len()
    }

    /// 选择下一个健康端点（round-robin）
    pub async fn select(&self) -> Option<SelectedEndpoint> {
        let endpoints = self.endpoints.read().await;
        let count = endpoints.len();
        if count == 0 {
            return None;
        }

        let start = self.current_index.fetch_add(1, Ordering::Relaxed) % count;

        // 尝试找到一个健康端点
        for i in 0..count {
            let idx = (start + i) % count;
            let ep = &endpoints[idx];
            if *ep.healthy.read().await {
                return Some(SelectedEndpoint {
                    url: ep.config.url.clone(),
                    api_key: ep.config.api_key.clone(),
                    index: idx,
                });
            }
        }

        // 所有端点不健康，fallback 到 last-known-good
        if let Some(idx) = *self.last_known_good.read().await {
            if idx < count {
                let ep = &endpoints[idx];
                return Some(SelectedEndpoint {
                    url: ep.config.url.clone(),
                    api_key: ep.config.api_key.clone(),
                    index: idx,
                });
            }
        }

        // 没有 last-known-good，尝试第一个
        let ep = &endpoints[0];
        Some(SelectedEndpoint {
            url: ep.config.url.clone(),
            api_key: ep.config.api_key.clone(),
            index: 0,
        })
    }

    /// 选择下一个健康端点，排除指定索引
    pub async fn select_excluding(&self, exclude: usize) -> Option<SelectedEndpoint> {
        let endpoints = self.endpoints.read().await;
        let count = endpoints.len();
        if count <= 1 {
            return None;
        }

        for i in 1..count {
            let idx = (exclude + i) % count;
            let ep = &endpoints[idx];
            if *ep.healthy.read().await {
                return Some(SelectedEndpoint {
                    url: ep.config.url.clone(),
                    api_key: ep.config.api_key.clone(),
                    index: idx,
                });
            }
        }

        None
    }

    /// 报告请求成功
    pub async fn report_success(&self, endpoint: &SelectedEndpoint, latency_ms: u64) {
        let endpoints = self.endpoints.read().await;
        if endpoint.index >= endpoints.len() {
            return;
        }
        let ep = &endpoints[endpoint.index];
        ep.total_requests.fetch_add(1, Ordering::Relaxed);
        ep.total_successes.fetch_add(1, Ordering::Relaxed);
        ep.latency_sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
        *ep.consecutive_failures.write().await = 0;
        *ep.healthy.write().await = true;
        *self.last_known_good.write().await = Some(endpoint.index);
    }

    /// 报告请求失败
    pub async fn report_failure(&self, endpoint: &SelectedEndpoint) {
        let endpoints = self.endpoints.read().await;
        if endpoint.index >= endpoints.len() {
            return;
        }
        let ep = &endpoints[endpoint.index];
        ep.total_requests.fetch_add(1, Ordering::Relaxed);
        let mut failures = ep.consecutive_failures.write().await;
        *failures += 1;
        let max_failures = *self.max_failures.read().await;
        if *failures >= max_failures {
            *ep.healthy.write().await = false;
        }
    }

    /// 探活所有不健康端点
    pub async fn probe_unhealthy(&self, http_client: &reqwest::Client) {
        let endpoints = self.endpoints.read().await;
        for ep in endpoints.iter() {
            if *ep.healthy.read().await {
                continue;
            }

            let body = serde_json::json!({
                "text": "hi",
                "source_lang": "EN",
                "target_lang": "ZH"
            });

            let mut req = http_client.post(&ep.config.url).json(&body);
            if !ep.config.api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", ep.config.api_key));
            }

            let start = Instant::now();
            if let Ok(resp) = req.send().await {
                let latency = start.elapsed().as_millis() as u64;
                if resp.status().is_success() {
                    *ep.consecutive_failures.write().await = 0;
                    *ep.healthy.write().await = true;
                    ep.latency_sum_ms.fetch_add(latency, Ordering::Relaxed);
                    ep.total_requests.fetch_add(1, Ordering::Relaxed);
                    ep.total_successes.fetch_add(1, Ordering::Relaxed);
                    tracing::info!("Endpoint '{}' recovered", ep.config.name);
                }
            }
        }
    }

    /// 演示模式：填充假统计数据
    pub async fn seed_demo_stats(&self) {
        let endpoints = self.endpoints.read().await;
        // 为每个已配置的端点填充不同状态的假数据
        let presets: &[(bool, u32, u64, u64, u64)] = &[
            // (healthy, consecutive_failures, total_requests, total_successes, latency_sum_ms)
            (true,  0, 1248,  1241, 312_000),   // 健康，低延迟
            (true,  1, 856,   843,  428_000),   // 健康但有偶发失败
            (false, 5, 412,   389,  825_000),   // 不健康
            (true,  0, 2104,  2098, 442_000),   // 健康，高吞吐
        ];

        for (i, ep) in endpoints.iter().enumerate() {
            let preset = presets[i % presets.len()];
            let (healthy, fails, total, succ, lat_sum) = preset;
            *ep.healthy.write().await = healthy;
            *ep.consecutive_failures.write().await = fails;
            ep.total_requests.store(total, Ordering::Relaxed);
            ep.total_successes.store(succ, Ordering::Relaxed);
            ep.latency_sum_ms.store(lat_sum, Ordering::Relaxed);
        }

        // 标记一个 last-known-good
        if !endpoints.is_empty() {
            *self.last_known_good.write().await = Some(0);
        }
    }

    /// 获取所有端点状态
    pub async fn status(&self) -> Vec<EndpointStatus> {
        let endpoints = self.endpoints.read().await;
        let mut result = Vec::with_capacity(endpoints.len());
        for ep in endpoints.iter() {
            let total = ep.total_requests.load(Ordering::Relaxed);
            let successes = ep.total_successes.load(Ordering::Relaxed);
            let latency_sum = ep.latency_sum_ms.load(Ordering::Relaxed);
            let avg_latency = if successes > 0 { latency_sum / successes } else { 0 };

            result.push(EndpointStatus {
                name: ep.config.name.clone(),
                url: ep.config.url.clone(),
                healthy: *ep.healthy.read().await,
                consecutive_failures: *ep.consecutive_failures.read().await,
                total_requests: total,
                total_successes: successes,
                avg_latency_ms: avg_latency,
            });
        }
        result
    }
}
