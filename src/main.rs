//! DeepLX Monitor — 翻译代理与监控仪表盘的主入口。
//!
//! 启动流程：
//! 1. 初始化日志系统（tracing）
//! 2. 加载配置文件（config.toml）
//! 3. 初始化 SQLite 数据库，演示模式下填充假数据
//! 4. 构建 AppState（共享状态）
//! 5. 从数据库恢复缓存统计和端点统计（确保重启后数据不丢失）
//! 6. 启动后台任务（日志清理、配置文件监听、端点探活、缓存持久化）
//! 7. 注册路由（API + 翻译代理 + SPA 静态资源）
//! 8. 绑定端口并开始监听

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
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 嵌入前端构建产物（dist/ 目录）到二进制中
///
/// 使用 rust-embed 在编译时将 `dist/` 目录的所有文件打包进可执行文件，
/// 运行时无需额外的静态文件目录。
#[derive(Embed)]
#[folder = "dist/"]
struct Assets;

/// 应用主入口
///
/// 完整启动流程见模块级文档。使用 `#[tokio::main]` 启动异步运行时。
#[tokio::main]
async fn main() {
    // --- 阶段1：初始化日志系统 ---
    // 使用 tracing_subscriber，默认日志级别为 info（deeplx_monitor 模块）
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("deeplx_monitor=info".parse().unwrap()),
        )
        .init();

    // --- 阶段2：加载配置文件 ---
    // 优先从环境变量 DEEPLX_CONFIG 指定路径加载，否则使用当前目录的 config.toml
    let config_path = std::env::var("DEEPLX_CONFIG").unwrap_or_else(|_| "config.toml".to_string());

    let config = config::Config::load(&config_path).unwrap_or_else(|e| {
        tracing::warn!("Failed to load {}: {}, using defaults", config_path, e);
        config::Config::default()
    });

    // --- 阶段3：初始化数据库 ---
    let listen_addr = format!("{}:{}", config.proxy.host, config.proxy.port);
    let db = db::Database::new("deeplx-monitor.db").expect("Failed to open database");

    // 演示模式：用合成数据替换数据库内容，方便展示仪表盘功能
    if config.demo.enabled {
        let ep_names: Vec<String> = config
            .upstream
            .endpoints
            .iter()
            .map(|e| e.name.clone())
            .collect();
        db.replace_with_demo_data(config.demo.seed, &ep_names)
            .expect("Failed to seed demo database");
        tracing::info!("Demo mode enabled, seeded synthetic dashboard data");
    }

    // --- 阶段4：构建共享状态 ---
    let demo_enabled = config.demo.enabled;
    let state = state::AppState::new(config, db);

    // --- 阶段5：从数据库恢复持久化的统计数据 ---
    // 恢复缓存命中/未命中计数器（存储在 stats_anchor 表中）
    {
        let (hits, misses) = state.db.load_cache_stats();
        if hits > 0 || misses > 0 {
            state.cache.restore_stats(hits, misses);
            tracing::info!(
                "Restored cache stats from DB: hits={}, misses={}",
                hits,
                misses
            );
        }
    }

    // 恢复端点请求统计（请求数、成功数、延迟累计），确保重启后请求占比和平均延迟不清零
    {
        let ep_stats = state.db.load_endpoint_stats();
        if !ep_stats.is_empty() {
            state.load_balancer.restore_stats(&ep_stats).await;
            tracing::info!(
                "Restored endpoint stats from DB for {} endpoints",
                ep_stats.len()
            );
        }
    }

    // 演示模式：预填充缓存假数据
    if demo_enabled {
        let demo_translations = [
            (
                "Hello, world!",
                "EN",
                "ZH",
                r#"{"code":200,"data":"你好，世界！"}"#,
            ),
            (
                "Good morning",
                "EN",
                "ZH",
                r#"{"code":200,"data":"早上好"}"#,
            ),
            (
                "Thank you very much",
                "EN",
                "ZH",
                r#"{"code":200,"data":"非常感谢"}"#,
            ),
            (
                "How are you?",
                "EN",
                "ZH",
                r#"{"code":200,"data":"你好吗？"}"#,
            ),
            (
                "Machine learning",
                "EN",
                "ZH",
                r#"{"code":200,"data":"机器学习"}"#,
            ),
            (
                "今天天气真好",
                "ZH",
                "EN",
                r#"{"code":200,"data":"The weather is really nice today"}"#,
            ),
            (
                "人工智能",
                "ZH",
                "EN",
                r#"{"code":200,"data":"Artificial Intelligence"}"#,
            ),
            (
                "おはようございます",
                "JA",
                "ZH",
                r#"{"code":200,"data":"早上好"}"#,
            ),
            (
                "Bonjour le monde",
                "FR",
                "ZH",
                r#"{"code":200,"data":"你好世界"}"#,
            ),
            (
                "Guten Morgen",
                "DE",
                "ZH",
                r#"{"code":200,"data":"早上好"}"#,
            ),
        ];
        for (text, src, tgt, resp) in demo_translations {
            let json: serde_json::Value = serde_json::from_str(resp).unwrap();
            state.cache.insert(text, src, tgt, json);
        }
        tracing::info!(
            "Demo mode: pre-populated cache with {} entries",
            demo_translations.len()
        );
        // 触发缓存命中以填充命中日志
        for (text, src, tgt, _) in demo_translations {
            state.cache.get(text, src, tgt);
        }
        tracing::info!("Demo mode: seeded cache hit logs");
        state.load_balancer.seed_demo_stats().await;
        tracing::info!("Demo mode: seeded upstream endpoint stats");
    }

    // --- 阶段6：启动后台任务 ---

    // 后台任务1：定时清理过期日志（每小时一次，动态读取 log_retention_days）
    // 累计统计存储在 stats_anchor 表中，不会因日志清理而丢失
    let cleanup_db = Arc::clone(&state.db);
    let config_for_cleanup = Arc::clone(&state.config);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            let retention_days = config_for_cleanup.read().await.monitor.log_retention_days;
            match cleanup_db.cleanup_old_logs(retention_days) {
                Ok(count) if count > 0 => {
                    tracing::info!("清理了 {} 条超过 {} 天的旧日志", count, retention_days);
                }
                Err(e) => {
                    tracing::warn!("Failed to cleanup old logs: {}", e);
                }
                _ => {}
            }
        }
    });

    // 后台任务2：监听配置文件变更，自动热加载
    // 使用 notify crate 的文件系统事件，检测到 config.toml 修改后
    // 自动重新加载配置并触发 LoadBalancer 和 Cache 的 reload()
    {
        let config_for_watcher = Arc::clone(&state.config);
        let lb_for_watcher = Arc::clone(&state.load_balancer);
        let cache_for_watcher = Arc::clone(&state.cache);
        let path_for_watcher = config_path.clone();
        tokio::spawn(async move {
            config::watch_config_file(
                path_for_watcher,
                config_for_watcher,
                lb_for_watcher,
                cache_for_watcher,
            )
            .await;
        });
    }

    // 后台任务3：探活不健康端点（动态读取 probe_interval_secs）
    // 仅对标记为不健康的端点发送测试请求，成功后恢复为健康状态
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

    // 后台任务4：定时持久化缓存统计到数据库（每60秒一次）
    // 将内存中的 hits/misses 计数器写入 stats_anchor 表，防止重启丢失
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

    // --- 阶段7：注册路由 ---
    // 路由分为三类：
    // - POST /translate：翻译代理入口
    // - GET/POST /api/*：监控面板 API
    // - 静态资源 + SPA fallback：前端仪表盘
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
        .route("/api/analytics/timeline", get(api::timeline))
        .route("/api/analytics/error-trend", get(api::error_trend))
        .route("/api/version", get(api::version))
        .route("/api/export", get(api::export))
        .route("/favicon.svg", get(favicon_handler))
        .route("/assets/*path", get(asset_handler))
        .fallback(get(spa_handler))
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::mirror_request())
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state);

    // --- 阶段8：绑定端口并开始监听 ---
    let addr: SocketAddr = listen_addr.parse().expect("Invalid listen address");
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// SPA 路由 handler — 处理所有未匹配到 API/静态资源的请求
///
/// 路由逻辑：
/// 1. 如果请求路径包含文件扩展名（如 .js, .css），尝试从嵌入资源中提供
/// 2. 如果找到对应文件则直接返回
/// 3. 否则返回 index.html（支持前端 history mode 路由）
async fn spa_handler(uri: axum::http::Uri) -> Response {
    // 如果请求的是静态资源文件（有扩展名），尝试直接提供
    let path = uri.path().trim_start_matches('/');
    if !path.is_empty() && path.contains('.') {
        let resp = serve_file(path);
        if resp.status() != StatusCode::NOT_FOUND {
            return resp;
        }
    }
    // 否则返回 index.html（SPA history mode）
    serve_file("index.html")
}

/// GET /favicon.svg — 提供网站图标
async fn favicon_handler() -> Response {
    serve_file("favicon.svg")
}

/// GET /assets/*path — 提供 Vite 构建的静态资源（JS/CSS/图片等）
async fn asset_handler(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    let path = format!("assets/{}", path);
    serve_file(&path)
}

/// 从嵌入的静态资源中提供文件
///
/// 缓存策略：
/// - index.html：`no-cache, must-revalidate`（确保用户始终获取最新版本）
/// - 其他文件：`public, max-age=31536000, immutable`（Vite 构建产物带 hash，可永久缓存）
///
/// 如果文件不存在，返回 404 响应。
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
