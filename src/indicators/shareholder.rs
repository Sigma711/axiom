//! 股东 / 股本 指标 —— PDF 第十章
//!
//! 这些指标需要外部财报/公告数据(中登/SEC Form 4),
//! 但作为"理论概念 + 计算方法"已经完整实现,
//! 用户喂入真实数据即可使用。

/// 流通股比例 = 流通股 / 总股本
/// PDF 第十章 9 节
pub fn free_float_ratio(free_float_shares: f64, total_shares: f64) -> f64 {
    if total_shares == 0.0 {
        return 0.0;
    }
    free_float_shares / total_shares
}

/// 股东户均持股 = 总股本 / 股东户数
/// PDF 第十章 6 节
pub fn shares_per_holder(total_shares: f64, num_holders: f64) -> f64 {
    if num_holders == 0.0 {
        return 0.0;
    }
    total_shares / num_holders
}

/// 户均持股变化 = (本期户均 - 上期户均) / 上期户均
/// PDF 第十章 6 节
/// 正值: 筹码集中(吸筹)
/// 负值: 筹码分散(派发)
pub fn holder_concentration_change(shares_now: f64, shares_prev: f64) -> f64 {
    if shares_prev == 0.0 {
        return 0.0;
    }
    (shares_now - shares_prev) / shares_prev
}

/// 商誉/净资产比 = (商誉 + 无形资产) / 净资产
/// PDF 第十章 11 节
/// > 30% 警惕商誉减值风险
pub fn goodwill_to_equity(goodwill: f64, intangibles: f64, equity: f64) -> f64 {
    if equity == 0.0 {
        return 0.0;
    }
    (goodwill + intangibles) / equity
}

/// 股权质押比例 = 质押股数 / 总股本
/// PDF 第十章 8 节
/// > 50% 警惕平仓风险
pub fn share_pledge_ratio(pledged_shares: f64, total_shares: f64) -> f64 {
    if total_shares == 0.0 {
        return 0.0;
    }
    pledged_shares / total_shares
}

/// 解禁市值 = 解禁股数 × 当前价
/// PDF 第十章 7 节
pub fn unlock_market_value(unlock_shares: f64, current_price: f64) -> f64 {
    unlock_shares * current_price
}

/// 回购率 = 回购股数 / 总股本
/// PDF 第十章 9 节
pub fn buyback_ratio(buyback_shares: f64, total_shares: f64) -> f64 {
    if total_shares == 0.0 {
        return 0.0;
    }
    buyback_shares / total_shares
}

/// 做空比例 (Short Interest) = 卖空股数 / 流通股
/// PDF 第十章 10 节
/// 美股: 需 FINRA 半月报
pub fn short_interest_ratio(short_shares: f64, float_shares: f64) -> f64 {
    if float_shares == 0.0 {
        return 0.0;
    }
    short_shares / float_shares
}

/// 平仓天数 (Days to Cover) = 卖空股数 / 日均成交量
/// PDF 第十章 10 节
/// > 5 警惕轧空风险
pub fn days_to_cover(short_shares: f64, avg_daily_volume: f64) -> f64 {
    if avg_daily_volume == 0.0 {
        return 0.0;
    }
    short_shares / avg_daily_volume
}

/// Piotroski F-Score (9 因子质量评分)
/// PDF 第十章 2 节
/// 输入 0-9 的各项指标(满足 = 1, 不满足 = 0)
pub struct FScoreInput {
    pub positive_roa: bool,             // 1. ROA > 0
    pub positive_operating_cf: bool,    // 2. 经营现金流 > 0
    pub roa_increasing: bool,           // 3. ROA 同比上升
    pub accruals_decreasing: bool,      // 4. 应计减少
    pub current_ratio_improved: bool,   // 5. 流动比率改善
    pub shares_issued: bool,            // 6. 没有新股发行
    pub gross_margin_increased: bool,   // 7. 毛利率上升
    pub asset_turnover_increased: bool, // 8. 资产周转率上升
}

pub fn piotroski_f_score(input: FScoreInput) -> i32 {
    let mut score = 0;
    if input.positive_roa {
        score += 1;
    }
    if input.positive_operating_cf {
        score += 1;
    }
    if input.roa_increasing {
        score += 1;
    }
    if input.accruals_decreasing {
        score += 1;
    }
    if input.current_ratio_improved {
        score += 1;
    }
    if input.shares_issued {
        score += 1;
    }
    if input.gross_margin_increased {
        score += 1;
    }
    if input.asset_turnover_increased {
        score += 1;
    }
    score
}

/// Beneish M-Score (8 因子)
/// PDF 第十章 4 节
/// > -1.78 警惕财务操纵
pub fn beneish_m_score(
    dsri: f64, // 应收账款指数 (Days Sales Receivables Index)
    gmi: f64,  // 毛利率指数 (Gross Margin Index)
    aqi: f64,  // 资产质量指数 (Asset Quality Index)
    sgi: f64,  // 销售增长指数 (Sales Growth Index)
    depi: f64, // 折旧指数 (Depreciation Index)
    sgai: f64, // 销售一般管理费用指数
    lvgi: f64, // 杠杆指数 (Leverage Index)
    tata: f64, // 流动资产周转率 (Total Accruals To Total Assets)
) -> f64 {
    -4.84 + 0.92 * dsri + 0.528 * gmi + 0.404 * aqi + 0.892 * sgi + 0.115 * depi - 0.172 * sgai
        + 4.679 * tata
        - 0.327 * lvgi
}
