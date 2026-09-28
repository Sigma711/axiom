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
    pub filing_source: crate::filing_case::FilingSourceConfig,
    pub industry_sources: crate::industry_case::IndustrySourceRegistry,
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
        let execution_profile =
            crate::execution::ExecutionProfile::for_market("binance", &paper_cfg.symbol);
        let paper_state =
            PaperState::new_with_execution_profile(paper_cfg, strategy, execution_profile);
        let feed = Arc::new(HttpFeed::new(data_cache_dir.clone()));
        Self {
            config,
            data_cache_dir,
            paper_state: Arc::new(RwLock::new(paper_state)),
            feed,
            filing_source: crate::filing_case::FilingSourceConfig::default(),
            industry_sources: crate::industry_case::IndustrySourceRegistry::default(),
        }
    }

    /// Replaces only the document transport identity; useful for a hermetic HTTP seam test.
    pub fn with_filing_source(mut self, source: crate::filing_case::FilingSourceConfig) -> Self {
        self.filing_source = source;
        self
    }

    pub fn with_industry_sources(
        mut self,
        sources: crate::industry_case::IndustrySourceRegistry,
    ) -> Self {
        self.industry_sources = sources;
        self
    }
}
