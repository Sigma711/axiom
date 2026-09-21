use approx::assert_relative_eq;
use axiom::{diagrams, types::*};
use chrono::{Duration, TimeZone, Utc};
use serde_json::json;

#[test]
fn domain_types_preserve_accounting_and_trade_direction() {
    assert_eq!(Side::Buy.to_string(), "BUY");
    assert_eq!(Side::Sell.to_string(), "SELL");
    assert_eq!(Side::Hold.to_string(), "HOLD");
    assert_eq!(OrderType::Market.to_string(), "MARKET");
    assert_eq!(OrderType::Limit.to_string(), "LIMIT");
    let ts = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let bar = Bar {
        timestamp: ts,
        open: 10.0,
        high: 16.0,
        low: 7.0,
        close: 13.0,
        volume: 20.0,
    };
    assert_relative_eq!(bar.typical_price(), 12.0);
    let buy = Fill {
        order_id: "o".into(),
        timestamp: ts,
        symbol: "X".into(),
        side: Side::Buy,
        size: 2.0,
        price: 10.0,
        commission: 1.0,
    };
    assert_relative_eq!(buy.value(), 20.0);
    assert_relative_eq!(buy.cash_impact(), 21.0);
    let sell = Fill {
        side: Side::Sell,
        ..buy.clone()
    };
    assert_relative_eq!(sell.cash_impact(), -19.0);
    assert_eq!(
        Fill {
            side: Side::Hold,
            ..buy
        }
        .cash_impact(),
        0.0
    );
    let position = Position {
        symbol: "X".into(),
        size: 2.0,
        avg_entry_price: 10.0,
        realized_pnl: 0.0,
    };
    assert!(!position.is_flat() && position.is_long() && !position.is_short());
    assert_relative_eq!(position.market_value(12.0), 24.0);
    assert_relative_eq!(position.unrealized_pnl(12.0), 4.0);
    assert!(Position::default().is_flat());
    assert!(Position {
        size: -1.0,
        ..Position::default()
    }
    .is_short());
    let trade = Trade {
        symbol: "X".into(),
        side: Side::Buy,
        entry_time: ts,
        exit_time: Some(ts + Duration::seconds(60)),
        entry_price: 10.0,
        exit_price: Some(14.0),
        size: 2.0,
        entry_commission: 1.0,
        exit_commission: 1.0,
    };
    assert!(trade.is_closed());
    assert_relative_eq!(trade.total_commission(), 2.0);
    assert_relative_eq!(trade.pnl(), 6.0);
    assert_relative_eq!(trade.pnl_pct(), 0.3);
    assert_eq!(trade.duration_seconds(), Some(60.0));
    let short = Trade {
        side: Side::Sell,
        ..trade.clone()
    };
    assert_relative_eq!(short.pnl(), -10.0);
    let open = Trade {
        exit_time: None,
        exit_price: None,
        ..trade
    };
    assert!(!open.is_closed());
    assert_eq!(open.pnl(), 0.0);
    assert_eq!(open.pnl_pct(), 0.0);
    assert_eq!(open.duration_seconds(), None);
}

#[test]
fn backtest_equity_endpoints_and_learning_diagrams_have_stable_meaningful_labels() {
    let ts = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let point = EquityPoint {
        timestamp: ts,
        cash: 80.0,
        position_value: 20.0,
        equity: 110.0,
    };
    let result = BacktestResult {
        config: json!({}),
        equity_curve: vec![point.clone()],
        trades: vec![],
        fills: vec![],
        signals: vec![],
        metrics: json!({}),
    };
    assert_relative_eq!(result.initial_equity(), 100.0);
    assert_relative_eq!(result.final_equity(), 110.0);
    let empty = BacktestResult {
        equity_curve: vec![],
        ..result
    };
    assert_eq!(empty.initial_equity(), 0.0);
    assert_eq!(empty.final_equity(), 0.0);
    for (diagram, label) in [
        (diagrams::rsi_diagram(), "RSI"),
        (diagrams::macd_diagram(), "MACD"),
        (diagrams::bollinger_diagram(), "Bollinger"),
        (diagrams::ichimoku_diagram(), "一目均衡"),
        (diagrams::sharpe_diagram(), "Sharpe"),
        (diagrams::drawdown_diagram(), "最大回撤"),
        (diagrams::kdj_diagram(), "KDJ"),
        (diagrams::obv_diagram(), "OBV"),
        (diagrams::hammer_diagram(), "锤子线"),
        (diagrams::doji_diagram(), "十字星"),
        (diagrams::engulfing_diagram(), "吞没"),
        (diagrams::morning_star_diagram(), "早晨之星"),
    ] {
        assert!(diagram.contains(label), "{label}");
    }
    assert!(diagrams::generic_diagram("ATR").contains("ATR"));
}
