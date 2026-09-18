use axiom::data::SyntheticFeed;
use axiom::data::DataFeed;

fn main() {
    let feed = SyntheticFeed::new(42);
    let bars = feed.fetch_historical("BTCUSDT",
        chrono::Utc::now() - chrono::Duration::days(180), 500).unwrap();
    println!("bars 数量: {}", bars.len());
    println!("前 10 个 close:");
    for b in bars.iter().take(10) {
        println!("  close = {}", b.close);
    }
    println!("...");
    println!("后 10 个 close:");
    for b in bars.iter().rev().take(10) {
        println!("  close = {}", b.close);
    }
}