use axiom::broker::{new_order, Broker, BrokerConfig, SimulatedBroker};
use axiom::types::{Fill, OrderType, Side};
use axiom::{Portfolio, PortfolioConfig};
use chrono::Utc;

fn order(b: &mut SimulatedBroker, side: Side, size: f64) -> axiom::Fill {
    b.place_order(new_order(
        "X",
        side,
        size,
        Utc::now(),
        OrderType::Market,
        None,
    ))
}
#[test]
fn malformed_orders_cannot_create_cash_or_poison_positions() {
    let mut b = SimulatedBroker::new(BrokerConfig::default(), 1000.0);
    b.set_market_price("X", 100.0);
    for size in [-1.0, 0.0, f64::NAN, f64::INFINITY] {
        assert_eq!(order(&mut b, Side::Buy, size).size, 0.0);
        assert_eq!(order(&mut b, Side::Sell, size).size, 0.0);
    }
    assert_eq!(b.get_cash(), 1000.0);
    assert!(b.get_position("X").is_flat());
}
#[test]
fn partial_short_cover_and_direction_reversal_conserve_equity() {
    let mut b = SimulatedBroker::new(
        BrokerConfig {
            commission_rate: 0.01,
            slippage_rate: 0.0,
            allow_short: true,
        },
        2000.0,
    );
    b.set_market_price("X", 100.0);
    order(&mut b, Side::Sell, 10.0);
    assert_eq!(b.get_cash(), 2990.0);
    assert_eq!(b.get_position("X").avg_entry_price, 99.0);
    b.set_market_price("X", 80.0);
    order(&mut b, Side::Buy, 4.0);
    assert!((b.get_position("X").realized_pnl - 72.8).abs() < 1e-10);
    assert!((b.get_cash() - 2666.8).abs() < 1e-10);
    assert_eq!(b.get_position("X").size, -6.0);
    order(&mut b, Side::Buy, 8.0);
    let p = b.get_position("X");
    assert_eq!(p.size, 2.0);
    assert!((p.avg_entry_price - 80.8).abs() < 1e-10);
    assert!((p.realized_pnl - 182.0).abs() < 1e-10);
    assert!((b.get_cash() + p.market_value(80.0) - 2180.4).abs() < 1e-10);
}
#[test]
fn one_hundred_percent_allocation_reserves_fees_and_slippage() {
    let mut b = SimulatedBroker::new(
        BrokerConfig {
            commission_rate: 0.01,
            slippage_rate: 0.01,
            allow_short: false,
        },
        1000.0,
    );
    b.set_market_price("X", 100.0);
    let mut p = Portfolio::new(
        Box::new(b),
        PortfolioConfig {
            max_position_pct: 1.0,
            min_trade_size: 0.0001,
            symbol: "X".into(),
        },
    );
    let order = p.on_signal(Side::Buy, 100.0, Utc::now(), 1.0).unwrap();
    let fill = p.broker.place_order(order);
    assert!(fill.size > 9.8 && fill.size < 9.81);
    assert!(p.cash().abs() < 1e-10);
}
#[test]
fn marketable_limit_receives_available_price_improvement() {
    let mut b = SimulatedBroker::new(
        BrokerConfig {
            commission_rate: 0.0,
            slippage_rate: 0.0,
            allow_short: false,
        },
        1000.0,
    );
    b.set_market_price("X", 95.0);
    let fill = b.place_order(new_order(
        "X",
        Side::Buy,
        1.0,
        Utc::now(),
        OrderType::Limit,
        Some(100.0),
    ));
    assert_eq!(fill.price, 95.0);
    assert_eq!(b.get_cash(), 905.0);
}

#[test]
fn portfolio_signals_and_fills_cover_flat_small_and_unmatched_paths() {
    let mut portfolio = Portfolio::new(
        Box::new(SimulatedBroker::new(BrokerConfig::default(), 1000.0)),
        PortfolioConfig {
            min_trade_size: 1.0,
            symbol: "X".into(),
            ..Default::default()
        },
    );
    let ts = Utc::now();
    assert!(portfolio.on_signal(Side::Hold, 100.0, ts, 1.0).is_none());
    assert!(portfolio.on_signal(Side::Sell, 100.0, ts, 1.0).is_none());
    portfolio.broker.set_market_price("X", 100.0);
    assert!(portfolio.on_signal(Side::Buy, 100.0, ts, 0.0001).is_none());

    portfolio.on_fill(&Fill {
        order_id: "zero".into(),
        timestamp: ts,
        symbol: "X".into(),
        side: Side::Buy,
        size: 0.0,
        price: 100.0,
        commission: 0.0,
    });
    assert!(portfolio.open_trade().is_none());
    portfolio.on_fill(&Fill {
        order_id: "sell-first".into(),
        timestamp: ts,
        symbol: "X".into(),
        side: Side::Sell,
        size: 1.0,
        price: 100.0,
        commission: 0.0,
    });
    assert!(portfolio.closed_trades().is_empty());
}

#[test]
fn broker_rejects_unpriced_hold_and_non_marketable_orders_with_audit_log() {
    let ts = Utc::now();
    let mut broker = SimulatedBroker::new(BrokerConfig::default(), 1_000.0);
    let no_price = order(&mut broker, Side::Buy, 1.0);
    assert_eq!(no_price.size, 0.0);

    broker.set_market_price("X", 100.0);
    assert_eq!(
        broker
            .place_order(new_order("X", Side::Hold, 1.0, ts, OrderType::Market, None))
            .size,
        0.0
    );
    assert_eq!(
        broker
            .place_order(new_order("X", Side::Buy, 1.0, ts, OrderType::Limit, None))
            .size,
        0.0
    );
    assert_eq!(
        broker
            .place_order(new_order(
                "X",
                Side::Buy,
                1.0,
                ts,
                OrderType::Limit,
                Some(f64::NAN),
            ))
            .size,
        0.0
    );
    assert_eq!(
        broker
            .place_order(new_order(
                "X",
                Side::Buy,
                1.0,
                ts,
                OrderType::Limit,
                Some(99.0)
            ))
            .size,
        0.0
    );
    assert_eq!(
        broker
            .place_order(new_order(
                "X",
                Side::Sell,
                1.0,
                ts,
                OrderType::Limit,
                Some(101.0)
            ))
            .size,
        0.0
    );
    assert_eq!(
        broker
            .place_order(new_order(
                "X",
                Side::Buy,
                100.0,
                ts,
                OrderType::Market,
                None
            ))
            .size,
        0.0
    );
    assert_eq!(
        broker
            .place_order(new_order("X", Side::Sell, 1.0, ts, OrderType::Market, None))
            .size,
        0.0
    );
    assert!(broker
        .trade_log()
        .iter()
        .any(|(_, message, _)| message.contains("资金不足")));
    assert!(broker
        .trade_log()
        .iter()
        .any(|(_, message, _)| message.contains("持仓不足")));

    broker.set_market_price("X", f64::NAN);
    assert_eq!(
        broker
            .place_order(new_order("X", Side::Buy, 1.0, ts, OrderType::Market, None))
            .size,
        0.0
    );
    assert_eq!(broker.trade_log().len(), 2);
    assert_eq!(axiom::broker::err("expected").to_string(), "expected");
}
