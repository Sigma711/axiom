//! 启动 axum 服务器,监听 0.0.0.0:8080

use axiom::api;
use axiom::app_state::AppState;
use axiom::config::{default_config, load_config};
use axiom::paper::run_market_paper_loop;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 加载配置(config.yaml 不存在就用默认)
    let config = load_config(&PathBuf::from("config.yaml")).unwrap_or_else(|_| {
        tracing::warn!("未找到 config.yaml,使用默认配置");
        default_config()
    });

    let data_cache_dir =
        PathBuf::from(std::env::var("AXIOM_DATA_DIR").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data_cache_dir).ok();

    let state = Arc::new(AppState::new(config, data_cache_dir));

    // 启动后台模拟盘循环
    let paper_clone = state.paper_state.clone();
    let feed_clone = state.feed.clone();
    if std::env::var("AXIOM_OFFLINE").as_deref() == Ok("1") {
        tokio::spawn(axiom::paper::run_offline_paper_loop(paper_clone));
    } else {
        tokio::spawn(async move {
            run_market_paper_loop(feed_clone, paper_clone).await;
        });
    }

    let app = api::router(state);
    let port: u16 = std::env::var("AXIOM_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()?;
    let host: IpAddr = std::env::var("AXIOM_HOST")
        .unwrap_or_else(|_| "0.0.0.0".into())
        .parse()?;
    let addr = SocketAddr::new(host, port);
    tracing::info!("◆ AXIOM 已启动,监听 {}", addr);
    tracing::info!("→ 打开浏览器访问 http://localhost:{port}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut terminate = signal(SignalKind::terminate()).expect("register SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await.expect("register Ctrl-C");
}
