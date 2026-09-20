use axiom::{
    indicators::momentum::{ar, br, psy},
    types::Bar,
};
use chrono::{TimeZone, Utc};

fn bar(index: i64, open: f64, high: f64, low: f64, close: f64) -> Bar {
    Bar {
        timestamp: Utc.timestamp_opt(index, 0).unwrap(),
        open,
        high,
        low,
        close,
        volume: 1.0,
    }
}

#[test]
fn psy_counts_n_price_changes_over_n_days() {
    let values = psy(&[1.0, 2.0, 3.0, 4.0], 3);
    assert_eq!(values[3], Some(100.0));
}

#[test]
fn ar_is_the_ratio_of_full_period_high_open_and_open_low_sums() {
    let values = ar(&[bar(0, 10.0, 12.0, 8.0, 11.0)], 1);
    assert_eq!(values[0], Some(100.0));
}

#[test]
fn br_includes_the_current_bar_in_its_n_comparisons() {
    let bars = [
        bar(0, 100.0, 100.0, 100.0, 100.0),
        bar(1, 100.0, 110.0, 99.0, 100.0),
        bar(2, 90.0, 100.0, 80.0, 90.0),
    ];
    let values = br(&bars, 2);
    assert!((values[2].unwrap() - 10.0 / 21.0 * 100.0).abs() < 1e-12);
}
