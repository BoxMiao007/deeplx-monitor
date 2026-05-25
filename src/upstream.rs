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
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EndpointHealthCheckResult {
    pub name: String,
    pub ok: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

struct EndpointState {
    config: EndpointConfig,
    healthy: RwLock<bool>,
    consecutive_failures: RwLock<u32>,
    total_requests: AtomicU64,
    total_successes: AtomicU64,
    latency_sum_ms: AtomicU64,
    last_error: RwLock<Option<String>>,
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
    name: String,
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

    pub fn name(&self) -> &str {
        &self.name
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
                    last_error: RwLock::new(None),
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
                    last_error: RwLock::new(None),
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
                    name: ep.config.name.clone(),
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
                    name: ep.config.name.clone(),
                });
            }
        }

        // 没有 last-known-good，尝试第一个
        let ep = &endpoints[0];
        Some(SelectedEndpoint {
            url: ep.config.url.clone(),
            api_key: ep.config.api_key.clone(),
            index: 0,
            name: ep.config.name.clone(),
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
                    name: ep.config.name.clone(),
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
        *ep.last_error.write().await = None;
        *self.last_known_good.write().await = Some(endpoint.index);
    }

    /// 报告请求失败
    pub async fn report_failure(&self, endpoint: &SelectedEndpoint, error: &str) {
        let endpoints = self.endpoints.read().await;
        if endpoint.index >= endpoints.len() {
            return;
        }
        let ep = &endpoints[endpoint.index];
        ep.total_requests.fetch_add(1, Ordering::Relaxed);
        *ep.last_error.write().await = Some(error.to_string());
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
            match req.send().await {
                Ok(resp) => {
                    let latency = start.elapsed().as_millis() as u64;
                    if resp.status().is_success() {
                        *ep.consecutive_failures.write().await = 0;
                        *ep.healthy.write().await = true;
                        *ep.last_error.write().await = None;
                        ep.latency_sum_ms.fetch_add(latency, Ordering::Relaxed);
                        ep.total_requests.fetch_add(1, Ordering::Relaxed);
                        ep.total_successes.fetch_add(1, Ordering::Relaxed);
                        tracing::info!("Endpoint '{}' recovered", ep.config.name);
                    } else {
                        *ep.last_error.write().await = Some(format!("HTTP {}", resp.status().as_u16()));
                    }
                }
                Err(e) => {
                    *ep.last_error.write().await = Some(format!("{}", e));
                }
            }
        }
    }

    /// 手动探活所有端点，并让状态立即反映本次探测结果。
    pub async fn check_all(
        &self,
        http_client: &reqwest::Client,
        source_lang: &str,
        target_lang: &str,
    ) -> Vec<EndpointHealthCheckResult> {
        let endpoints = self.endpoints.read().await.clone();
        let mut results = Vec::with_capacity(endpoints.len());

        for (idx, ep) in endpoints.iter().enumerate() {
            let body = serde_json::json!({
                "text": "hi",
                "source_lang": source_lang,
                "target_lang": target_lang
            });

            let mut req = http_client.post(&ep.config.url).json(&body);
            if !ep.config.api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", ep.config.api_key));
            }

            let start = Instant::now();
            let result = req.send().await;
            let latency_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(resp) if resp.status().is_success() => {
                    ep.total_requests.fetch_add(1, Ordering::Relaxed);
                    ep.total_successes.fetch_add(1, Ordering::Relaxed);
                    ep.latency_sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
                    *ep.consecutive_failures.write().await = 0;
                    *ep.healthy.write().await = true;
                    *ep.last_error.write().await = None;
                    *self.last_known_good.write().await = Some(idx);

                    results.push(EndpointHealthCheckResult {
                        name: ep.config.name.clone(),
                        ok: true,
                        latency_ms,
                        error: None,
                    });
                }
                Ok(resp) => {
                    let error = format!("HTTP {}", resp.status().as_u16());
                    ep.total_requests.fetch_add(1, Ordering::Relaxed);
                    *ep.consecutive_failures.write().await += 1;
                    *ep.healthy.write().await = false;
                    *ep.last_error.write().await = Some(error.clone());

                    results.push(EndpointHealthCheckResult {
                        name: ep.config.name.clone(),
                        ok: false,
                        latency_ms,
                        error: Some(error),
                    });
                }
                Err(e) => {
                    let error = e.to_string();
                    ep.total_requests.fetch_add(1, Ordering::Relaxed);
                    *ep.consecutive_failures.write().await += 1;
                    *ep.healthy.write().await = false;
                    *ep.last_error.write().await = Some(error.clone());

                    results.push(EndpointHealthCheckResult {
                        name: ep.config.name.clone(),
                        ok: false,
                        latency_ms,
                        error: Some(error),
                    });
                }
            }
        }

        results
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
            *ep.last_error.write().await = if healthy { None } else { Some("Connection timeout (demo)".to_string()) };
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
                last_error: ep.last_error.read().await.clone(),
            });
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_endpoints(n: usize) -> Vec<EndpointConfig> {
        (0..n).map(|i| EndpointConfig {
            name: format!("ep-{}", i),
            url: format!("http://localhost:800{}/translate", i),
            api_key: format!("key-{}", i),
        }).collect()
    }

    #[tokio::test]
    async fn test_select_round_robin() {
        let lb = LoadBalancer::new(make_endpoints(3), 3);

        let ep0 = lb.select().await.unwrap();
        let ep1 = lb.select().await.unwrap();
        let ep2 = lb.select().await.unwrap();
        let ep3 = lb.select().await.unwrap();

        assert_eq!(ep0.index(), 0);
        assert_eq!(ep1.index(), 1);
        assert_eq!(ep2.index(), 2);
        assert_eq!(ep3.index(), 0); // wraps around
    }

    #[tokio::test]
    async fn test_select_skips_unhealthy() {
        let lb = LoadBalancer::new(make_endpoints(3), 2);

        // 让 ep-0 失败到不健康
        let ep = lb.select().await.unwrap();
        assert_eq!(ep.index(), 0);
        lb.report_failure(&ep, "error1").await;
        lb.report_failure(&ep, "error2").await;

        // 下一次选择应跳过 ep-0
        let ep = lb.select().await.unwrap();
        assert_ne!(ep.index(), 0, "不健康端点应被跳过");
    }

    #[tokio::test]
    async fn test_all_unhealthy_falls_back() {
        let lb = LoadBalancer::new(make_endpoints(2), 1);

        // 让所有端点不健康
        let ep0 = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 0, name: String::new() };
        let ep1 = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 1, name: String::new() };
        lb.report_failure(&ep0, "err").await;
        lb.report_failure(&ep1, "err").await;

        // 应该仍然返回一个端点（fallback）
        let result = lb.select().await;
        assert!(result.is_some(), "所有端点不健康时应 fallback");
    }

    #[tokio::test]
    async fn test_report_success_resets_failures() {
        let lb = LoadBalancer::new(make_endpoints(2), 3);

        let ep = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 0, name: String::new() };
        lb.report_failure(&ep, "err1").await;
        lb.report_failure(&ep, "err2").await;
        lb.report_success(&ep, 100).await;

        // 端点应该恢复健康
        let status = lb.status().await;
        assert!(status[0].healthy);
        assert_eq!(status[0].consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_select_excluding() {
        let lb = LoadBalancer::new(make_endpoints(3), 3);

        let result = lb.select_excluding(0).await;
        assert!(result.is_some());
        assert_ne!(result.unwrap().index(), 0);
    }

    #[tokio::test]
    async fn test_select_excluding_single_endpoint() {
        let lb = LoadBalancer::new(make_endpoints(1), 3);
        let result = lb.select_excluding(0).await;
        assert!(result.is_none(), "单端点时排除后应返回 None");
    }

    #[tokio::test]
    async fn test_empty_endpoints() {
        let lb = LoadBalancer::new(vec![], 3);
        let result = lb.select().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_reload_resets_state() {
        let lb = LoadBalancer::new(make_endpoints(2), 3);

        let ep = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 0, name: String::new() };
        lb.report_failure(&ep, "err").await;
        lb.report_failure(&ep, "err").await;
        lb.report_failure(&ep, "err").await;

        // reload 后应该全部健康
        lb.reload(make_endpoints(2), 3).await;
        let status = lb.status().await;
        assert!(status[0].healthy);
        assert!(status[1].healthy);
    }

    #[tokio::test]
    async fn test_status_avg_latency() {
        let lb = LoadBalancer::new(make_endpoints(1), 3);

        let ep = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 0, name: String::new() };
        lb.report_success(&ep, 100).await;
        lb.report_success(&ep, 200).await;
        lb.report_success(&ep, 300).await;

        let status = lb.status().await;
        assert_eq!(status[0].avg_latency_ms, 200); // (100+200+300)/3
        assert_eq!(status[0].total_requests, 3);
        assert_eq!(status[0].total_successes, 3);
    }

    #[tokio::test]
    async fn test_last_known_good_fallback() {
        let lb = LoadBalancer::new(make_endpoints(3), 1);

        // 先让 ep-1 成功（设为 last-known-good）
        let ep1 = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 1, name: String::new() };
        lb.report_success(&ep1, 50).await;

        // 让所有端点不健康
        let ep0 = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 0, name: String::new() };
        let ep2 = SelectedEndpoint { url: String::new(), api_key: String::new(), index: 2, name: String::new() };
        lb.report_failure(&ep0, "err").await;
        // ep1 刚成功过，是健康的，需要让它也失败
        lb.report_failure(&ep1, "err").await;
        lb.report_failure(&ep2, "err").await;

        // 应该 fallback 到 last-known-good (index 1)
        let selected = lb.select().await.unwrap();
        assert_eq!(selected.index(), 1, "应 fallback 到 last-known-good");
    }
}
