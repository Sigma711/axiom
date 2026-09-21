//! Independent worked examples for the remaining public formula surface.
use axiom::{
    indicators::{breadth, extra, options},
    types::Bar,
};
use chrono::{TimeZone, Utc};

fn bar(day: u32, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Bar {
    Bar {
        timestamp: Utc.with_ymd_and_hms(2024, 1, day, 0, 0, 0).unwrap(),
        open,
        high,
        low,
        close,
        volume,
    }
}

#[test]
fn breadth_worked_examples_cover_direction_ratios_and_zero_denominators() {
    assert_eq!(breadth::advance_decline_one_bar(11.0, 10.0), 1);
    assert_eq!(breadth::advance_decline_one_bar(9.0, 10.0), -1);
    assert_eq!(breadth::advance_decline_one_bar(10.0, 10.0), 0);
    assert_eq!(breadth::adl_one_bar(100.0, 12.0, 8.0, 12.0, 50.0), 150.0);
    assert_eq!(breadth::adl_one_bar(100.0, 10.0, 10.0, 10.0, 50.0), 100.0);
    assert_eq!(breadth::ad_ratio(30.0, 10.0), 3.0);
    assert_eq!(breadth::new_high_low_ratio(8.0, 2.0), 4.0);
    assert_eq!(breadth::trin(30.0, 10.0, 300.0, 100.0), 1.0);
    assert_eq!(breadth::up_down_volume_ratio(90.0, 30.0), 3.0);
    assert_eq!(breadth::bullish_percent_index(25.0, 100.0), 25.0);
    assert_eq!(
        breadth::advances_declines(&[1.0, 2.0, 2.0, 1.0, 3.0]),
        (2, 1)
    );
    assert_eq!(breadth::tick_index(&[10, 2], &[3, 5]), vec![7, -3]);

    assert_eq!(breadth::ad_ratio(1.0, 0.0), 0.0);
    assert_eq!(breadth::new_high_low_ratio(1.0, 0.0), 0.0);
    assert_eq!(breadth::trin(1.0, 0.0, 1.0, 1.0), 0.0);
    assert_eq!(breadth::trin(1.0, 1.0, 1.0, 0.0), 0.0);
    assert_eq!(breadth::up_down_volume_ratio(1.0, 0.0), 0.0);
    assert_eq!(breadth::bullish_percent_index(1.0, 0.0), 0.0);
}

#[test]
fn candle_and_pivot_examples_match_their_written_definitions() {
    let doji = bar(1, 10.0, 12.0, 8.0, 10.1, 1.0);
    let hammer = bar(2, 10.0, 11.2, 6.0, 11.0, 1.0);
    let star = bar(3, 10.0, 14.0, 9.0, 9.0, 1.0);
    let flat = bar(4, 10.0, 10.0, 10.0, 10.0, 1.0);
    assert_eq!(extra::detect_pattern(&doji), extra::CandlePattern::Doji);
    assert_eq!(extra::detect_pattern(&hammer), extra::CandlePattern::Hammer);
    assert_eq!(
        extra::detect_pattern(&star),
        extra::CandlePattern::ShootingStar
    );
    assert_eq!(extra::detect_pattern(&flat), extra::CandlePattern::None);
    assert_eq!(extra::CandlePattern::Engulfing.name_zh(), "吞没形态");

    let bear = bar(5, 11.0, 12.0, 9.0, 10.0, 1.0);
    let bull_engulf = bar(6, 9.0, 12.0, 8.0, 12.0, 1.0);
    assert_eq!(
        extra::detect_engulfing(&bear, &bull_engulf),
        extra::CandlePattern::Engulfing
    );
    let inside = bar(7, 10.0, 11.0, 9.0, 10.5, 1.0);
    assert_eq!(
        extra::detect_inside_outside(&bull_engulf, &inside),
        extra::CandlePattern::Inside
    );
    let outside = bar(8, 10.0, 13.0, 7.0, 10.5, 1.0);
    assert_eq!(
        extra::detect_inside_outside(&inside, &outside),
        extra::CandlePattern::Outside
    );

    let pivots = extra::pivot_points(110.0, 90.0, 100.0);
    assert_eq!(pivots.pivot, 100.0);
    assert_eq!(pivots.r1, 110.0);
    assert_eq!(pivots.s1, 90.0);
    assert_eq!(pivots.r2, 120.0);
    assert_eq!(pivots.s2, 80.0);
    assert_eq!(pivots.r3, 130.0);
    assert_eq!(pivots.s3, 70.0);
    assert!(extra::bar_summary(&bull_engulf).contains("阳线"));
    assert_eq!(extra::zscore(&[2.0, 2.0], 2), vec![None, None]);
}

#[test]
fn black_scholes_worked_example_respects_parity_greeks_and_expiry_payoff() {
    let (spot, strike, time, rate, volatility) = (100.0, 100.0, 1.0, 0.05, 0.20);
    let call = options::bs_price(spot, strike, time, rate, volatility, true);
    let put = options::bs_price(spot, strike, time, rate, volatility, false);
    assert!((call - 10.4506).abs() < 0.001);
    assert!((put - 5.5735).abs() < 0.001);
    assert!((call - put - (spot - strike * (-rate * time).exp())).abs() < 0.002);
    assert!(options::delta(spot, strike, time, rate, volatility, true) > 0.5);
    assert!(options::delta(spot, strike, time, rate, volatility, false) < 0.0);
    assert!(options::gamma(spot, strike, time, rate, volatility) > 0.0);
    assert!(options::theta(spot, strike, time, rate, volatility, true) < 0.0);
    assert!(options::vega(spot, strike, time, rate, volatility) > 0.0);
    assert!(options::rho(spot, strike, time, rate, volatility, true) > 0.0);
    assert!(options::rho(spot, strike, time, rate, volatility, false) < 0.0);
    assert_eq!(
        options::bs_price(110.0, 100.0, 0.0, rate, volatility, true),
        10.0
    );
    assert_eq!(
        options::bs_price(90.0, 100.0, 0.0, rate, volatility, false),
        10.0
    );
    assert_eq!(
        options::delta(100.0, 100.0, 0.0, rate, volatility, true),
        0.0
    );
    assert_eq!(options::gamma(100.0, 100.0, 0.0, rate, volatility), 0.0);
    assert_eq!(
        options::theta(100.0, 100.0, 0.0, rate, volatility, true),
        0.0
    );
    assert_eq!(options::vega(100.0, 100.0, 0.0, rate, volatility), 0.0);
    assert_eq!(options::rho(100.0, 100.0, 0.0, rate, volatility, true), 0.0);
    assert!((options::iv_rank(0.4, 0.2, 0.6) - 50.0).abs() < 1e-10);
    assert_eq!(options::iv_percentile(10.0, 20.0), 50.0);
    assert_eq!(options::put_call_ratio(60.0, 30.0), 2.0);
    assert_eq!(options::iv_rank(0.4, 0.6, 0.2), 0.0);
    assert_eq!(options::iv_percentile(1.0, 0.0), 0.0);
    assert_eq!(options::put_call_ratio(1.0, 0.0), 0.0);
}

#[test]
fn star_patterns_require_confirmation_and_reject_ambiguous_sequences() {
    let morning = [
        bar(10, 12.0, 12.5, 7.0, 8.0, 1.0),
        bar(11, 8.5, 8.7, 8.3, 8.4, 1.0),
        bar(12, 8.0, 12.5, 7.8, 12.2, 1.0),
    ];
    assert_eq!(extra::detect_star(&morning[0], &morning[1], &morning[2]), 1);
    let evening = [
        bar(13, 8.0, 13.0, 7.8, 12.0, 1.0),
        bar(14, 11.6, 11.8, 11.4, 11.5, 1.0),
        bar(15, 12.0, 12.2, 7.5, 7.8, 1.0),
    ];
    assert_eq!(
        extra::detect_star(&evening[0], &evening[1], &evening[2]),
        -1
    );
    let no_confirmation = bar(16, 8.0, 10.0, 7.8, 9.0, 1.0);
    assert_eq!(
        extra::detect_star(&morning[0], &morning[1], &no_confirmation),
        0
    );
    let wide_middle = bar(17, 8.0, 10.0, 7.0, 9.5, 1.0);
    assert_eq!(
        extra::detect_star(&morning[0], &wide_middle, &morning[2]),
        0
    );
}

#[test]
fn candle_boundaries_cover_bearish_engulfing_marubozu_and_neutral_summary() {
    let marubozu = bar(20, 10.0, 12.0, 10.0, 12.0, 1.0);
    assert_eq!(
        extra::detect_pattern(&marubozu),
        extra::CandlePattern::Marubozu
    );
    let bull = bar(21, 9.0, 11.0, 8.0, 10.0, 1.0);
    let bear_engulf = bar(22, 11.0, 12.0, 7.0, 8.0, 1.0);
    assert_eq!(
        extra::detect_engulfing(&bull, &bear_engulf),
        extra::CandlePattern::Engulfing
    );
    assert_eq!(
        extra::detect_inside_outside(&bull, &bear_engulf),
        extra::CandlePattern::Outside
    );
    let neutral = bar(23, 0.0, 1.0, -1.0, 0.0, 12.0);
    assert!(extra::bar_summary(&neutral).contains("─ 平"));
    assert_eq!(extra::zscore(&[1.0], 0), vec![None]);
}

#[test]
fn candle_pattern_labels_and_short_zscore_windows_are_explicit() {
    assert_eq!(extra::CandlePattern::Hammer.name_zh(), "锤子线");
    assert_eq!(extra::CandlePattern::ShootingStar.name_zh(), "流星线");
    assert_eq!(extra::CandlePattern::Marubozu.name_zh(), "光头光脚");
    assert_eq!(extra::CandlePattern::Inside.name_zh(), "内包线");
    assert_eq!(extra::CandlePattern::Outside.name_zh(), "外包线");
    assert_eq!(extra::CandlePattern::None.name_zh(), "无特殊形态");
    assert_eq!(extra::zscore(&[1.0, 2.0], 3), vec![None, None]);
}
