//! 技术指标库 —— 所有指标的纯函数实现。
//!
//! 设计原则:
//!   - 每个函数输入价格/成交量数据,返回对齐的指标值序列(`Vec<Option<f64>>`)
//!   - 前 N 个值通常是 `None`,因为需要预热(lookback period)
//!   - 纯函数,无副作用,易于测试
//!   - 来自《股票交易软件专业指标全解》,每条注释都对应 PDF 里的章节

pub mod breadth; // 市场宽度:ADL/TRIN/McClellan
pub mod extra; // 额外:Heikin Ashi、形态识别、Z-Score
pub mod fundamental; // 基本面:PE/PB/ROE/ROA 等
pub mod ma; // 移动平均线及其变体
pub mod momentum; // 动量类(RSI, KDJ, Stochastic, ...)
pub mod options; // 期权:Black-Scholes + Greeks
pub mod shareholder;
pub mod statistics; // 统计与回归类
pub mod trend; // 趋势类(MACD, DMI, Ichimoku, ...)
pub mod volatility; // 波动率类(ATR, BBands, Keltner, ...)
pub mod volume; // 成交量与资金流类 // 股东:质押/解禁/F-Score/M-Score

// 重导出常用类型
pub use breadth::*;
pub use extra::{
    bar_summary, detect_engulfing, detect_inside_outside, detect_pattern, detect_star, heikin_ashi,
    pivot_points, zscore as extra_zscore, CandlePattern, PivotPoints,
};
pub use fundamental::*;
pub use ma::*;
pub use momentum::*;
pub use options::*;
pub use shareholder::*;
pub use statistics::*;
pub use trend::*;
pub use volatility::*;
pub use volume::*;
