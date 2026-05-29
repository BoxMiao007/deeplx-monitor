//! 上游端点负载均衡模块。
//!
//! 本模块实现一个面向多个 DeepLX 上游端点的负载均衡器，主要特性：
//! - **轮询调度（round-robin）**：通过原子索引依次选择端点，分摊请求压力。
//! - **故障转移（failover）**：当某端点连续失败次数达到阈值时自动剔除，
//!   后续请求会跳过该端点直到它在探活中恢复。
//! - **last-known-good 兜底**：当所有端点都不健康时，仍会返回最近一次
//!   成功响应过的端点，避免完全没有上游可用。
//! - **运行时热重载**：支持在不重启进程的前提下替换端点列表。
//! - **统计计数持久化恢复**：支持将数据库中累计的请求量、成功数、
//!   延迟总和重新写回各端点状态，使得重启后总览统计保持连续。

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::config::EndpointConfig;
use crate::utils::chrono_now;

/// 端点对外暴露的运行时状态快照。
///
/// 通过 `LoadBalancer::status()` 返回，供 `/api/upstream` 等接口序列化为 JSON。
/// 字段都是某一时刻的瞬时值，并非持续追踪的引用。
#[derive(Debug, Clone, serde::Serialize)]
pub struct EndpointStatus {
    /// 端点显示名（若配置中未填写则会被自动赋值为 `endpoint-{n}`）。
    pub name: String,
    /// 上游 DeepLX 接口完整 URL。
    pub url: String,
    /// 当前是否健康（连续失败未超阈值）。
    pub healthy: bool,
    /// 当前连续失败次数；任意一次成功后清零。
    pub consecutive_failures: u32,
    /// 累计请求数（包含失败与重试）。
    pub total_requests: u64,
    /// 累计成功数。
    pub total_successes: u64,
    /// 平均延迟（毫秒），按成功请求计算（latency_sum_ms / total_successes）。
    pub avg_latency_ms: u64,
    /// 最近一次失败的错误描述，恢复后会被清空。
    pub last_error: Option<String>,
    /// 最近一次健康检查的时间戳（ISO 格式），未检测时为 None。
    pub last_check_at: Option<String>,
}

/// 一次手动健康检查（`check_all`）针对单个端点的执行结果。
///
/// 与 `EndpointStatus` 的区别在于：本结构仅描述本次探测，不包含累计统计。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EndpointHealthCheckResult {
    /// 端点名。
    pub name: String,
    /// 本次探测是否成功（HTTP 2xx 视为成功）。
    pub ok: bool,
    /// 本次探测耗时（毫秒）。
    pub latency_ms: u64,
    /// 失败时的错误描述；成功为 `None`。
    pub error: Option<String>,
}

/// 单个上游端点的内部运行时状态。
///
/// 该结构对外不可见，由 `LoadBalancer` 通过 `Arc` 持有以便共享。
/// 字段拆分为读写锁与原子计数：写入并发频次低的状态用 `RwLock`，
/// 高并发递增的计数器使用原子类型避免争用。
struct EndpointState {
    /// 端点不可变配置（URL、API key、显示名）。
    config: EndpointConfig,
    /// 是否仍被视为健康；`report_failure` 累计达到阈值时置 false，
    /// `report_success` 或探活成功时置 true。
    healthy: RwLock<bool>,
    /// 连续失败计数；成功一次即清零。
    consecutive_failures: RwLock<u32>,
    /// 累计请求总数（无论成败）。
    total_requests: AtomicU64,
    /// 累计成功请求数。
    total_successes: AtomicU64,
    /// 成功请求的延迟累加和（毫秒），用于计算平均延迟。
    latency_sum_ms: AtomicU64,
    /// 最近一次失败原因；成功后会被清空为 None。
    last_error: RwLock<Option<String>>,
    /// 最近一次健康检查的时间戳（ISO 格式）。
    last_check_at: RwLock<Option<String>>,
}

/// 多端点负载均衡器。
///
/// 负责在多个 DeepLX 上游之间进行调度，整体策略包含：
/// 1. **轮询（round-robin）**：每次 `select` 自增 `current_index` 并取模，
///    线性扫描首个健康端点返回。
/// 2. **故障转移（failover）**：`report_failure` 累计达 `max_failures` 时
///    将端点标记为不健康，后续 `select` 会跳过它；后台 `probe_unhealthy`
///    定期对不健康端点发送探活请求以尝试恢复。
/// 3. **last-known-good 兜底**：当所有端点都被标记为不健康时，
///    `select` 会回退到最近一次成功过的端点，最后兜底到索引 0，
///    确保至少返回一个候选，避免因瞬时全部异常而完全无法服务。
///
/// 所有字段使用内部可变性（RwLock / 原子）以便通过 `Arc<LoadBalancer>` 在
/// 多个 Tokio 任务间共享。
pub struct LoadBalancer {
    /// 当前端点列表；热重载时整体替换。
    endpoints: RwLock<Vec<Arc<EndpointState>>>,
    /// 轮询使用的全局索引；只递增不回绕，使用时再对端点数取模。
    current_index: AtomicUsize,
    /// 触发不健康标记的连续失败阈值（来自 `[health_check]` 配置，可热更新）。
    max_failures: RwLock<u32>,
    /// 最近一次成功响应的端点索引，用于全员不健康时的兜底选择。
    last_known_good: RwLock<Option<usize>>,
}

/// `select` 系列方法返回的端点选择结果。
///
/// 包含完成 HTTP 请求所需的全部上下文（URL、API key、显示名）以及
/// 用于事后回报状态的原始索引（`report_success` / `report_failure`）。
pub struct SelectedEndpoint {
    /// 上游 URL。
    url: String,
    /// 上游 API key（可能为空字符串）。
    api_key: String,
    /// 在端点列表中的下标，回报状态时用于定位 `EndpointState`。
    index: usize,
    /// 端点显示名，主要用于日志与统计聚合。
    name: String,
}

impl SelectedEndpoint {
    /// 返回上游 URL。
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 返回上游 API key（若未配置则为空字符串）。
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// 返回端点在列表中的索引，供 `report_success` / `report_failure` 定位。
    pub fn index(&self) -> usize {
        self.index
    }

    /// 返回端点显示名。
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl LoadBalancer {
    /// 创建新的负载均衡器实例。
    ///
    /// # 参数
    /// - `endpoints`：上游端点配置列表；若某端点 `name` 为空，
    ///   会自动赋值为 `endpoint-{序号}`（从 1 开始）。
    /// - `max_failures`：连续失败多少次后将端点标记为不健康。
    ///
    /// 初始状态下所有端点均视为健康，`last_known_good` 为 None。
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
                    config: EndpointConfig { name, ..config },
                    healthy: RwLock::new(true),
                    consecutive_failures: RwLock::new(0),
                    total_requests: AtomicU64::new(0),
                    total_successes: AtomicU64::new(0),
                    latency_sum_ms: AtomicU64::new(0),
                    last_error: RwLock::new(None),
                    last_check_at: RwLock::new(None),
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

    /// 热加载：用新的端点配置替换当前端点列表。
    ///
    /// 调用后所有端点状态（计数器、健康标记）重置为初始值，
    /// `current_index` 归零，`last_known_good` 清空。
    /// 适用于配置文件变更或 Web UI 修改端点后的即时生效。
    ///
    /// # 参数
    /// - `new_endpoints`：新的端点配置列表。
    /// - `max_failures`：新的连续失败阈值。
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
                    last_check_at: RwLock::new(None),
                })
            })
            .collect();

        *self.max_failures.write().await = max_failures;
        *self.endpoints.write().await = states;
        *self.last_known_good.write().await = None;
        self.current_index.store(0, Ordering::Relaxed);
        tracing::info!("LoadBalancer 已重新加载端点配置");
    }

    /// 返回当前端点总数。
    pub async fn endpoint_count(&self) -> usize {
        self.endpoints.read().await.len()
    }

    /// 选择下一个健康端点（round-robin + failover + last-known-good 兜底）。
    ///
    /// # 算法流程
    /// 1. 原子自增 `current_index` 并对端点数取模，得到起始位置。
    /// 2. 从起始位置开始线性扫描，返回第一个 `healthy == true` 的端点。
    /// 3. 若所有端点都不健康，尝试返回 `last_known_good` 记录的端点。
    /// 4. 若 `last_known_good` 也不可用，兜底返回索引 0 的端点。
    /// 5. 端点列表为空时返回 `None`。
    ///
    /// # 返回值
    /// - `Some(SelectedEndpoint)`：选中的端点信息，调用方用它发起请求后
    ///   需调用 `report_success` 或 `report_failure` 回报结果。
    /// - `None`：端点列表为空，无法选择。
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

    /// 选择下一个健康端点，排除指定索引。
    ///
    /// 用于 failover 场景：当首选端点请求失败后，调用方可用此方法
    /// 获取另一个健康端点进行重试，同时避免再次选中刚失败的那个。
    ///
    /// # 参数
    /// - `exclude`：需要排除的端点索引（通常是刚失败的端点）。
    ///
    /// # 返回值
    /// - `Some(SelectedEndpoint)`：找到了另一个健康端点。
    /// - `None`：只有一个端点或其余端点均不健康。
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

    /// 报告请求成功，更新端点统计与健康状态。
    ///
    /// # 状态转换
    /// - `total_requests` +1、`total_successes` +1、`latency_sum_ms` 累加本次延迟。
    /// - `consecutive_failures` 清零。
    /// - `healthy` 置为 true（即使之前被标记为不健康，一次成功即恢复）。
    /// - `last_error` 清空。
    /// - 更新全局 `last_known_good` 为该端点索引。
    ///
    /// # 参数
    /// - `endpoint`：由 `select` 返回的端点引用。
    /// - `latency_ms`：本次请求耗时（毫秒）。
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

    /// 报告请求失败，累计连续失败次数并可能触发不健康标记。
    ///
    /// # 状态转换
    /// - `total_requests` +1（失败也计入总请求数）。
    /// - `last_error` 更新为本次错误描述。
    /// - `consecutive_failures` +1。
    /// - 若 `consecutive_failures >= max_failures`，将 `healthy` 置为 false，
    ///   后续 `select` 会跳过该端点直到探活恢复。
    ///
    /// # 参数
    /// - `endpoint`：由 `select` 返回的端点引用。
    /// - `error`：本次失败的错误描述字符串。
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

    /// 后台探活：对所有端点发送测试翻译请求，更新健康状态与 `last_check_at`。
    ///
    /// # 探活机制
    /// 1. 遍历所有端点，逐一发送简短翻译请求（`"hi"` EN→ZH）。
    /// 2. 若收到 HTTP 2xx 响应：
    ///    - 清零 `consecutive_failures`，标记 `healthy = true`。
    ///    - 累加延迟和请求计数（探活也计入统计）。
    ///    - 若端点此前不健康，输出恢复日志。
    /// 3. 若响应非 2xx 或网络错误：更新 `last_error`，累加失败计数，
    ///    达到阈值时标记为不健康。
    /// 4. 无论成败，均更新 `last_check_at` 时间戳。
    ///
    /// 该方法由 `main.rs` 中的后台定时任务周期性调用。
    ///
    /// # 参数
    /// - `http_client`：共享的 reqwest 客户端实例。
    pub async fn probe_unhealthy(&self, http_client: &reqwest::Client) {
        let endpoints = self.endpoints.read().await;
        for ep in endpoints.iter() {
            let body = serde_json::json!({
                "text": "hi",
                "source_lang": "EN",
                "target_lang": "ZH"
            });

            let mut req = http_client.post(&ep.config.url).json(&body);
            if !ep.config.api_key.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", ep.config.api_key));
            }

            let was_healthy = *ep.healthy.read().await;
            let start = Instant::now();
            let send_result = req.send().await;
            *ep.last_check_at.write().await = Some(chrono_now());
            match send_result {
                Ok(resp) => {
                    let latency = start.elapsed().as_millis() as u64;
                    if resp.status().is_success() {
                        *ep.consecutive_failures.write().await = 0;
                        *ep.healthy.write().await = true;
                        *ep.last_error.write().await = None;
                        ep.latency_sum_ms.fetch_add(latency, Ordering::Relaxed);
                        ep.total_requests.fetch_add(1, Ordering::Relaxed);
                        ep.total_successes.fetch_add(1, Ordering::Relaxed);
                        if !was_healthy {
                            tracing::info!("Endpoint '{}' recovered", ep.config.name);
                        }
                    } else {
                        let error = format!("HTTP {}", resp.status().as_u16());
                        *ep.last_error.write().await = Some(error);
                        let mut failures = ep.consecutive_failures.write().await;
                        *failures += 1;
                        let max_failures = *self.max_failures.read().await;
                        if *failures >= max_failures {
                            *ep.healthy.write().await = false;
                        }
                    }
                }
                Err(e) => {
                    *ep.last_error.write().await = Some(format!("{}", e));
                    let mut failures = ep.consecutive_failures.write().await;
                    *failures += 1;
                    let max_failures = *self.max_failures.read().await;
                    if *failures >= max_failures {
                        *ep.healthy.write().await = false;
                    }
                }
            }
        }
    }

    /// 手动探活所有端点，并让状态立即反映本次探测结果。
    ///
    /// 与 `probe_unhealthy` 的区别：
    /// - 本方法对**所有**端点（包括健康的）发送探测请求。
    /// - 探测结果会立即更新端点健康状态（成功→恢复，失败→标记不健康）。
    /// - 返回每个端点的探测结果列表，供 Web UI 展示。
    ///
    /// 由 `POST /api/health/check` 触发。
    ///
    /// # 参数
    /// - `http_client`：共享的 reqwest 客户端实例。
    /// - `source_lang`：探测请求的源语言代码。
    /// - `target_lang`：探测请求的目标语言代码。
    ///
    /// # 返回值
    /// 每个端点的 `EndpointHealthCheckResult`，顺序与端点列表一致。
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

            let now = chrono_now();
            *ep.last_check_at.write().await = Some(now);

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

    /// 手动探活单个端点，按名称匹配。
    ///
    /// 与 `check_all` 类似但只针对一个端点，用于 Web UI 中的"刷新"按钮：
    /// - 命中端点：发送测试请求，更新该端点的健康状态、`last_check_at`，返回探测结果。
    /// - 未命中：返回 `None`。
    ///
    /// # 参数
    /// - `http_client`：共享的 reqwest 客户端实例。
    /// - `source_lang`：探测请求的源语言代码。
    /// - `target_lang`：探测请求的目标语言代码。
    /// - `name`：要探测的端点显示名。
    pub async fn check_one(
        &self,
        http_client: &reqwest::Client,
        source_lang: &str,
        target_lang: &str,
        name: &str,
    ) -> Option<EndpointHealthCheckResult> {
        let endpoints = self.endpoints.read().await.clone();
        let (idx, ep) = endpoints
            .iter()
            .enumerate()
            .find(|(_, e)| e.config.name == name)?;

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

        let now = chrono_now();
        *ep.last_check_at.write().await = Some(now);

        let outcome = match result {
            Ok(resp) if resp.status().is_success() => {
                ep.total_requests.fetch_add(1, Ordering::Relaxed);
                ep.total_successes.fetch_add(1, Ordering::Relaxed);
                ep.latency_sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
                *ep.consecutive_failures.write().await = 0;
                *ep.healthy.write().await = true;
                *ep.last_error.write().await = None;
                *self.last_known_good.write().await = Some(idx);
                EndpointHealthCheckResult {
                    name: ep.config.name.clone(),
                    ok: true,
                    latency_ms,
                    error: None,
                }
            }
            Ok(resp) => {
                let error = format!("HTTP {}", resp.status().as_u16());
                ep.total_requests.fetch_add(1, Ordering::Relaxed);
                *ep.consecutive_failures.write().await += 1;
                *ep.healthy.write().await = false;
                *ep.last_error.write().await = Some(error.clone());
                EndpointHealthCheckResult {
                    name: ep.config.name.clone(),
                    ok: false,
                    latency_ms,
                    error: Some(error),
                }
            }
            Err(e) => {
                let error = e.to_string();
                ep.total_requests.fetch_add(1, Ordering::Relaxed);
                *ep.consecutive_failures.write().await += 1;
                *ep.healthy.write().await = false;
                *ep.last_error.write().await = Some(error.clone());
                EndpointHealthCheckResult {
                    name: ep.config.name.clone(),
                    ok: false,
                    latency_ms,
                    error: Some(error),
                }
            }
        };

        Some(outcome)
    }

    /// 从数据库恢复端点统计计数器（重启后保持数据连续）。
    ///
    /// 应用启动时，`db.rs` 从 `stats_anchor` 表读取每个端点的累计统计，
    /// 然后调用本方法将这些值写回对应的 `EndpointState` 原子计数器。
    /// 这样即使进程重启，前端看到的总请求数、成功数、平均延迟也不会归零。
    ///
    /// # 参数
    /// - `stats`：元组切片 `(端点名, 累计请求数, 累计成功数, 延迟总和ms)`。
    ///   按端点名匹配；若某端点名在当前列表中不存在则跳过。
    pub async fn restore_stats(&self, stats: &[(String, u64, u64, u64)]) {
        let endpoints = self.endpoints.read().await;
        for (name, total_requests, total_successes, latency_sum_ms) in stats {
            if let Some(ep) = endpoints.iter().find(|e| &e.config.name == name) {
                ep.total_requests.store(*total_requests, Ordering::Relaxed);
                ep.total_successes
                    .store(*total_successes, Ordering::Relaxed);
                ep.latency_sum_ms.store(*latency_sum_ms, Ordering::Relaxed);
            }
        }
    }

    /// 演示模式：为所有端点填充预设的假统计数据。
    ///
    /// 用于 `[demo]` 模式下让仪表盘有数据可展示。
    /// 预设包含 4 种不同状态的端点模板，循环分配给实际端点。
    pub async fn seed_demo_stats(&self) {
        let endpoints = self.endpoints.read().await;
        // 为每个已配置的端点填充不同状态的假数据
        let presets: &[(bool, u32, u64, u64, u64)] = &[
            // (healthy, consecutive_failures, total_requests, total_successes, latency_sum_ms)
            (true, 0, 1248, 1241, 312_000), // 健康，低延迟
            (true, 1, 856, 843, 428_000),   // 健康但有偶发失败
            (false, 5, 412, 389, 825_000),  // 不健康
            (true, 0, 2104, 2098, 442_000), // 健康，高吞吐
        ];

        for (i, ep) in endpoints.iter().enumerate() {
            let preset = presets[i % presets.len()];
            let (healthy, fails, total, succ, lat_sum) = preset;
            *ep.healthy.write().await = healthy;
            *ep.consecutive_failures.write().await = fails;
            ep.total_requests.store(total, Ordering::Relaxed);
            ep.total_successes.store(succ, Ordering::Relaxed);
            ep.latency_sum_ms.store(lat_sum, Ordering::Relaxed);
            *ep.last_error.write().await = if healthy {
                None
            } else {
                Some("Connection timeout (demo)".to_string())
            };
        }

        // 标记一个 last-known-good
        if !endpoints.is_empty() {
            *self.last_known_good.write().await = Some(0);
        }
    }

    /// 获取所有端点的当前状态快照。
    ///
    /// 遍历端点列表，读取各字段并组装为 `EndpointStatus` 向量。
    /// 平均延迟按 `latency_sum_ms / total_successes` 计算；
    /// 若尚无成功请求则为 0。
    ///
    /// # 返回值
    /// 端点状态列表，顺序与内部端点列表一致。
    pub async fn status(&self) -> Vec<EndpointStatus> {
        let endpoints = self.endpoints.read().await;
        let mut result = Vec::with_capacity(endpoints.len());
        for ep in endpoints.iter() {
            let total = ep.total_requests.load(Ordering::Relaxed);
            let successes = ep.total_successes.load(Ordering::Relaxed);
            let latency_sum = ep.latency_sum_ms.load(Ordering::Relaxed);
            let avg_latency = latency_sum.checked_div(successes).unwrap_or(0);

            result.push(EndpointStatus {
                name: ep.config.name.clone(),
                url: ep.config.url.clone(),
                healthy: *ep.healthy.read().await,
                consecutive_failures: *ep.consecutive_failures.read().await,
                total_requests: total,
                total_successes: successes,
                avg_latency_ms: avg_latency,
                last_error: ep.last_error.read().await.clone(),
                last_check_at: ep.last_check_at.read().await.clone(),
            });
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_endpoints(n: usize) -> Vec<EndpointConfig> {
        (0..n)
            .map(|i| EndpointConfig {
                name: format!("ep-{}", i),
                url: format!("http://localhost:800{}/translate", i),
                api_key: format!("key-{}", i),
            })
            .collect()
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
        let ep0 = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 0,
            name: String::new(),
        };
        let ep1 = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 1,
            name: String::new(),
        };
        lb.report_failure(&ep0, "err").await;
        lb.report_failure(&ep1, "err").await;

        // 应该仍然返回一个端点（fallback）
        let result = lb.select().await;
        assert!(result.is_some(), "所有端点不健康时应 fallback");
    }

    #[tokio::test]
    async fn test_report_success_resets_failures() {
        let lb = LoadBalancer::new(make_endpoints(2), 3);

        let ep = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 0,
            name: String::new(),
        };
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

        let ep = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 0,
            name: String::new(),
        };
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

        let ep = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 0,
            name: String::new(),
        };
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
        let ep1 = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 1,
            name: String::new(),
        };
        lb.report_success(&ep1, 50).await;

        // 让所有端点不健康
        let ep0 = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 0,
            name: String::new(),
        };
        let ep2 = SelectedEndpoint {
            url: String::new(),
            api_key: String::new(),
            index: 2,
            name: String::new(),
        };
        lb.report_failure(&ep0, "err").await;
        // ep1 刚成功过，是健康的，需要让它也失败
        lb.report_failure(&ep1, "err").await;
        lb.report_failure(&ep2, "err").await;

        // 应该 fallback 到 last-known-good (index 1)
        let selected = lb.select().await.unwrap();
        assert_eq!(selected.index(), 1, "应 fallback 到 last-known-good");
    }
}
