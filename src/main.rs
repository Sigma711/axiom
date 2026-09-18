//! 启动 axum 服务器,监听 0.0.0.0:8080

use axiom::api;
use axiom::app_state::AppState;
use axiom::config::{default_config, load_config};
use axiom::paper::run_paper_loop;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 加载配置(config.yaml 不存在就用默认)
    let config = load_config(&PathBuf::from("config.yaml"))
        .unwrap_or_else(|_| {
            tracing::warn!("未找到 config.yaml,使用默认配置");
            default_config()
        });

    let data_cache_dir = PathBuf::from("data");
    std::fs::create_dir_all(&data_cache_dir).ok();

    let state = Arc::new(AppState::new(config, data_cache_dir));

    // 启动后台模拟盘循环
    let paper_clone = state.paper_state.clone();
    let feed_clone = state.feed.clone();
    tokio::spawn(async move {
        run_paper_loop(feed_clone, paper_clone).await;
    });

    let app = api::router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("◆ AXIOM 已启动,监听 {}", addr);
    tracing::info!("→ 打开浏览器访问 http://localhost:8080");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}