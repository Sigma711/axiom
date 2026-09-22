//! Validation at HTTP boundaries, before allocating windows or doing network I/O.
use crate::types::Bar;
use axum::http::StatusCode;
use std::collections::HashMap;

pub type ApiError = (StatusCode, String);
pub fn bad(message: impl Into<String>) -> ApiError {
    (StatusCode::BAD_REQUEST, message.into())
}

pub fn market(symbol: &str, source: &str, limit: usize, maximum: usize) -> Result<(), ApiError> {
    if symbol.is_empty() || symbol.len() > 30 {
        return Err(bad("symbol must contain 1–30 characters"));
    }
    // Legacy and deterministic fixture sources stay protocol-compatible for
    // older saved lessons and automated checks; they are never offered by the UI.
    if !crate::data::is_public_market_source(source) && !matches!(source, "real" | "synthetic") {
        return Err(bad("source must be binance, a_share or us_stock"));
    }
    let valid_symbol = match source {
        "synthetic" => symbol
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()),
        "real" | "binance" => symbol
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()),
        "a_share" => symbol.len() == 6 && symbol.bytes().all(|b| b.is_ascii_digit()),
        "us_stock" => symbol
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'.' || b == b'-'),
        _ => false,
    };
    if !valid_symbol {
        return Err(bad("symbol is invalid for the selected market"));
    }
    if limit == 0 || limit > maximum {
        return Err(bad(format!("limit must be between 1 and {maximum}")));
    }
    Ok(())
}

pub fn bars(bars: &[Bar]) -> Result<(), ApiError> {
    if bars.is_empty() || bars.len() > 5000 {
        return Err(bad("bars must contain 1–5000 candles"));
    }
    for (i, bar) in bars.iter().enumerate() {
        if ![bar.open, bar.high, bar.low, bar.close, bar.volume]
            .iter()
            .all(|x| x.is_finite())
            || bar.low <= 0.0
            || bar.volume < 0.0
            || bar.low > bar.open.min(bar.close)
            || bar.high < bar.open.max(bar.close)
            || (i > 0 && bars[i - 1].timestamp >= bar.timestamp)
        {
            return Err(bad(format!(
                "invalid OHLCV or non-increasing timestamp at bar {i}"
            )));
        }
    }
    Ok(())
}

pub fn bounded(name: &str, value: f64, min: f64, max: f64) -> Result<(), ApiError> {
    if !value.is_finite() || value < min || value > max {
        return Err(bad(format!("{name} must be between {min} and {max}")));
    }
    Ok(())
}

pub fn strategy(name: &str, p: &HashMap<String, f64>) -> anyhow::Result<()> {
    let keys: &[&str] = match name {
        "buy_and_hold" => &[],
        "random" => &["buy_prob", "sell_prob"],
        "sma_cross" => &["fast", "slow"],
        "rsi" => &["period", "overbought", "oversold"],
        "macd" | "ppo" => &["fast", "slow", "signal"],
        "bollinger" => &["period", "num_std"],
        "supertrend" => &["period", "multiplier"],
        "donchian_breakout" => &["entry_period", "exit_period"],
        "vwap_reversion" => &["period", "threshold_pct"],
        "kdj" => &["n", "m1", "m2"],
        "ichimoku" => &["tenkan", "kijun", "senkou_b", "displacement"],
        "vortex" | "elder_ray" => &["period"],
        _ => anyhow::bail!("未知策略: {name}"),
    };
    for (key, value) in p {
        anyhow::ensure!(keys.contains(&key.as_str()), "unknown parameter: {key}");
        anyhow::ensure!(value.is_finite(), "{key} must be finite");
        match key.as_str() {
            "buy_prob" | "sell_prob" => {
                anyhow::ensure!((0.0..=1.0).contains(value), "invalid probability")
            }
            "overbought" | "oversold" => {
                anyhow::ensure!((0.0..=100.0).contains(value), "invalid RSI threshold")
            }
            "num_std" | "multiplier" | "threshold_pct" => anyhow::ensure!(
                *value > 0.0 && *value <= 100.0,
                "invalid multiplier/threshold"
            ),
            "displacement" => anyhow::ensure!(
                *value >= 0.0 && *value <= 500.0 && value.fract() == 0.0,
                "invalid displacement"
            ),
            _ => anyhow::ensure!(
                *value >= 1.0 && *value <= 500.0 && value.fract() == 0.0,
                "{key} must be an integer between 1 and 500"
            ),
        }
    }
    let get = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    if matches!(name, "sma_cross" | "macd" | "ppo") {
        let (fast, slow) = if name == "sma_cross" {
            (5.0, 20.0)
        } else {
            (12.0, 26.0)
        };
        anyhow::ensure!(
            get("fast", fast) < get("slow", slow),
            "fast must be less than slow"
        );
    }
    if name == "rsi" {
        anyhow::ensure!(
            get("oversold", 30.0) < get("overbought", 70.0),
            "oversold must be less than overbought"
        );
    }
    if name == "random" {
        anyhow::ensure!(
            get("buy_prob", 0.05) + get("sell_prob", 0.05) <= 1.0,
            "probabilities must sum to at most 1"
        );
    }
    Ok(())
}

pub fn indicator(token: &str) -> Result<(&str, usize), ApiError> {
    let (name, supplied_period) = match token.rsplit_once('_') {
        Some((name, suffix))
            if suffix.bytes().all(|x| x.is_ascii_digit()) && !suffix.is_empty() =>
        {
            (
                name,
                Some(
                    suffix
                        .parse::<usize>()
                        .map_err(|_| bad("invalid indicator period"))?,
                ),
            )
        }
        _ => (token, None),
    };
    let allowed = [
        "sma",
        "ema",
        "rsi",
        "vwma",
        "bbands",
        "macd",
        "vwap",
        "atr",
        "atr_percent",
        "obv",
        "zscore",
        "z_score",
        "ichimoku",
        "kdj",
        "stoch",
        "stochastic",
        "williams_r",
        "cci",
        "adx",
        "dmi_adx",
        "bbi",
        "alligator",
        "ppo",
        "vortex",
    ];
    if !allowed.contains(&name) {
        return Err(bad(format!("unknown indicator: {token}")));
    }
    if supplied_period.is_some()
        && matches!(
            name,
            "macd" | "vwap" | "obv" | "ichimoku" | "kdj" | "bbi" | "alligator" | "ppo"
        )
    {
        return Err(bad(format!(
            "{name} does not accept a single period suffix"
        )));
    }
    let period =
        supplied_period.unwrap_or(if matches!(name, "bbands" | "cci" | "zscore" | "z_score") {
            20
        } else {
            14
        });
    if !(1..=500).contains(&period) {
        return Err(bad("indicator period must be between 1 and 500"));
    }
    Ok((name, period))
}

#[cfg(test)]
mod market_source_tests {
    use super::*;

    #[test]
    fn accepts_only_real_supported_market_and_symbol_pairs() {
        assert!(market("BTCUSDT", "binance", 100, 5000).is_ok());
        assert!(market("600519", "a_share", 100, 5000).is_ok());
        assert!(market("AAPL", "us_stock", 100, 5000).is_ok());
        assert!(market("BTCUSDT", "synthetic", 100, 5000).is_ok());
    }
}
