use cipher::data::{DataFeed, HttpFeed};
use cipher::strategy::{RsiStrategy, Strategy};
use cipher::types::Side;

fn main() {
    let feed = HttpFeed::new("data".into());
    let r = tokio_test::block_on(async {
        feed.fetch_historical_async("BTCUSDT", chrono::Utc::now() - chrono::Duration::days(30), 500).await
    });
    let bars = match r {
        Ok(b) => b,
        Err(e) => { println!("fetch err: {}", e); return; }
    };
    println!("拿到 {} 根 K 线", bars.len());

    let mut strat = RsiStrategy::new(14, 70.0, 30.0);
    let mut buy_count = 0;
    let mut sell_count = 0;
    let mut prev_rsi: Option<f64> = None;
    for (i, bar) in bars.iter().enumerate() {
        let signal = strat.on_bar(bar);
        if signal.side == Side::Buy { buy_count += 1; }
        if signal.side == Side::Sell { sell_count += 1; }
        if i < 25 || (i % 50 == 0) {
            println!("i={}: side={:?}, reason={}", i, signal.side, signal.reason);
        }
        // 计算当前 RSI
        if i + 1 >= 14 {
            let closes: Vec<f64> = bars[..=i].iter().map(|b| b.close).collect();
            if closes.len() >= 15 {
                let mut ag = 0.0; let mut al = 0.0;
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
    println!("\nBUY: {}, SELL: {}", buy_count, sell_count);
}