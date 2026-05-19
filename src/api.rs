use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::{DailyStat, HourlyStat, HeatmapCell};
use crate::state::{AppState, HealthStatus};
use crate::utils::chrono_now;

/// 返回编译时嵌入的版本号
pub async fn version() -> impl IntoResponse {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_requests: i64,
    pub total_chars: i64,
    pub health: HealthStatus,
    pub config: ConfigInfo,
    pub period: PeriodStats,
}

#[derive(Debug, Serialize)]
pub struct PeriodStats {
    pub today_requests: i64,
    pub today_chars: i64,
    pub week_requests: i64,
    pub week_chars: i64,
    pub month_requests: i64,
    pub month_chars: i64,
}

#[derive(Debug, Serialize)]
pub struct ConfigInfo {
    pub upstream_url: String,
    pub api_key: String,
    pub auto_refresh_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub days: Option<u32>,
}

pub async fn stats(
    State(state): State<AppState>,
    Query(query): Query<StatsQuery>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    let upstream_url = config.upstream.endpoints.first()
        .map(|e| e.url.clone())
        .unwrap_or_default();
    let api_key = config.upstream.endpoints.first()
        .map(|e| e.api_key.clone())
        .unwrap_or_default();
    let auto_refresh = config.monitor.auto_refresh_seconds;
    drop(config);

    let db = state.db.clone();
    let query_days = query.days;
    let db_result = tokio::task::spawn_blocking(move || {
        let today = db.get_period_stats(1).unwrap_or((0, 0));
        let week = db.get_period_stats(7).unwrap_or((0, 0));
        let month = db.get_period_stats(30).unwrap_or((0, 0));
        let total = if let Some(days) = query_days {
            db.get_period_stats(days).unwrap_or((0, 0))
        } else {
            db.get_current_log_totals().unwrap_or((0, 0))
        };
        (today, week, month, total)
    }).await.unwrap_or(((0,0), (0,0), (0,0), (0,0)));

    let ((today_requests, today_chars), (week_requests, week_chars), (month_requests, month_chars), (total_requests, total_chars)) = db_result;

    let health = state.health.read().await.clone();

    Json(StatsResponse {
        total_requests,
        total_chars,
        health,
        config: ConfigInfo {
            upstream_url,
            api_key,
            auto_refresh_seconds: auto_refresh,
        },
        period: PeriodStats {
            today_requests,
            today_chars,
            week_requests,
            week_chars,
            month_requests,
            month_chars,
        },
    }).into_response()
}

#[derive(Debug, Serialize)]
pub struct ChartResponse {
    pub hourly: Vec<HourlyStat>,
    pub daily: Vec<DailyStat>,
}

pub async fn chart(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.clone();
    let (hourly, daily) = tokio::task::spawn_blocking(move || {
        let h = db.get_hourly_stats(24).unwrap_or_default();
        let d = db.get_daily_stats(30).unwrap_or_default();
        (h, d)
    }).await.unwrap_or_default();
    Json(ChartResponse { hourly, daily }).into_response()
}

#[derive(Debug, Deserialize)]
pub struct RequestsQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct RequestsResponse {
    pub items: Vec<crate::db::RequestLog>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

pub async fn requests(
    State(state): State<AppState>,
    Query(query): Query<RequestsQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(50, 200);

    let db = state.db.clone();
    let (items, total) = tokio::task::spawn_blocking(move || {
        db.get_requests(page, page_size).unwrap_or((vec![], 0))
    }).await.unwrap_or((vec![], 0));

    Json(RequestsResponse {
        items,
        total,
        page,
        page_size,
    }).into_response()
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.health.read().await.clone();
    Json(health).into_response()
}

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
    } else if let Some(min_latency) = results.iter().filter(|r| r.ok).map(|r| r.latency_ms).min()
    {
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

// --- 完整配置 API 结构体 ---

#[derive(Debug, Serialize, Deserialize)]
pub struct FullConfigInfo {
    pub upstream: UpstreamInfo,
    pub proxy: ProxyInfo,
    pub monitor: MonitorInfo,
    pub health_check: HealthCheckInfo,
    pub cache: CacheInfo,
    pub demo: DemoInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpstreamInfo {
    pub endpoints: Vec<EndpointInfo>,
    pub max_failures: u32,
    pub probe_interval_secs: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointInfo {
    pub name: String,
    pub url: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub auto_refresh_seconds: u64,
    pub max_log_entries: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckInfo {
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheInfo {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_entries: u64,
    pub max_memory_mb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DemoInfo {
    pub enabled: bool,
    pub seed: u64,
}

fn config_to_full_info(config: &crate::config::Config) -> FullConfigInfo {
    FullConfigInfo {
        upstream: UpstreamInfo {
            endpoints: config.upstream.endpoints.iter().map(|ep| EndpointInfo {
                name: ep.name.clone(),
                url: ep.url.clone(),
                api_key: ep.api_key.clone(),
            }).collect(),
            max_failures: config.upstream.max_failures,
            probe_interval_secs: config.upstream.probe_interval_secs,
        },
        proxy: ProxyInfo {
            host: config.proxy.host.clone(),
            port: config.proxy.port,
        },
        monitor: MonitorInfo {
            auto_refresh_seconds: config.monitor.auto_refresh_seconds,
            max_log_entries: config.monitor.max_log_entries,
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

pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.read().await;
    Json(config_to_full_info(&config)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct UpdateFullConfigPayload {
    pub upstream: Option<UpstreamInfo>,
    pub proxy: Option<ProxyInfo>,
    pub monitor: Option<MonitorInfo>,
    pub health_check: Option<HealthCheckInfo>,
    pub cache: Option<CacheInfo>,
    pub demo: Option<DemoInfo>,
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(payload): Json<UpdateFullConfigPayload>,
) -> impl IntoResponse {
    let mut config = state.config.write().await;

    if let Some(upstream) = payload.upstream {
        config.upstream.endpoints = upstream.endpoints.into_iter().map(|ep| {
            crate::config::EndpointConfig {
                name: ep.name,
                url: ep.url,
                api_key: ep.api_key,
            }
        }).collect();
        config.upstream.max_failures = upstream.max_failures;
        config.upstream.probe_interval_secs = upstream.probe_interval_secs;
    }
    if let Some(proxy) = payload.proxy {
        config.proxy.host = proxy.host;
        config.proxy.port = proxy.port;
    }
    if let Some(monitor) = payload.monitor {
        config.monitor.auto_refresh_seconds = monitor.auto_refresh_seconds;
        config.monitor.max_log_entries = monitor.max_log_entries;
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

    let config_path = std::env::var("DEEPLX_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());
    if let Err(e) = config.save(&config_path) {
        tracing::error!("Failed to save config: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
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
    state.cache.reload(cache_enabled, cache_ttl, cache_max, cache_max_memory_mb);

    Json(response).into_response()
}

#[derive(Debug, Deserialize)]
pub struct LangStatsQuery {
    pub days: Option<u32>,
}

pub async fn lang_stats(
    State(state): State<AppState>,
    Query(query): Query<LangStatsQuery>,
) -> impl IntoResponse {
    let db = state.db.clone();
    let days = query.days;
    let stats = tokio::task::spawn_blocking(move || {
        if let Some(d) = days {
            db.get_lang_stats_by_days(d).unwrap_or_default()
        } else {
            db.get_lang_stats().unwrap_or_default()
        }
    }).await.unwrap_or_default();
    Json(stats).into_response()
}

#[derive(Debug, Deserialize)]
pub struct LangHourlyStatsQuery {
    pub days: Option<u32>,
}

pub async fn lang_hourly_stats(
    State(state): State<AppState>,
    Query(query): Query<LangHourlyStatsQuery>,
) -> impl IntoResponse {
    let db = state.db.clone();
    let days = query.days;
    let stats = tokio::task::spawn_blocking(move || {
        if days.unwrap_or(1) > 1 {
            db.get_lang_daily_stats(days.unwrap_or(30)).unwrap_or_default()
        } else {
            db.get_lang_hourly_stats().unwrap_or_default()
        }
    }).await.unwrap_or_default();
    Json(stats).into_response()
}

// --- 新增端点 ---

pub async fn upstream_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.load_balancer.status().await;
    Json(status).into_response()
}

pub async fn cache_stats(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.cache.stats();
    Json(stats).into_response()
}

pub async fn clear_cache(State(state): State<AppState>) -> impl IntoResponse {
    state.cache.clear();
    Json(serde_json::json!({"ok": true})).into_response()
}

pub async fn cache_hit_logs(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.cache.hit_log()).into_response()
}

#[derive(Debug, Deserialize)]
pub struct HeatmapQuery {
    pub view: Option<String>,
    pub days: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct HeatmapResponse {
    pub view: String,
    pub data: Vec<HeatmapCell>,
}

pub async fn heatmap(
    State(state): State<AppState>,
    Query(query): Query<HeatmapQuery>,
) -> impl IntoResponse {
    let view = query.view.unwrap_or_else(|| "weekday".to_string());
    let days = query.days.unwrap_or(30);
    let db = state.db.clone();
    let view_clone = view.clone();

    let data = tokio::task::spawn_blocking(move || {
        if view_clone == "date" {
            db.get_heatmap_by_date(days).unwrap_or_default()
        } else {
            db.get_heatmap_by_weekday(days).unwrap_or_default()
        }
    }).await.unwrap_or_default();

    Json(HeatmapResponse { view, data }).into_response()
}

#[derive(Debug, Deserialize)]
pub struct ErrorTrendQuery {
    pub days: Option<u32>,
    pub granularity: Option<String>,
}

pub async fn error_trend(
    State(state): State<AppState>,
    Query(query): Query<ErrorTrendQuery>,
) -> impl IntoResponse {
    let days = query.days.unwrap_or(7);
    let granularity = query.granularity.unwrap_or_else(|| {
        if days <= 2 { "hourly".to_string() } else { "daily".to_string() }
    });
    let db = state.db.clone();

    let data = tokio::task::spawn_blocking(move || {
        if granularity == "hourly" {
            db.get_error_trend_hourly(days).unwrap_or_default()
        } else {
            db.get_error_trend_daily(days).unwrap_or_default()
        }
    }).await.unwrap_or_default();

    Json(data).into_response()
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub lang: Option<String>,
    pub status: Option<String>,
}

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
        ).unwrap_or_default()
    }).await.unwrap_or_default();

    if format == "csv" {
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(["id", "source_lang", "target_lang", "source_chars", "target_chars", "status", "error_msg", "created_at"]).ok();
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
            ]).ok();
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
