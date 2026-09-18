// 测试 RSI strategy 的 on_bar 逻辑
use chrono::{Duration, TimeZone, Utc};
use cipher::data::{DataFeed, SyntheticFeed};
use cipher::strategy::{RsiStrategy, Strategy};
use cipher::types::{Bar, Side};

fn main() {
    // 拿 500 根真实 BTC K 线
    let feed = SyntheticFeed::default();
    let since = Utc::now() - Duration::days(180);
    let bars = feed.fetch_historical("BTCUSDT", since, 500).unwrap();

    let mut strat = RsiStrategy::new(14, 70.0, 30.0);
    let mut buy_count = 0;
    let mut sell_count = 0;
    let mut prev_rsi: Option<f64> = None;
    for (i, bar) in bars.iter().enumerate() {
        let signal = strat.on_bar(bar);
        if i >= 14 { // 已经过预热
            if let Some(p) = prev_rsi {
                let p32 = p <= 30.0;
                let c32 = signal.side == Side::Buy;
                if p32 || c32 {
                    // 输出有趣的时刻
                    if i < 25 || p32 {
                        println!("i={}: prev={:.2}, cur_side={:?}, reason={}", i, p, signal.side, signal.reason);
                    }
                }
            }
        }
        if signal.side == Side::Buy { buy_count += 1; }
        if signal.side == Side::Sell { sell_count += 1; }
        // 计算当前 RSI 用于下次
        if i + 1 >= 14 {
            let closes: Vec<f64> = bars[..=i].iter().map(|b| b.close).collect();
            if closes.len() >= 15 {
                let mut ag = 0.0;
                let mut al = 0.0;
                for j in 1..=14 {
                    let d = closes[j + closes.len() - 14] - closes[j + closes.len() - 15];
                    if d > 0.0 { ag += d; } else { al -= d; }
                }
                ag /= 14.0; al /= 14.0;
                if al > 0.0 {
                    prev_rsi = Some(100.0 - 100.0 / (1.0 + ag / al));
                } else {
                    prev_rsi = Some(100.0);
                }
            }
        }
    }
    println!("BUY: {}, SELL: {}", buy_count, sell_count);
}