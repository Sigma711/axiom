use axiom::broker::{new_order, Broker, BrokerConfig, SimulatedBroker};
use axiom::types::{OrderType, Side};
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
