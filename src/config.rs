//! 配置加载 —— 从 YAML 读取默认参数,允许运行时覆盖。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub exchange: ExchangeConfig,
    #[serde(default)]
    pub trading: TradingConfig,
    #[serde(default)]
    pub backtest: BacktestConfig,
    #[serde(default)]
    pub paper: PaperConfig,
    #[serde(default)]
    pub risk: RiskConfigY,
    #[serde(default)]
    pub strategies: HashMap<String, StrategyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    #[serde(default = "default_exchange_name")]
    pub name: String,
    #[serde(default)]
    pub sandbox: bool,
}
fn default_exchange_name() -> String {
    "binance".to_string()
}
impl Default for ExchangeConfig {
    fn default() -> Self {
        Self { name: "binance".to_string(), sandbox: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    #[serde(default = "default_symbol")]
    pub symbol: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default = "default_initial_capital")]
    pub initial_capital: f64,
    #[serde(default = "default_commission")]
    pub commission_rate: f64,
    #[serde(default = "default_slippage")]
    pub slippage_rate: f64,
}
fn default_symbol() -> String { "BTCUSDT".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
fn default_initial_capital() -> f64 { 10_000.0 }
fn default_commission() -> f64 { 0.001 }
fn default_slippage() -> f64 { 0.0005 }
impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            symbol: default_symbol(),
            timeframe: default_timeframe(),
            initial_capital: default_initial_capital(),
            commission_rate: default_commission(),
            slippage_rate: default_slippage(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    #[serde(default = "default_lookback")]
    pub lookback_days: u32,
    #[serde(default = "default_speed")]
    pub speed: String,
}
fn default_lookback() -> u32 { 180 }
fn default_speed() -> String { "instant".to_string() }
impl Default for BacktestConfig {
    fn default() -> Self {
        Self { lookback_days: default_lookback(), speed: default_speed() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperConfig {
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
    #[serde(default = "default_speed_multiplier")]
    pub speed_multiplier: u64,
}
fn default_poll_interval() -> u64 { 5 }
fn default_speed_multiplier() -> u64 { 60 }
impl Default for PaperConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: default_poll_interval(),
            speed_multiplier: default_speed_multiplier(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfigY {
    #[serde(default = "default_max_pos")]
    pub max_position_pct: f64,
    #[serde(default)]
    pub stop_loss_pct: f64,
    #[serde(default)]
    pub take_profit_pct: f64,
}
fn default_max_pos() -> f64 { 0.95 }
impl Default for RiskConfigY {
    fn default() -> Self {
        Self {
            max_position_pct: default_max_pos(),
            stop_loss_pct: 0.0,
            take_profit_pct: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub params: HashMap<String, f64>,
}

pub fn load_config(path: &Path) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("读取配置文件失败: {:?}", path))?;
    let cfg: AppConfig = serde_yaml::from_str(&content)
        .with_context(|| "解析 YAML 失败")?;
    Ok(cfg)
}

pub fn default_config() -> AppConfig {
    AppConfig::default()
}