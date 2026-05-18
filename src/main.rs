mod api;
mod cache;
mod config;
mod db;
mod proxy;
mod state;
mod upstream;
mod utils;

use axum::{
    body::Body,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use rust_embed::Embed;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{CorsLayer, AllowOrigin};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Embed)]
#[folder = "dist/"]
struct Assets;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("deeplx_monitor=info".parse().unwrap()),
        )
        .init();

    let config_path = std::env::var("DEEPLX_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());

    let config = config::Config::load(&config_path).unwrap_or_else(|e| {
        tracing::warn!("Failed to load {}: {}, using defaults", config_path, e);
        config::Config::default()
    });

    let listen_addr = format!("{}:{}", config.proxy.host, config.proxy.port);
    let db = db::Database::new("deeplx-monitor.db").expect("Failed to open database");
    if config.demo.enabled {
        db.replace_with_demo_data(config.demo.seed)
            .expect("Failed to seed demo database");
        tracing::info!("Demo mode enabled, seeded synthetic dashboard data");
    }

    let demo_enabled = config.demo.enabled;
    let state = state::AppState::new(config, db);

    // 启动时从数据库恢复缓存统计计数器
    {
        let (hits, misses) = state.db.load_cache_stats();
        if hits > 0 || misses > 0 {
            state.cache.restore_stats(hits, misses);
            tracing::info!("Restored cache stats from DB: hits={}, misses={}", hits, misses);
        }
    }

    // 演示模式：预填充缓存假数据
    if demo_enabled {
        let demo_translations = [
            ("Hello, world!", "EN", "ZH", r#"{"code":200,"data":"你好，世界！"}"#),
            ("Good morning", "EN", "ZH", r#"{"code":200,"data":"早上好"}"#),
            ("Thank you very much", "EN", "ZH", r#"{"code":200,"data":"非常感谢"}"#),
            ("How are you?", "EN", "ZH", r#"{"code":200,"data":"你好吗？"}"#),
            ("Machine learning", "EN", "ZH", r#"{"code":200,"data":"机器学习"}"#),
            ("今天天气真好", "ZH", "EN", r#"{"code":200,"data":"The weather is really nice today"}"#),
            ("人工智能", "ZH", "EN", r#"{"code":200,"data":"Artificial Intelligence"}"#),
            ("おはようございます", "JA", "ZH", r#"{"code":200,"data":"早上好"}"#),
            ("Bonjour le monde", "FR", "ZH", r#"{"code":200,"data":"你好世界"}"#),
            ("Guten Morgen", "DE", "ZH", r#"{"code":200,"data":"早上好"}"#),
        ];
        for (text, src, tgt, resp) in demo_translations {
            let json: serde_json::Value = serde_json::from_str(resp).unwrap();
            state.cache.insert(text, src, tgt, json);
        }
        tracing::info!("Demo mode: pre-populated cache with {} entries", demo_translations.len());
        // 触发缓存命中以填充命中日志
        for (text, src, tgt, _) in demo_translations {
            state.cache.get(text, src, tgt);
        }
        tracing::info!("Demo mode: seeded cache hit logs");
        state.load_balancer.seed_demo_stats().await;
        tracing::info!("Demo mode: seeded upstream endpoint stats");
    }

    // 后台定时清理过期日志（每小时一次，动态读取 retention_days）
    let cleanup_db = Arc::clone(&state.db);
    let config_for_cleanup = Arc::clone(&state.config);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            let days = config_for_cleanup.read().await.monitor.retention_days;
            match cleanup_db.cleanup_old_logs(days) {
                Ok(count) if count > 0 => {
                    tracing::info!("Cleaned up {} old log entries", count);
                }
                Err(e) => {
                    tracing::warn!("Failed to cleanup old logs: {}", e);
                }
                _ => {}
            }
        }
    });

    // 后台监听配置文件变更，自动热加载
    {
        let config_for_watcher = Arc::clone(&state.config);
        let lb_for_watcher = Arc::clone(&state.load_balancer);
        let cache_for_watcher = Arc::clone(&state.cache);
        let path_for_watcher = config_path.clone();
        tokio::spawn(async move {
            config::watch_config_file(path_for_watcher, config_for_watcher, lb_for_watcher, cache_for_watcher).await;
        });
    }

    // 后台探活不健康端点（动态读取 probe_interval_secs）
    if state.load_balancer.endpoint_count().await > 0 {
        let lb = Arc::clone(&state.load_balancer);
        let client = state.http_client.clone();
        let config_for_probe = Arc::clone(&state.config);
        tokio::spawn(async move {
            loop {
                let interval = config_for_probe.read().await.upstream.probe_interval_secs;
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                lb.probe_unhealthy(&client).await;
            }
        });
    }

    // 后台定时持久化缓存统计到数据库（每60秒一次）
    {
        let cache_for_persist = Arc::clone(&state.cache);
        let db_for_persist = Arc::clone(&state.db);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let (hits, misses) = cache_for_persist.get_raw_stats();
                if let Err(e) = db_for_persist.save_cache_stats(hits, misses) {
                    tracing::warn!("Failed to persist cache stats: {}", e);
                }
            }
        });
    }

    let app = Router::new()
        .route("/translate", post(proxy::translate))
        .route("/api/stats", get(api::stats))
        .route("/api/chart", get(api::chart))
        .route("/api/requests", get(api::requests))
        .route("/api/health", get(api::health))
        .route("/api/health/check", post(api::health_check))
        .route("/api/config", get(api::get_config))
        .route("/api/config", post(api::update_config))
        .route("/api/lang-stats", get(api::lang_stats))
        .route("/api/lang-hourly-stats", get(api::lang_hourly_stats))
        .route("/api/upstream/status", get(api::upstream_status))
        .route("/api/cache/stats", get(api::cache_stats))
        .route("/api/cache/clear", post(api::clear_cache))
        .route("/api/cache/hits", get(api::cache_hit_logs))
        .route("/api/analytics/heatmap", get(api::heatmap))
        .route("/api/analytics/error-trend", get(api::error_trend))
        .route("/api/export", get(api::export))
        .route("/favicon.svg", get(favicon_handler))
        .route("/", get(spa_handler))
        .route("/assets/*path", get(asset_handler))
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::mirror_request())
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state);

    let addr: SocketAddr = listen_addr.parse().expect("Invalid listen address");
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn spa_handler() -> Response {
    serve_file("index.html")
}

async fn favicon_handler() -> Response {
    serve_file("favicon.svg")
}

async fn asset_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let path = format!("assets/{}", path);
    serve_file(&path)
}

fn serve_file(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let body: Body = content.data.into();
            let cache_value = if path == "index.html" {
                "no-cache, must-revalidate"
            } else {
                "public, max-age=31536000, immutable"
            };
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", mime.as_ref())
                .header("Cache-Control", cache_value)
                .body(body)
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(format!("{} not found", path)))
            .unwrap(),
    }
}
