//! 图示模块 - 为关键知识点提供 ASCII/SVG 图示
//!
//! 每个函数返回一个 `String`,在知识库 entry 的 diagram 字段中使用。

/// RSI 图示 - 三段(超卖/中性/超买)
pub fn rsi_diagram() -> String {
    r#"RSI 0-100 范围:
   0 ───────── 30 ───────── 50 ───────── 70 ─────── 100
   ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
   超卖区(买)    中性区间     超买区(卖)
   30 阈值       50 中轴       70 阈值
"#.to_string()
}

/// MACD 图示 - 柱状图在零轴上下翻转
pub fn macd_diagram() -> String {
    r#"MACD 柱状图与零轴:
                  ┌─── 多头(柱在零上)
                  │
        ▓▓▓ ▓▓▓    │
        ▓▓▓ ▓▓▓ ───┼─── 零轴
        ▓▓▓ ▓▓▓    │
        ▓▓▓ ▓▓▓    │
                  └─── 空头(柱在零下)
   金叉:    死叉:   顶背离:
   MACD 柱   MACD 柱   价格↑而
   由负转正  由正转负  MACD 柱↓
"#.to_string()
}

/// Bollinger Bands 图示 - 三个轨道包络价格
pub fn bollinger_diagram() -> String {
    r#"Bollinger Bands (20, 2σ):
   上轨(均价+2σ)
   ─────────────────
     ╱─╲    ╱─╲
    ╱   ╲──╱   ╲── 价格
   ─────────────────
   中轨(20 SMA)
   ─────────────────
   下轨(均价-2σ)
   Squeeze: 上下轨极度收窄 → 即将大幅波动
   触上轨: 超买;触下轨: 超卖
"#.to_string()
}

/// Ichimoku 云图 - 五条线+云带
pub fn ichimoku_diagram() -> String {
    r#"一目均衡表 (9, 26, 52, 26):
                  迟行线
                  ↓
   价格 ──╲  ╱────── 价格
           ╲╱ (云之上=多头)
   ════════════ 转换线
   ════════════ 基准线
   ╲╲ 云: 绿(多)/红(空) ╱╱
"#.to_string()
}

/// Sharpe Ratio 图示 - 收益 vs 波动
pub fn sharpe_diagram() -> String {
    r#"Sharpe Ratio = (收益 - 无风险) / 波动率:
   0.0  0.5  1.0  1.5  2.0  2.5  3.0
   ├────┼────┼────┼────┼────┼────┤
   差   凑合  合格  良好  优秀  顶尖
"#.to_string()
}

/// 最大回撤 图示 - 峰值跌到谷底
pub fn drawdown_diagram() -> String {
    r#"最大回撤 = 峰值跌到谷底的幅度:
   净值
   ↑
   │╱╲
   │   ╲      ╱╲
   │    ╲    ╱  ╲
   │     ╲  ╱    ╲
   │      ╲╱      ╲
   ├─────────────── 谷底
   ↑
   峰值
   回撤% = (峰值 - 谷底) / 峰值
"#.to_string()
}

/// KDJ 图示 - 三线在 0-100 区间震荡
pub fn kdj_diagram() -> String {
    r#"KDJ 指标 (9, 3, 3):
   100 ────── 超买区(死叉)
        ╱╲
       ╱  ╲ ╱╲
   20  ╱  K ╲╱  ╲
   ────╱──D─╲──── J=3K-2D
      ╲   ╱  ╲ ╱
       ╲ ╱    ╲
   0   ──── 超卖区(金叉)
"#.to_string()
}

/// OBV 图示 - 能量潮累积
pub fn obv_diagram() -> String {
    r#"OBV 能量潮:
   价格     OBV (累计成交量)
   ↑     ↗ 持续新高 = 资金流入
   │   ╱
   │  ╱
   ↓ ╱
     ╲ 持续新低 = 资金流出
   OBV 创新高 + 价格未创新高 = 潜在买入信号
"#.to_string()
}

/// 通用 fallback - ASCII
pub fn generic_diagram(name: &str) -> String {
    format!("{} 关键点: 详见 PDF 详细图表 (本项目侧重代码实现)", name)
}
