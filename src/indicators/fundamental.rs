//! 基本面分析指标 —— PE / PB / ROE 等
//!
//! PDF 第四部分-四、估值指标 + 五、盈利能力与经营质量 + 七、资产负债与偿债指标
//!
//! 注意: 这些指标需要公司财报数据,本项目主要做技术分析;
//! 但作为"理论概念 + 计算方法 + 默认示例"已经完整实现,
//! 用户可以用真实的公司数据喂进来。

/// 估值指标结构:输入公司财报数据,计算所有 PDF 第七章的估值比率。
#[derive(Debug, Clone)]
pub struct CompanyFinancials {
    pub price: f64,                // 当前股价
    pub eps: f64,                  // 每股收益
    pub book_value_per_share: f64, // 每股净资产
    pub revenue_per_share: f64,    // 每股营收
    pub ev_ebitda: f64,            // EV/EBITDA 倍数
    pub growth_rate: f64,          // 盈利增长率 (例如 0.15 = 15%)
    pub dividend_per_share: f64,   // 每股股息
    pub stock_price_target: f64,   // 分析师目标价
    pub earnings_yield: f64,       // 盈利收益率 (0.05 = 5%)
}

impl CompanyFinancials {
    /// 市盈率 PE = 股价 / EPS
    /// PDF 第七章 2 节
    pub fn pe(&self) -> f64 {
        if self.eps == 0.0 {
            return 0.0;
        }
        self.price / self.eps
    }
    /// 市净率 PB = 股价 / 每股净资产
    /// PDF 第七章 3 节
    pub fn pb(&self) -> f64 {
        if self.book_value_per_share == 0.0 {
            return 0.0;
        }
        self.price / self.book_value_per_share
    }
    /// 市销率 PS = 股价 / 每股营收 (或 总市值 / 总营收)
    /// PDF 第七章 5 节
    pub fn ps(&self) -> f64 {
        if self.revenue_per_share == 0.0 {
            return 0.0;
        }
        self.price / self.revenue_per_share
    }
    /// PEG = PE / 盈利增长率(%)
    /// PDF 第七章 7 节
    pub fn peg(&self) -> f64 {
        if self.growth_rate == 0.0 {
            return 0.0;
        }
        self.pe() / (self.growth_rate * 100.0)
    }
    /// EV/EBITDA 倍数
    /// PDF 第七章 9 节
    pub fn ev_ebitda_ratio(&self) -> f64 {
        self.ev_ebitda
    }
    /// 股息率 = 每股股息 / 股价
    /// PDF 第七章 12 节
    pub fn dividend_yield(&self) -> f64 {
        if self.price == 0.0 {
            return 0.0;
        }
        self.dividend_per_share / self.price
    }
    /// 分红支付率 = 每股股息 / EPS
    /// PDF 第七章 13 节
    pub fn payout_ratio(&self) -> f64 {
        if self.eps == 0.0 {
            return 0.0;
        }
        self.dividend_per_share / self.eps
    }
    /// 盈利收益率 = EPS / 股价
    /// PDF 第七章 14 节
    pub fn earnings_yield(&self) -> f64 {
        if self.price == 0.0 {
            return 0.0;
        }
        self.eps / self.price
    }
    /// 目标价上涨空间 = (目标价 / 现价) - 1
    /// PDF 第二十五章 2 节
    pub fn target_upside(&self) -> f64 {
        if self.price == 0.0 {
            return 0.0;
        }
        self.stock_price_target / self.price - 1.0
    }
}

/// 杜邦分析 —— ROE 拆解为三因子
/// PDF 第七章 10 节
/// ROE = 净利率 × 资产周转率 × 权益乘数
pub fn dupont(net_margin: f64, asset_turnover: f64, equity_multiplier: f64) -> f64 {
    net_margin * asset_turnover * equity_multiplier
}

/// Z-Score (Altman) —— 财务困境预测
/// PDF 第七章 3 节
/// Z = 1.2A + 1.4B + 3.3C + 0.6D + 1.0E
/// A=营运资金/总资产, B=留存收益/总资产, C=EBIT/总资产, D=市值/总负债, E=销售额/总资产
pub fn altman_z(
    working_capital: f64,
    retained_earnings: f64,
    ebit: f64,
    market_value: f64,
    total_liabilities: f64,
    sales: f64,
    total_assets: f64,
) -> f64 {
    if total_assets == 0.0 {
        return 0.0;
    }
    let wc_ratio = working_capital / total_assets;
    let re_ratio = retained_earnings / total_assets;
    let ebit_ratio = ebit / total_assets;
    let mv_ratio = if total_liabilities == 0.0 {
        0.0
    } else {
        market_value / total_liabilities
    };
    let sales_ratio = sales / total_assets;
    1.2 * wc_ratio + 1.4 * re_ratio + 3.3 * ebit_ratio + 0.6 * mv_ratio + 1.0 * sales_ratio
}

/// 应计比率 Accrual Ratio (Sloan) —— 盈利质量
/// PDF 第七章 1 节
/// Accrual = (净利润 - 经营现金流) / 总资产
/// > 0 越多说明利润"算"出来的成分越多,质量越差
pub fn accrual_ratio(net_income: f64, operating_cash_flow: f64, total_assets: f64) -> f64 {
    if total_assets == 0.0 {
        return 0.0;
    }
    (net_income - operating_cash_flow) / total_assets
}

/// ROE = 净利润 / 平均股东权益
/// PDF 第七章 6 节
pub fn roe(net_income: f64, avg_shareholders_equity: f64) -> f64 {
    if avg_shareholders_equity == 0.0 {
        return 0.0;
    }
    net_income / avg_shareholders_equity
}

/// ROA = 净利润 / 平均总资产
/// PDF 第七章 7 节
pub fn roa(net_income: f64, avg_total_assets: f64) -> f64 {
    if avg_total_assets == 0.0 {
        return 0.0;
    }
    net_income / avg_total_assets
}

/// 流动比率 = 流动资产 / 流动负债
/// PDF 第七章 16 节
pub fn current_ratio(current_assets: f64, current_liabilities: f64) -> f64 {
    if current_liabilities == 0.0 {
        return 0.0;
    }
    current_assets / current_liabilities
}

/// 速动比率 = (流动资产 - 存货) / 流动负债
/// PDF 第七章 17 节
pub fn quick_ratio(quick_assets: f64, current_liabilities: f64) -> f64 {
    if current_liabilities == 0.0 {
        return 0.0;
    }
    quick_assets / current_liabilities
}

/// 资产负债率 = 总负债 / 总资产
/// PDF 第七章 1 节
pub fn debt_to_assets(total_liabilities: f64, total_assets: f64) -> f64 {
    if total_assets == 0.0 {
        return 0.0;
    }
    total_liabilities / total_assets
}

/// 负债权益比 D/E = 总负债 / 股东权益
/// PDF 第七章 2 节
pub fn debt_to_equity(total_liabilities: f64, equity: f64) -> f64 {
    if equity == 0.0 {
        return 0.0;
    }
    total_liabilities / equity
}

/// 利息保障倍数 = EBIT / 利息费用
/// PDF 第七章 5 节
pub fn interest_coverage(ebit: f64, interest_expense: f64) -> f64 {
    if interest_expense == 0.0 {
        return 0.0;
    }
    ebit / interest_expense
}

/// 净债务 = 总债务 - 现金
/// PDF 第七章 3 节
pub fn net_debt(total_debt: f64, cash: f64) -> f64 {
    total_debt - cash
}

/// 净债务/EBITDA = (总债务 - 现金) / EBITDA
/// PDF 第七章 4 节
pub fn net_debt_to_ebitda(total_debt: f64, cash: f64, ebitda: f64) -> f64 {
    if ebitda == 0.0 {
        return 0.0;
    }
    (total_debt - cash) / ebitda
}
