//! API 模块 — 提供所有 `/api/*` 路由的 handler 函数。
//!
//! 本模块包含：
//! - 统计数据查询（总计、分时段、按端点筛选）
//! - 图表数据（小时/日维度）
//! - 请求日志分页查询
//! - 健康检查（读取缓存状态 / 主动探测所有上游）
//! - 完整配置的读取与热更新
//! - 语言统计、热力图、时间线、错误趋势等分析接口
//! - 数据导出（JSON / CSV 双格式）
//! - 上游状态、缓存统计与管理

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::{DailyStat, HeatmapCell, HourlyStat};
use crate::state::{AppState, HealthStatus};
use crate::utils::chrono_now;

/// 返回编译时嵌入的版本号
pub async fn version() -> impl IntoResponse {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// 统计数据响应体
///
/// 包含累计总量、健康状态、基础配置信息以及分时段统计。
/// 总量数据来源：
/// - 无筛选时：从 `stats_anchor` 表读取（不受日志清理影响）
/// - 按端点筛选时：从 `stats_anchor` 的 per-endpoint 计数器读取
/// - 指定 `days` 时：从 `translation_logs` 表按时间范围聚合
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    /// 累计总请求数
    pub total_requests: i64,
    /// 累计总字符数
    pub total_chars: i64,
    /// 当前健康状态（最近一次探测结果）
    pub health: HealthStatus,
    /// 基础配置摘要（首个上游地址、自动刷新间隔等）
    pub config: ConfigInfo,
    /// 分时段统计（今日/本周/本月）
    pub period: PeriodStats,
}

/// 分时段统计（今日/本周/本月的请求数和字符数）
///
/// 数据来源：`translation_logs` 表按 `created_at` 时间范围聚合。
/// 注意：日志清理后，历史时段数据可能不完整。
#[derive(Debug, Serialize)]
pub struct PeriodStats {
    /// 今日请求数
    pub today_requests: i64,
    /// 今日字符数
    pub today_chars: i64,
    /// 近7天请求数
    pub week_requests: i64,
    /// 近7天字符数
    pub week_chars: i64,
    /// 近30天请求数
    pub month_requests: i64,
    /// 近30天字符数
    pub month_chars: i64,
}

/// 基础配置摘要（返回给前端仪表盘使用）
#[derive(Debug, Serialize)]
pub struct ConfigInfo {
    /// 首个上游端点的 URL
    pub upstream_url: String,
    /// 首个上游端点的 API Key
    pub api_key: String,
    /// 前端自动刷新间隔（秒）
    pub auto_refresh_seconds: u64,
    /// 后台健康探测间隔（秒）
    pub probe_interval_secs: u64,
}

/// 统计查询参数
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// 指定天数范围（可选）；若提供则 total 从日志按天数聚合
    pub days: Option<u32>,
    /// 按端点名称筛选（可选）；若提供则仅统计该端点的数据
    pub endpoint: Option<String>,
}

/// GET /api/stats — 获取统计概览
///
/// 返回累计总量、分时段统计、健康状态和基础配置。
///
/// 总量（total_requests / total_chars）的数据来源逻辑：
/// 1. 指定 `days` 参数 → 从 `translation_logs` 按天数聚合
/// 2. 指定 `endpoint` 但无 `days` → 从 `stats_anchor` 读取该端点的累计计数
/// 3. 两者都未指定 → 从 `stats_anchor` 读取全局累计计数（不受日志清理影响）
///
/// 分时段统计（today/week/month）始终从 `translation_logs` 聚合。
pub async fn stats(
    State(state): State<AppState>,
    Query(query): Query<StatsQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    let upstream_url = config
        .upstream
        .endpoints
        .first()
        .map(|e| e.url.clone())
        .unwrap_or_default();
    let api_key = config
        .upstream
        .endpoints
        .first()
        .map(|e| e.api_key.clone())
        .unwrap_or_default();
    let auto_refresh = config.monitor.auto_refresh_seconds;
    let probe_interval = config.upstream.probe_interval_secs;
    drop(config);

    let db = state.db.clone();
    let query_days = query.days;
    let endpoint_filter = query.endpoint.clone().unwrap_or_default();
    let db_result = tokio::task::spawn_blocking(move || {
        let ep = if endpoint_filter.is_empty() {
            None
        } else {
            Some(endpoint_filter.as_str())
        };
        let today = db.get_period_stats_filtered(1, ep).unwrap_or((0, 0));
        let week = db.get_period_stats_filtered(7, ep).unwrap_or((0, 0));
        let month = db.get_period_stats_filtered(30, ep).unwrap_or((0, 0));
        let total = if let Some(days) = query_days {
            db.get_period_stats_filtered(days, ep).unwrap_or((0, 0))
        } else if ep.is_some() {
            db.get_endpoint_totals(ep.unwrap()).unwrap_or((0, 0))
        } else {
            db.get_current_log_totals().unwrap_or((0, 0))
        };
        (today, week, month, total)
    })
    .await
    .unwrap_or(((0, 0), (0, 0), (0, 0), (0, 0)));

    let (
        (today_requests, today_chars),
        (week_requests, week_chars),
        (month_requests, month_chars),
        (total_requests, total_chars),
    ) = db_result;

    let health = state.health.read().await.clone();

    Json(StatsResponse {
        total_requests,
        total_chars,
        health,
        config: ConfigInfo {
            upstream_url,
            api_key,
            auto_refresh_seconds: auto_refresh,
            probe_interval_secs: probe_interval,
        },
        period: PeriodStats {
            today_requests,
            today_chars,
            week_requests,
            week_chars,
            month_requests,
            month_chars,
        },
    })
    .into_response()
}

/// 图表数据响应体
#[derive(Debug, Serialize)]
pub struct ChartResponse {
    /// 最近24小时的逐小时统计
    pub hourly: Vec<HourlyStat>,
    /// 最近 N 天的逐日统计
    pub daily: Vec<DailyStat>,
}

/// 图表查询参数
#[derive(Debug, Deserialize)]
pub struct ChartQuery {
    /// 按端点名称筛选（可选）
    pub endpoint: Option<String>,
    /// 日统计的天数范围，默认30天
    pub days: Option<u32>,
}

/// GET /api/chart — 获取图表数据（小时维度 + 日维度）
///
/// 返回最近24小时的逐小时统计和最近 N 天的逐日统计。
/// 支持按端点筛选。
pub async fn chart(
    State(state): State<AppState>,
    Query(query): Query<ChartQuery>,
) -> impl IntoResponse {
    let db = state.db.clone();
    let endpoint_filter = query.endpoint.unwrap_or_default();
    let days = query.days.unwrap_or(30);
    let (hourly, daily) = tokio::task::spawn_blocking(move || {
        let ep = if endpoint_filter.is_empty() {
            None
        } else {
            Some(endpoint_filter.as_str())
        };
        let h = db.get_hourly_stats_filtered(24, ep).unwrap_or_default();
        let d = db.get_daily_stats_filtered(days, ep).unwrap_or_default();
        (h, d)
    })
    .await
    .unwrap_or_default();
    Json(ChartResponse { hourly, daily }).into_response()
}

/// 请求日志查询参数
#[derive(Debug, Deserialize)]
pub struct RequestsQuery {
    /// 页码，从1开始，默认1
    pub page: Option<u32>,
    /// 每页条数，默认50，范围 [1, 200]
    pub page_size: Option<u32>,
    /// 按端点名称筛选（可选）
    pub endpoint: Option<String>,
    /// 按状态筛选（可选），如 "success" / "error"
    pub status: Option<String>,
}

/// 请求日志分页响应体
#[derive(Debug, Serialize)]
pub struct RequestsResponse {
    /// 当前页的日志条目
    pub items: Vec<crate::db::RequestLog>,
    /// 符合筛选条件的总条数
    pub total: i64,
    /// 当前页码
    pub page: u32,
    /// 每页条数
    pub page_size: u32,
}

/// GET /api/requests — 分页查询翻译请求日志
///
/// 支持按端点和状态筛选，返回分页结果。
/// page_size 通过 clamp(1, 200) 限制范围，防止过大查询。
pub async fn requests(
    State(state): State<AppState>,
    Query(query): Query<RequestsQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let endpoint_filter = query.endpoint.unwrap_or_default();
    let status_filter = query.status.unwrap_or_default();

    let db = state.db.clone();
    let (items, total) = tokio::task::spawn_blocking(move || {
        let ep = if endpoint_filter.is_empty() {
            None
        } else {
            Some(endpoint_filter.as_str())
        };
        let st = if status_filter.is_empty() {
            None
        } else {
            Some(status_filter.as_str())
        };
        db.get_requests_filtered(page, page_size, ep, st)
            .unwrap_or((vec![], 0))
    })
    .await
    .unwrap_or((vec![], 0));

    Json(RequestsResponse {
        items,
        total,
        page,
        page_size,
    })
    .into_response()
}

/// GET /api/health — 获取缓存的健康状态（不触发探测）
///
/// 返回最近一次健康检查的结果，包含状态、延迟和检查时间。
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.health.read().await.clone();
    Json(health).into_response()
}

/// POST /api/health/check — 主动探测所有上游端点的健康状态
///
/// 向每个上游端点发送测试翻译请求，收集延迟和错误信息。
/// 探测完成后更新全局 HealthStatus：
/// - 至少一个端点成功 → status="ok"，latency_ms 取最小值
/// - 全部失败 → status="error"，附带第一个错误信息
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let source_lang = config.health_check.source_lang.clone();
    let target_lang = config.health_check.target_lang.clone();
    drop(config);

    let results = state
        .load_balancer
        .check_all(&state.http_client, &source_lang, &target_lang)
        .await;

    // 根据所有端点的探测结果推导整体健康状态
    let health = if results.is_empty() {
        HealthStatus {
            status: "error".to_string(),
            latency_ms: None,
            checked_at: Some(chrono_now()),
            error: Some("No upstream endpoints configured".to_string()),
        }
    } else if let Some(min_latency) = results.iter().filter(|r| r.ok).map(|r| r.latency_ms).min() {
        // 至少有一个端点成功
        HealthStatus {
            status: "ok".to_string(),
            latency_ms: Some(min_latency),
            checked_at: Some(chrono_now()),
            error: None,
        }
    } else {
        // 全部失败，取第一个错误
        let first_err = results
            .first()
            .and_then(|r| r.error.clone())
            .unwrap_or_else(|| "Unknown error".to_string());
        HealthStatus {
            status: "error".to_string(),
            latency_ms: results.first().map(|r| r.latency_ms),
            checked_at: Some(chrono_now()),
            error: Some(first_err),
        }
    };

    *state.health.write().await = health.clone();
    Json(health).into_response()
}

/// POST /api/health/check/:name — 主动探测单个上游端点
///
/// 向指定端点发送测试翻译请求，更新该端点的健康状态和 `last_check_at`。
/// 返回该端点的最新状态快照。
pub async fn health_check_one(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    let config = state.config.read().await.clone();
    let source_lang = config.health_check.source_lang.clone();
    let target_lang = config.health_check.target_lang.clone();
    drop(config);

    let result = state
        .load_balancer
        .check_one(&state.http_client, &source_lang, &target_lang, &name)
        .await;

    match result {
        Some(_) => {
            let status = state.load_balancer.status().await;
            let ep_status = status.into_iter().find(|s| s.name == name);
            Json(ep_status).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Endpoint not found"})),
        )
            .into_response(),
    }
}
// 以下结构体用于 GET/POST /api/config 的序列化/反序列化，
// 与 config.rs 中的内部配置结构一一对应但解耦（API 层独立定义）。

/// 完整配置信息（对应 config.toml 的所有 section）
#[derive(Debug, Serialize, Deserialize)]
pub struct FullConfigInfo {
    /// 上游端点配置
    pub upstream: UpstreamInfo,
    /// 代理监听地址配置（host + port，修改后需重启生效）
    pub proxy: ProxyInfo,
    /// 监控面板配置
    pub monitor: MonitorInfo,
    /// 健康检查使用的语言对
    pub health_check: HealthCheckInfo,
    /// 翻译缓存配置
    pub cache: CacheInfo,
    /// 演示模式配置
    pub demo: DemoInfo,
}

/// 上游端点组配置
#[derive(Debug, Serialize, Deserialize)]
pub struct UpstreamInfo {
    /// 端点列表
    pub endpoints: Vec<EndpointInfo>,
    /// 连续失败多少次后标记端点为不健康
    pub max_failures: u32,
    /// 不健康端点的探活间隔（秒）
    pub probe_interval_secs: u64,
}

/// 单个上游端点信息
#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointInfo {
    /// 端点显示名称
    pub name: String,
    /// DeepLX 服务的 URL
    pub url: String,
    /// 访问该端点所需的 API Key
    pub api_key: String,
}

/// 代理监听配置
///
/// 注意：修改 host/port 后需要重启服务才能生效。
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyInfo {
    /// 监听地址，如 "0.0.0.0"
    pub host: String,
    /// 监听端口，默认 5555
    pub port: u16,
}

/// 监控面板配置
#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// 前端自动刷新间隔（秒）
    pub auto_refresh_seconds: u64,
    /// 日志保留天数，超过此天数的日志会被后台任务清理
    pub log_retention_days: u32,
}

/// 健康检查语言对配置
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckInfo {
    /// 探测时使用的源语言代码（如 "EN"）
    pub source_lang: String,
    /// 探测时使用的目标语言代码（如 "ZH"）
    pub target_lang: String,
}

/// 翻译缓存配置
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheInfo {
    /// 是否启用缓存
    pub enabled: bool,
    /// 缓存条目的 TTL（秒）
    pub ttl_secs: u64,
    /// 最大缓存条目数
    pub max_entries: u64,
    /// 最大缓存内存占用（MB）
    pub max_memory_mb: u64,
}

/// 演示模式配置
#[derive(Debug, Serialize, Deserialize)]
pub struct DemoInfo {
    /// 是否启用演示模式（启动时填充假数据）
    pub enabled: bool,
    /// 随机数种子（确保演示数据可复现）
    pub seed: u64,
}

/// 将内部 Config 结构转换为 API 层的 FullConfigInfo
///
/// 用于 GET /api/config 响应和 POST /api/config 更新后的返回值。
fn config_to_full_info(config: &crate::config::Config) -> FullConfigInfo {
    FullConfigInfo {
        upstream: UpstreamInfo {
            endpoints: config
                .upstream
                .endpoints
                .iter()
                .map(|ep| EndpointInfo {
                    name: ep.name.clone(),
                    url: ep.url.clone(),
                    api_key: ep.api_key.clone(),
                })
                .collect(),
            max_failures: config.upstream.max_failures,
            probe_interval_secs: config.upstream.probe_interval_secs,
        },
        proxy: ProxyInfo {
            host: config.proxy.host.clone(),
            port: config.proxy.port,
        },
        monitor: MonitorInfo {
            auto_refresh_seconds: config.monitor.auto_refresh_seconds,
            log_retention_days: config.monitor.log_retention_days,
        },
        health_check: HealthCheckInfo {
            source_lang: config.health_check.source_lang.clone(),
            target_lang: config.health_check.target_lang.clone(),
        },
        cache: CacheInfo {
            enabled: config.cache.enabled,
            ttl_secs: config.cache.ttl_secs,
            max_entries: config.cache.max_entries,
            max_memory_mb: config.cache.max_memory_mb,
        },
        demo: DemoInfo {
            enabled: config.demo.enabled,
            seed: config.demo.seed,
        },
    }
}

/// GET /api/config — 获取当前完整配置
///
/// 返回所有配置 section 的当前值（包括运行时热加载后的最新状态）。
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.read().await;
    Json(config_to_full_info(&config)).into_response()
}

/// 配置更新请求体（所有字段均为 Option，支持部分更新）
#[derive(Debug, Deserialize)]
pub struct UpdateFullConfigPayload {
    /// 上游端点配置（可选）
    pub upstream: Option<UpstreamInfo>,
    /// 代理监听配置（可选，修改后需重启）
    pub proxy: Option<ProxyInfo>,
    /// 监控面板配置（可选）
    pub monitor: Option<MonitorInfo>,
    /// 健康检查配置（可选）
    pub health_check: Option<HealthCheckInfo>,
    /// 缓存配置（可选）
    pub cache: Option<CacheInfo>,
    /// 演示模式配置（可选）
    pub demo: Option<DemoInfo>,
}

/// POST /api/config — 更新配置并触发热加载
///
/// 热加载流程：
/// 1. 获取 config 写锁，将 payload 中非 None 的字段合并到内存配置
/// 2. 将更新后的配置序列化写入 config.toml 文件（持久化）
/// 3. 释放写锁后，调用 LoadBalancer.reload() 更新端点列表和失败阈值
/// 4. 调用 Cache.reload() 更新缓存参数（启用/TTL/容量/内存限制）
///
/// 注意：proxy.host / proxy.port 的变更需要重启服务才能生效。
pub async fn update_config(
    State(state): State<AppState>,
    Json(payload): Json<UpdateFullConfigPayload>,
) -> impl IntoResponse {
    let mut config = state.config.write().await;

    if let Some(upstream) = payload.upstream {
        config.upstream.endpoints = upstream
            .endpoints
            .into_iter()
            .map(|ep| crate::config::EndpointConfig {
                name: ep.name,
                url: ep.url,
                api_key: ep.api_key,
            })
            .collect();
        config.upstream.max_failures = upstream.max_failures;
        config.upstream.probe_interval_secs = upstream.probe_interval_secs;
    }
    if let Some(proxy) = payload.proxy {
        config.proxy.host = proxy.host;
        config.proxy.port = proxy.port;
    }
    if let Some(monitor) = payload.monitor {
        config.monitor.auto_refresh_seconds = monitor.auto_refresh_seconds;
        config.monitor.log_retention_days = monitor.log_retention_days;
    }
    if let Some(health_check) = payload.health_check {
        config.health_check.source_lang = health_check.source_lang;
        config.health_check.target_lang = health_check.target_lang;
    }
    if let Some(cache) = payload.cache {
        config.cache.enabled = cache.enabled;
        config.cache.ttl_secs = cache.ttl_secs;
        config.cache.max_entries = cache.max_entries;
        config.cache.max_memory_mb = cache.max_memory_mb;
    }
    if let Some(demo) = payload.demo {
        config.demo.enabled = demo.enabled;
        config.demo.seed = demo.seed;
    }

    let config_path = std::env::var("DEEPLX_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
    if let Err(e) = config.save(&config_path) {
        tracing::error!("Failed to save config: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }

    // 热加载 LoadBalancer 端点配置
    let endpoints = config.upstream.endpoints.clone();
    let max_failures = config.upstream.max_failures;

    // 热加载缓存配置
    let cache_enabled = config.cache.enabled;
    let cache_ttl = config.cache.ttl_secs;
    let cache_max = config.cache.max_entries;
    let cache_max_memory_mb = config.cache.max_memory_mb;

    let response = config_to_full_info(&config);
    drop(config); // 释放 config 写锁后再 await LB reload

    state.load_balancer.reload(endpoints, max_failures).await;
    state
        .cache
        .reload(cache_enabled, cache_ttl, cache_max, cache_max_memory_mb);

    Json(response).into_response()
}

/// 语言统计查询参数
#[derive(Debug, Deserialize)]
pub struct LangStatsQuery {
    /// 天数范围（可选）；若提供则仅统计该时间段内的语言分布
    pub days: Option<u32>,
    /// 按端点名称筛选（可选）
    pub endpoint: Option<String>,
}

/// GET /api/lang-stats — 获取语言对使用统计
///
/// 返回各源语言→目标语言组合的请求数和字符数。
/// 支持按天数范围和端点筛选。
pub async fn lang_stats(
    State(state): State<AppState>,
    Query(query): Query<LangStatsQuery>,
) -> impl IntoResponse {
    let db = state.db.clone();
    let days = query.days;
    let endpoint_filter = query.endpoint.unwrap_or_default();
    let stats = tokio::task::spawn_blocking(move || {
        let ep = if endpoint_filter.is_empty() {
            None
        } else {
            Some(endpoint_filter.as_str())
        };
        if let Some(d) = days {
            db.get_lang_stats_by_days_filtered(d, ep)
                .unwrap_or_default()
        } else {
            db.get_lang_stats_filtered(ep).unwrap_or_default()
        }
    })
    .await
    .unwrap_or_default();
    Json(stats).into_response()
}

/// 语言时间维度统计查询参数
#[derive(Debug, Deserialize)]
pub struct LangHourlyStatsQuery {
    /// 天数范围（可选）；days=1 返回逐小时数据，days>1 返回逐日数据
    pub days: Option<u32>,
}

/// GET /api/lang-hourly-stats — 获取语言使用的时间维度统计
///
/// - days <= 1（默认）：返回最近24小时的逐小时语言分布
/// - days > 1：返回最近 N 天的逐日语言分布
pub async fn lang_hourly_stats(
    State(state): State<AppState>,
    Query(query): Query<LangHourlyStatsQuery>,
) -> impl IntoResponse {
    let db = state.db.clone();
    let days = query.days;
    let stats = tokio::task::spawn_blocking(move || {
        if days.unwrap_or(1) > 1 {
            db.get_lang_daily_stats(days.unwrap_or(30))
                .unwrap_or_default()
        } else {
            db.get_lang_hourly_stats().unwrap_or_default()
        }
    })
    .await
    .unwrap_or_default();
    Json(stats).into_response()
}

// --- 上游状态与缓存管理端点 ---

/// GET /api/upstream/status — 获取所有上游端点的运行状态
///
/// 返回每个端点的名称、URL、健康状态、请求计数、成功率、平均延迟等。
pub async fn upstream_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.load_balancer.status().await;
    Json(status).into_response()
}

/// GET /api/cache/stats — 获取缓存统计信息
///
/// 返回缓存命中数、未命中数、当前条目数、命中率等。
pub async fn cache_stats(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.cache.stats();
    Json(stats).into_response()
}

/// POST /api/cache/clear — 清空翻译缓存
///
/// 立即清除所有缓存条目，返回 `{"ok": true}`。
pub async fn clear_cache(State(state): State<AppState>) -> impl IntoResponse {
    state.cache.clear();
    Json(serde_json::json!({"ok": true})).into_response()
}

/// GET /api/cache/hits — 获取最近的缓存命中日志
///
/// 返回最近 N 条缓存命中记录（文本摘要、语言对、命中时间）。
pub async fn cache_hit_logs(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.cache.hit_log()).into_response()
}

/// 热力图查询参数
#[derive(Debug, Deserialize)]
pub struct HeatmapQuery {
    /// 视图模式："weekday"（按星期几×小时）或 "date"（按日期×小时）
    pub view: Option<String>,
    /// 数据范围天数，默认30天
    pub days: Option<u32>,
    /// 按端点名称筛选（可选）
    pub endpoint: Option<String>,
}

/// 热力图响应体
#[derive(Debug, Serialize)]
pub struct HeatmapResponse {
    /// 当前视图模式
    pub view: String,
    /// 热力图单元格数据（x=小时, y=星期几或日期, value=请求数）
    pub data: Vec<HeatmapCell>,
}

/// GET /api/analytics/heatmap — 获取请求热力图数据
///
/// 支持两种视图：
/// - "weekday"（默认）：按星期几 × 小时聚合
/// - "date"：按日期 × 小时聚合
pub async fn heatmap(
    State(state): State<AppState>,
    Query(query): Query<HeatmapQuery>,
) -> impl IntoResponse {
    let view = query.view.unwrap_or_else(|| "weekday".to_string());
    let days = query.days.unwrap_or(30);
    let endpoint_filter = query.endpoint.unwrap_or_default();
    let db = state.db.clone();
    let view_clone = view.clone();

    let data = tokio::task::spawn_blocking(move || {
        let ep = if endpoint_filter.is_empty() {
            None
        } else {
            Some(endpoint_filter.as_str())
        };
        if view_clone == "date" {
            db.get_heatmap_by_date_filtered(days, ep)
                .unwrap_or_default()
        } else {
            db.get_heatmap_by_weekday_filtered(days, ep)
                .unwrap_or_default()
        }
    })
    .await
    .unwrap_or_default();

    Json(HeatmapResponse { view, data }).into_response()
}

/// 时间线查询参数
#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    /// 数据范围天数，默认7天
    pub days: Option<u32>,
}

/// GET /api/analytics/timeline — 获取翻译活动时间线
///
/// 返回指定天数内的活动块数据，用于前端时间线可视化。
pub async fn timeline(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> impl IntoResponse {
    let days = query.days.unwrap_or(7);
    let db = state.db.clone();

    let blocks =
        tokio::task::spawn_blocking(move || db.get_timeline_data(days).unwrap_or_default())
            .await
            .unwrap_or_default();

    Json(blocks).into_response()
}

/// 错误趋势查询参数
#[derive(Debug, Deserialize)]
pub struct ErrorTrendQuery {
    /// 数据范围天数，默认7天
    pub days: Option<u32>,
    /// 粒度："hourly" 或 "daily"；默认根据 days 自动选择（<=2天用hourly）
    pub granularity: Option<String>,
    /// 按端点名称筛选（可选）
    pub endpoint: Option<String>,
}

/// GET /api/analytics/error-trend — 获取错误趋势数据
///
/// 返回指定时间范围内的错误数量趋势，支持小时/日粒度。
/// 粒度自动选择逻辑：days <= 2 → hourly，days > 2 → daily。
pub async fn error_trend(
    State(state): State<AppState>,
    Query(query): Query<ErrorTrendQuery>,
) -> impl IntoResponse {
    let days = query.days.unwrap_or(7);
    let granularity = query.granularity.unwrap_or_else(|| {
        if days <= 2 {
            "hourly".to_string()
        } else {
            "daily".to_string()
        }
    });
    let endpoint_filter = query.endpoint.unwrap_or_default();
    let db = state.db.clone();

    let data = tokio::task::spawn_blocking(move || {
        let ep = if endpoint_filter.is_empty() {
            None
        } else {
            Some(endpoint_filter.as_str())
        };
        if granularity == "hourly" {
            db.get_error_trend_hourly_filtered(days, ep)
                .unwrap_or_default()
        } else {
            db.get_error_trend_daily_filtered(days, ep)
                .unwrap_or_default()
        }
    })
    .await
    .unwrap_or_default();

    Json(data).into_response()
}

/// 数据导出查询参数
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// 导出格式："json"（默认）或 "csv"
    pub format: Option<String>,
    /// 起始时间筛选（ISO 8601 格式，可选）
    pub start: Option<String>,
    /// 结束时间筛选（ISO 8601 格式，可选）
    pub end: Option<String>,
    /// 按语言筛选（可选），匹配 source_lang 或 target_lang
    pub lang: Option<String>,
    /// 按状态筛选（可选），如 "success" / "error"
    pub status: Option<String>,
}

/// GET /api/export — 导出翻译日志数据
///
/// 支持 JSON 和 CSV 两种格式：
/// - format=json（默认）：返回 JSON 数组，Content-Type: application/json
/// - format=csv：返回 CSV 文件，带 Content-Disposition 头触发浏览器下载
///
/// CSV 列：id, source_lang, target_lang, source_chars, target_chars, status, error_msg, created_at
pub async fn export(
    State(state): State<AppState>,
    Query(query): Query<ExportQuery>,
) -> impl IntoResponse {
    let format = query.format.unwrap_or_else(|| "json".to_string());
    let db = state.db.clone();

    let logs = tokio::task::spawn_blocking(move || {
        db.get_export_logs(
            query.start.as_deref(),
            query.end.as_deref(),
            query.lang.as_deref(),
            query.status.as_deref(),
        )
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    if format == "csv" {
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record([
            "id",
            "source_lang",
            "target_lang",
            "source_chars",
            "target_chars",
            "status",
            "error_msg",
            "created_at",
        ])
        .ok();
        for log in &logs {
            wtr.write_record([
                &log.id.to_string(),
                &log.source_lang,
                &log.target_lang,
                &log.source_chars.to_string(),
                &log.target_chars.to_string(),
                &log.status,
                log.error_msg.as_deref().unwrap_or(""),
                &log.created_at,
            ])
            .ok();
        }
        let csv_data = String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default();

        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/csv; charset=utf-8")
            .header("Content-Disposition", "attachment; filename=\"export.csv\"")
            .body(axum::body::Body::from(csv_data))
            .unwrap()
            .into_response()
    } else {
        Json(logs).into_response()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_page_size_clamp_allows_small_values() {
        // 验证修复后: clamp(1, 200) 允许 1-200 范围
        let test_cases: Vec<(u32, u32)> = vec![
            (1, 1),     // 用户请求 1 条/页 → 允许
            (10, 10),   // 用户请求 10 条/页 → 允许
            (20, 20),   // 用户请求 20 条/页 → 允许
            (50, 50),   // 用户请求 50 条/页 → 允许
            (100, 100), // 用户请求 100 条/页 → 允许
            (200, 200), // 用户请求 200 条/页 → 上限
            (300, 200), // 用户请求 300 条/页 → 强制为 200
            (0, 1),     // 用户请求 0 条/页 → 强制为 1
        ];

        for (input, expected) in test_cases {
            let result = input.clamp(1, 200);
            assert_eq!(
                result, expected,
                "page_size={} 应为 {}，实际为 {}",
                input, expected, result
            );
        }
    }
}
