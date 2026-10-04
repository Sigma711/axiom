use axiom::indicators::breadth::{
    breadth_thrust, bullish_percent_index, market_advance_decline_line, mcclellan_oscillator,
    point_and_figure_buy_signal, tick_index,
};

#[test]
fn market_ad_line_cumulates_cross_sectional_net_issues_from_an_explicit_base() {
    assert_eq!(
        market_advance_decline_line(&[3.0, 1.0, 4.0], &[1.0, 2.0, 2.0], 10.0),
        vec![12.0, 11.0, 13.0]
    );
    assert!(market_advance_decline_line(&[1.0], &[], 0.0).is_empty());
}

#[test]
fn point_and_figure_signal_persists_after_a_double_top_or_double_bottom() {
    let bullish = [100.0, 110.0, 100.0, 112.0, 108.0];
    assert_eq!(point_and_figure_buy_signal(&bullish, 0.01, 3), Some(true));

    let bearish = [100.0, 90.0, 100.0, 88.0, 92.0];
    assert_eq!(point_and_figure_buy_signal(&bearish, 0.01, 3), Some(false));
    assert_eq!(point_and_figure_buy_signal(&[100.0, 101.0], 0.01, 3), None);
    assert_eq!(point_and_figure_buy_signal(&bullish, 0.0, 3), None);
    assert_eq!(
        point_and_figure_buy_signal(&[100.0, 110.0, 100.0, 109.0, 113.0], 0.01, 3),
        Some(true)
    );
    assert_eq!(
        point_and_figure_buy_signal(&[100.0, 90.0, 100.0, 91.0, 87.0], 0.01, 3),
        Some(false)
    );
    assert!(!breadth_thrust(&[0.5; 5]));
}

#[test]
fn mcclellan_uses_daily_net_advances_and_both_ema_warmups() {
    let mut net_advances = vec![0.0; 39];
    net_advances[38] = 100.0;

    let values = mcclellan_oscillator(&net_advances);
    assert_eq!(values.len(), net_advances.len());
    assert_eq!(values[37], None);
    // EMA19 = 10 after a final +100; EMA39 is seeded at 100 / 39.
    assert!((values[38].unwrap() - (10.0 - 100.0 / 39.0)).abs() < 1e-12);
}

#[test]
fn zweig_thrust_requires_the_low_and_high_to_be_within_ten_observations() {
    let mut confirmed = vec![0.35; 10];
    confirmed.extend([0.90; 4]);
    assert!(breadth_thrust(&confirmed));

    let mut expired_low = vec![0.35; 10];
    expired_low.extend([0.90; 10]);
    assert!(!breadth_thrust(&expired_low));
}

#[test]
fn tick_and_bullish_percent_keep_their_cross_sectional_definitions() {
    assert_eq!(tick_index(&[100, 50, 80], &[30, 60, 20]), vec![70, -10, 60]);
    assert_eq!(bullish_percent_index(60.0, 100.0), 60.0);
}
