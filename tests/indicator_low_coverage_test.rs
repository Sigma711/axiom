use approx::assert_relative_eq;
use axiom::indicators::{
    accrual_ratio, altman_z, beneish_m_score, buyback_ratio, current_ratio, days_to_cover,
    debt_to_assets, debt_to_equity, dupont, free_float_ratio, goodwill_to_equity,
    holder_concentration_change, interest_coverage, net_debt, net_debt_to_ebitda,
    piotroski_f_score, quick_ratio, roa, roe, share_pledge_ratio, shares_per_holder,
    short_interest_ratio, unlock_market_value, CompanyFinancials, FScoreInput,
};

fn financials() -> CompanyFinancials {
    CompanyFinancials {
        price: 120.0,
        eps: 6.0,
        book_value_per_share: 30.0,
        revenue_per_share: 40.0,
        ev_ebitda: 8.0,
        growth_rate: 0.2,
        dividend_per_share: 2.4,
        stock_price_target: 150.0,
        earnings_yield: 0.05,
    }
}

#[test]
fn company_valuation_ratios_match_their_independent_arithmetic() {
    let f = financials();
    assert_relative_eq!(f.pe(), 20.0);
    assert_relative_eq!(f.pb(), 4.0);
    assert_relative_eq!(f.ps(), 3.0);
    assert_relative_eq!(f.peg(), 1.0);
    assert_relative_eq!(f.ev_ebitda_ratio(), 8.0);
    assert_relative_eq!(f.dividend_yield(), 0.02);
    assert_relative_eq!(f.payout_ratio(), 0.4);
    assert_relative_eq!(f.earnings_yield(), 0.05);
    assert_relative_eq!(f.target_upside(), 0.25);

    let zero = CompanyFinancials {
        eps: 0.0,
        price: 0.0,
        book_value_per_share: 0.0,
        revenue_per_share: 0.0,
        growth_rate: 0.0,
        ..f
    };
    assert_eq!(zero.pe(), 0.0);
    assert_eq!(zero.pb(), 0.0);
    assert_eq!(zero.ps(), 0.0);
    assert_eq!(zero.peg(), 0.0);
    assert_eq!(zero.dividend_yield(), 0.0);
    assert_eq!(zero.payout_ratio(), 0.0);
    assert_eq!(zero.earnings_yield(), 0.0);
    assert_eq!(zero.target_upside(), 0.0);
}

#[test]
fn financial_health_ratios_cover_normal_and_zero_denominator_cases() {
    assert_relative_eq!(dupont(0.1, 1.5, 2.0), 0.3);
    assert_relative_eq!(
        altman_z(120.0, 100.0, 80.0, 300.0, 200.0, 500.0, 1_000.0),
        1.2 * 0.12 + 1.4 * 0.1 + 3.3 * 0.08 + 0.6 * 1.5 + 0.5
    );
    assert_eq!(altman_z(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0), 0.0);
    assert_relative_eq!(accrual_ratio(130.0, 100.0, 600.0), 0.05);
    assert_relative_eq!(roe(20.0, 100.0), 0.2);
    assert_eq!(roe(1.0, 0.0), 0.0);
    assert_relative_eq!(roa(20.0, 200.0), 0.1);
    assert_eq!(roa(1.0, 0.0), 0.0);
    assert_relative_eq!(current_ratio(300.0, 150.0), 2.0);
    assert_eq!(current_ratio(1.0, 0.0), 0.0);
    assert_relative_eq!(quick_ratio(180.0, 90.0), 2.0);
    assert_eq!(quick_ratio(1.0, 0.0), 0.0);
    assert_relative_eq!(debt_to_assets(200.0, 800.0), 0.25);
    assert_eq!(debt_to_assets(1.0, 0.0), 0.0);
    assert_relative_eq!(debt_to_equity(200.0, 400.0), 0.5);
    assert_eq!(debt_to_equity(1.0, 0.0), 0.0);
    assert_relative_eq!(interest_coverage(60.0, 15.0), 4.0);
    assert_eq!(interest_coverage(1.0, 0.0), 0.0);
    assert_relative_eq!(net_debt(250.0, 90.0), 160.0);
    assert_relative_eq!(net_debt_to_ebitda(250.0, 90.0, 40.0), 4.0);
    assert_eq!(net_debt_to_ebitda(1.0, 1.0, 0.0), 0.0);
}

#[test]
fn shareholder_metrics_and_quality_scores_match_their_definitions() {
    assert_relative_eq!(free_float_ratio(60.0, 100.0), 0.6);
    assert_eq!(free_float_ratio(1.0, 0.0), 0.0);
    assert_relative_eq!(shares_per_holder(1_000.0, 20.0), 50.0);
    assert_eq!(shares_per_holder(1.0, 0.0), 0.0);
    assert_relative_eq!(holder_concentration_change(120.0, 100.0), 0.2);
    assert_eq!(holder_concentration_change(1.0, 0.0), 0.0);
    assert_relative_eq!(goodwill_to_equity(10.0, 5.0, 50.0), 0.3);
    assert_eq!(goodwill_to_equity(1.0, 1.0, 0.0), 0.0);
    assert_relative_eq!(share_pledge_ratio(20.0, 100.0), 0.2);
    assert_eq!(share_pledge_ratio(1.0, 0.0), 0.0);
    assert_relative_eq!(unlock_market_value(8.0, 12.5), 100.0);
    assert_relative_eq!(buyback_ratio(5.0, 100.0), 0.05);
    assert_eq!(buyback_ratio(1.0, 0.0), 0.0);
    assert_relative_eq!(short_interest_ratio(4.0, 80.0), 0.05);
    assert_eq!(short_interest_ratio(1.0, 0.0), 0.0);
    assert_relative_eq!(days_to_cover(100.0, 20.0), 5.0);
    assert_eq!(days_to_cover(1.0, 0.0), 0.0);
    let all_good = FScoreInput {
        positive_roa: true,
        positive_operating_cf: true,
        roa_increasing: true,
        accruals_decreasing: true,
        current_ratio_improved: true,
        shares_issued: false,
        leverage_decreased: true,
        gross_margin_increased: true,
        asset_turnover_increased: true,
    };
    assert_eq!(piotroski_f_score(all_good), 9);
    assert_relative_eq!(
        beneish_m_score(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0),
        -4.84 + 0.92 + 0.528 + 0.404 + 0.892 + 0.115 - 0.172 - 0.327
    );
}
