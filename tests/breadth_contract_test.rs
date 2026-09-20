use axiom::indicators::breadth::{
    breadth_thrust, bullish_percent_index, mcclellan_oscillator, tick_index,
};

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
