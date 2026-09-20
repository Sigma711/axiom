//! 全局应用状态 —— 在 axum 里通过 State 共享给所有处理器。

use crate::config::AppConfig;
use crate::data::HttpFeed;
use crate::paper::{PaperConfigP, PaperState};
use crate::strategy::{create_strategy, StrategyKind};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub config: AppConfig,
    pub data_cache_dir: PathBuf,
    pub paper_state: Arc<RwLock<PaperState>>,
    pub feed: Arc<HttpFeed>,
}

impl AppState {
    pub fn new(config: AppConfig, data_cache_dir: PathBuf) -> Self {
        let strategy = create_strategy(StrategyKind::SmaCross { fast: 5, slow: 20 });
        let paper_cfg = PaperConfigP {
            symbol: config.trading.symbol.clone(),
            initial_capital: config.trading.initial_capital,
            commission_rate: config.trading.commission_rate,
            slippage_rate: config.trading.slippage_rate,
            poll_interval_seconds: config.paper.poll_interval_seconds,
            risk: crate::risk::RiskConfig {
                stop_loss_pct: config.risk.stop_loss_pct,
                take_profit_pct: config.risk.take_profit_pct,
                max_position_pct: config.risk.max_position_pct,
            },
        };
        let paper_state = PaperState::new(paper_cfg, strategy);
        let feed = Arc::new(HttpFeed::new(data_cache_dir.clone()));
        Self {
            config,
            data_cache_dir,
            paper_state: Arc::new(RwLock::new(paper_state)),
            feed,
        }
    }
}
