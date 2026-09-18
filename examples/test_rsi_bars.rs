use axiom::data::SyntheticFeed;
use axiom::data::DataFeed;
use axiom::strategy::{RsiStrategy, Strategy};
use axiom::types::Side;

fn main() {
    let feed = SyntheticFeed::new(42);
    let bars = feed.fetch_historical("BTCUSDT",
        chrono::Utc::now() - chrono::Duration::days(180), 500).unwrap();
    println!("bars 数量: {}", bars.len());

    let mut strat = RsiStrategy::new(14, 70.0, 30.0);
    let mut buy_count = 0;
    let mut sell_count = 0;
    let mut hold_count = 0;
    let mut first_buy_reason = String::new();
    let mut first_sell_reason = String::new();

    for bar in &bars {
        let sig = strat.on_bar(bar);
        match sig.side {
            Side::Buy => {
                buy_count += 1;
                if first_buy_reason.is_empty() { first_buy_reason = sig.reason.clone(); }
            }
            Side::Sell => {
                sell_count += 1;
                if first_sell_reason.is_empty() { first_sell_reason = sig.reason.clone(); }
            }
            Side::Hold => hold_count += 1,
        }
    }
    println!("BUY: {}, SELL: {}, HOLD: {}", buy_count, sell_count, hold_count);
    if !first_buy_reason.is_empty() {
        println!("BUY 例子: {}", first_buy_reason);
    }
    if !first_sell_reason.is_empty() {
        println!("SELL 例子: {}", first_sell_reason);
    }
}