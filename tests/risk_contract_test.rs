use axiom::broker::new_order;
use axiom::types::{OrderType, Position};
use axiom::{
    Broker, BrokerConfig, Portfolio, PortfolioConfig, RiskConfig, RiskManager, Side,
    SimulatedBroker,
};
#[test]
fn short_risk_limits_follow_the_position_direction() {
    let mut broker = SimulatedBroker::new(
        BrokerConfig {
            commission_rate: 0.0,
            slippage_rate: 0.0,
            allow_short: true,
        },
        10000.0,
    );
    broker.set_market_price("BTCUSDT", 100.0);
    let fill = broker.place_order(new_order(
        "BTCUSDT",
        Side::Sell,
        10.0,
        chrono::Utc::now(),
        OrderType::Market,
        None,
    ));
    assert_eq!(fill.size, 10.0);
    let mut portfolio = Portfolio::new(
        Box::new(broker),
        PortfolioConfig {
            symbol: "BTCUSDT".into(),
            ..Default::default()
        },
    );
    let risk = RiskManager::new(
        RiskConfig {
            stop_loss_pct: 0.05,
            take_profit_pct: 0.1,
            max_position_pct: 0.95,
        },
        "BTCUSDT".into(),
    );
    portfolio.broker.set_market_price("BTCUSDT", 106.0);
    assert!(risk
        .force_close_reason(&portfolio, portfolio.broker.as_ref())
        .expect("short loses when price rises")
        .contains("止损"));
    portfolio.broker.set_market_price("BTCUSDT", 88.0);
    assert!(risk
        .force_close_reason(&portfolio, portfolio.broker.as_ref())
        .expect("short gains when price falls")
        .contains("止盈"));
}

#[test]
fn risk_manager_explains_missing_price_zero_equity_and_position_limits() {
    let no_cash = SimulatedBroker::new(BrokerConfig::default(), 0.0);
    let portfolio = Portfolio::new(
        Box::new(no_cash.clone()),
        PortfolioConfig {
            symbol: "X".into(),
            ..Default::default()
        },
    );
    let risk = RiskManager::new(RiskConfig::default(), "X".into());
    let buy = new_order(
        "X",
        Side::Buy,
        1.0,
        chrono::Utc::now(),
        OrderType::Market,
        None,
    );
    assert!(!risk.allow_order(&buy, &portfolio, &no_cash).0);

    let mut broker = SimulatedBroker::new(BrokerConfig::default(), 1_000.0);
    let portfolio = Portfolio::new(
        Box::new(broker.clone()),
        PortfolioConfig {
            symbol: "X".into(),
            ..Default::default()
        },
    );
    assert_eq!(
        risk.allow_order(&buy, &portfolio, &broker).1,
        "没有当前市场价"
    );
    broker.set_market_price("X", 100.0);
    let portfolio = Portfolio::new(
        Box::new(broker.clone()),
        PortfolioConfig {
            symbol: "X".into(),
            ..Default::default()
        },
    );
    let too_large = new_order(
        "X",
        Side::Buy,
        20.0,
        chrono::Utc::now(),
        OrderType::Market,
        None,
    );
    assert!(!risk.allow_order(&too_large, &portfolio, &broker).0);
    let sell = new_order(
        "X",
        Side::Sell,
        20.0,
        chrono::Utc::now(),
        OrderType::Market,
        None,
    );
    assert!(risk.allow_order(&sell, &portfolio, &broker).0);

    broker.positions.insert(
        "X".into(),
        Position {
            symbol: "X".into(),
            size: 1.0,
            ..Default::default()
        },
    );
    let portfolio = Portfolio::new(
        Box::new(broker),
        PortfolioConfig {
            symbol: "X".into(),
            ..Default::default()
        },
    );
    assert!(risk
        .force_close_reason(&portfolio, portfolio.broker.as_ref())
        .is_none());
}
