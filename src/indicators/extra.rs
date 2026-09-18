//! 额外指标和工具:Heikin Ashi、Z-Score、形态识别
//!
//! 这些是对"学习中心"中知识概念的额外实践实现,
//! 补全"非学习中心"部分对 PDF 知识点的覆盖。

use crate::types::Bar;

/// Heikin Ashi K 线
/// 平滑后的蜡烛图,比标准 K 线更清晰显示趋势
/// HA Close = (Open + High + Low + Close) / 4
/// HA Open = (前 HA Open + 前 HA Close) / 2
/// HA High = max(High, HA Open, HA Close)
/// HA Low = min(Low, HA Open, HA Close)
pub fn heikin_ashi(bars: &[Bar]) -> Vec<Bar> {
    let mut result = Vec::with_capacity(bars.len());
    let mut prev_ha_open = bars[0].open;
    let mut prev_ha_close = (bars[0].open + bars[0].high + bars[0].low + bars[0].close) / 4.0;

    for bar in bars {
        let ha_close = (bar.open + bar.high + bar.low + bar.close) / 4.0;
        let ha_open = (prev_ha_open + prev_ha_close) / 2.0;
        let ha_high = bar.high.max(ha_open).max(ha_close);
        let ha_low = bar.low.min(ha_open).min(ha_close);
        result.push(Bar {
            timestamp: bar.timestamp,
            open: ha_open,
            high: ha_high,
            low: ha_low,
            close: ha_close,
            volume: bar.volume,
        });
        prev_ha_open = ha_open;
        prev_ha_close = ha_close;
    }
    result
}

/// K 线形态识别:返回形态名称列表
/// PDF 第二十二章 K 线形态部分
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CandlePattern {
    Doji,           // 十字星:开收几乎相等
    Hammer,         // 锤子线:下影线 > 实体 × 2
    ShootingStar,   // 流星:上影线 > 实体 × 2
    Marubozu,       // 光头光脚:实体几乎占满整根 K 线
    Engulfing,      // 吞没:后一根实体完全包裹前一根(且方向相反)
    None,
}

impl CandlePattern {
    pub fn name_zh(&self) -> &'static str {
        match self {
            Self::Doji => "十字星",
            Self::Hammer => "锤子线",
            Self::ShootingStar => "流星线",
            Self::Marubozu => "光头光脚",
            Self::Engulfing => "吞没形态",
            Self::None => "无特殊形态",
        }
    }
}

pub fn detect_pattern(bar: &Bar) -> CandlePattern {
    let body = (bar.close - bar.open).abs();
    let range = bar.high - bar.low;
    let upper_shadow = bar.high - bar.close.max(bar.open);
    let lower_shadow = bar.close.min(bar.open) - bar.low;

    if range == 0.0 {
        return CandlePattern::None;
    }

    // 十字星:实体极小 (< 范围的 10%)
    if body / range < 0.1 {
        return CandlePattern::Doji;
    }
    // 锤子线:下影线 > 实体 × 2,上影线很短
    if lower_shadow > body * 2.0 && upper_shadow < body * 0.5 {
        return CandlePattern::Hammer;
    }
    // 流星:上影线 > 实体 × 2,下影线很短
    if upper_shadow > body * 2.0 && lower_shadow < body * 0.5 {
        return CandlePattern::ShootingStar;
    }
    // 光头光脚:实体 > 范围 × 90%
    if body / range > 0.9 {
        return CandlePattern::Marubozu;
    }
    CandlePattern::None
}

/// Z-Score:价格偏离均值的标准差倍数
/// 配对交易、统计套利中判断"价格是否异常"的核心指标
pub fn zscore(series: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = series.len();
    let mut out = vec![None; n];
    if n < period || period == 0 {
        return out;
    }
    for i in period - 1..n {
        let window = &series[i + 1 - period..=i];
        let mean: f64 = window.iter().sum::<f64>() / period as f64;
        let var: f64 = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
        let std = var.sqrt();
        if std > 0.0 {
            out[i] = Some((series[i] - mean) / std);
        }
    }
    out
}

/// 简单的 K 线摘要(用于数据探索面板)
pub fn bar_summary(bar: &Bar) -> String {
    let body = bar.close - bar.open;
    let pct = if bar.open != 0.0 { body / bar.open * 100.0 } else { 0.0 };
    let direction = if body > 0.0 { "↑ 阳线" } else if body < 0.0 { "↓ 阴线" } else { "─ 平" };
    format!(
        "{} 开盘 {:.2} 收盘 {:.2} 涨跌 {:+.2}% (幅度 {:.2}) 成交量 {:.0}",
        direction, bar.open, bar.close, pct, bar.high - bar.low, bar.volume
    )
}