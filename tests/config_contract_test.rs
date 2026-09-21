//! Configuration-file contract tests at the filesystem boundary.
use axiom::config::{default_config, load_config};
use std::{fs, path::PathBuf};

fn temporary_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("axiom-{label}-{}.yaml", std::process::id()))
}

#[test]
fn default_config_and_partial_yaml_apply_the_documented_defaults() {
    let defaults = default_config();
    assert_eq!(defaults.exchange.name, "binance");
    assert_eq!(defaults.trading.symbol, "BTCUSDT");
    assert_eq!(defaults.trading.timeframe, "1h");
    assert_eq!(defaults.backtest.lookback_days, 180);
    assert_eq!(defaults.paper.poll_interval_seconds, 5);
    assert_eq!(defaults.risk.max_position_pct, 0.95);

    let path = temporary_path("partial-config");
    fs::write(&path, "exchange: {}\ntrading:\n  symbol: ETHUSDT\n").unwrap();
    let parsed = load_config(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(parsed.trading.symbol, "ETHUSDT");
    assert_eq!(parsed.exchange.name, "binance");
    assert_eq!(parsed.trading.initial_capital, 10_000.0);
}

#[test]
fn configuration_load_reports_missing_and_invalid_yaml() {
    let missing = temporary_path("missing-config");
    assert!(load_config(&missing)
        .unwrap_err()
        .to_string()
        .contains("读取配置文件失败"));

    let invalid = temporary_path("invalid-config");
    fs::write(&invalid, "trading: [not a mapping").unwrap();
    assert!(load_config(&invalid)
        .unwrap_err()
        .to_string()
        .contains("解析 YAML 失败"));
    fs::remove_file(&invalid).unwrap();
}
