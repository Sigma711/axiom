//! AXIOM —— 策略库

use crate::types::{Bar, Side, Signal};
use std::collections::HashMap;

// ============================================================================
// 工具:History 与 SMA
// ============================================================================

pub struct History {
    pub bars: Vec<Bar>,
}

impl History {
    pub fn new() -> Self { Self { bars: Vec::new() } }
    pub fn push(&mut self, bar: &Bar) { self.bars.push(*bar); }
    pub fn reset(&mut self) { self.bars.clear(); }
    pub fn closes(&self, n: Option<usize>) -> Vec<f64> {
        let cs: Vec<f64> = self.bars.iter().map(|b| b.close).collect();
        match n {
            Some(k) if k <= cs.len() => cs[cs.len() - k..].to_vec(),
            _ => cs,
        }
    }
    pub fn sma(&self, n: usize) -> Option<f64> {
        if self.bars.len() < n || n == 0 { return None; }
        Some(self.bars.iter().rev().take(n).map(|b| b.close).sum::<f64>() / n as f64)
    }
}

// ============================================================================
// Strategy trait
// ============================================================================

pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn params(&self) -> HashMap<String, f64>;
    fn on_bar(&mut self, bar: &Bar) -> Signal;
    fn reset(&mut self);
    fn _min_history(&self) -> usize { 1 }
}

fn make_signal(bar: &Bar, side: Side, strength: f64, reason: String) -> Signal {
    Signal { timestamp: bar.timestamp, side, strength, reason, target_size: None }
}

fn hold(bar: &Bar, reason: &str) -> Signal {
    make_signal(bar, Side::Hold, 0.0, reason.to_string())
}

// ============================================================================
// BuyAndHoldStrategy
// ============================================================================

pub struct BuyAndHoldStrategy { bought: bool }
impl BuyAndHoldStrategy { pub fn new() -> Self { Self { bought: false } } }
impl Default for BuyAndHoldStrategy { fn default() -> Self { Self::new() } }
impl Strategy for BuyAndHoldStrategy {
    fn name(&self) -> &str { "buy_and_hold" }
    fn params(&self) -> HashMap<String, f64> { HashMap::new() }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        if !self.bought {
            self.bought = true;
            make_signal(bar, Side::Buy, 1.0, "基准策略:首根 K 全仓买入并持有".into())
        } else { hold(bar, "持有中") }
    }
    fn reset(&mut self) { self.bought = false; }
}

// ============================================================================
// SmaCrossStrategy
// ============================================================================

pub struct SmaCrossStrategy {
    history: History,
    fast: usize,
    slow: usize,
    prev_diff: Option<f64>,
}
impl SmaCrossStrategy {
    pub fn new(fast: usize, slow: usize) -> Self {
        Self { history: History::new(), fast, slow, prev_diff: None }
    }
    fn min_history(&self) -> usize { self.slow + 1 }
}
impl Strategy for SmaCrossStrategy {
    fn name(&self) -> &str { "sma_cross" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("fast".into(), self.fast as f64);
        m.insert("slow".into(), self.slow as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.min_history() {
            return hold(bar, "等待预热");
        }
        let f = self.history.sma(self.fast).unwrap();
        let s = self.history.sma(self.slow).unwrap();
        let diff = f - s;
        let prev = self.prev_diff;
        self.prev_diff = Some(diff);
        if let Some(p) = prev {
            if p <= 0.0 && diff > 0.0 {
                return make_signal(bar, Side::Buy, 0.8, format!("金叉 f{:.2} 上穿 s{:.2}", f, s));
            }
            if p >= 0.0 && diff < 0.0 {
                return make_signal(bar, Side::Sell, 0.8, format!("死叉 f{:.2} 下穿 s{:.2}", f, s));
            }
        }
        hold(bar, &format!("无交叉 f{:.2} s{:.2}", f, s))
    }
    fn reset(&mut self) { self.history.reset(); self.prev_diff = None; }
}

// ============================================================================
// RsiStrategy
// ============================================================================

pub struct RsiStrategy {
    history: History,
    period: usize,
    overbought: f64,
    oversold: f64,
    prev_rsi: Option<f64>,
}
impl RsiStrategy {
    pub fn new(period: usize, overbought: f64, oversold: f64) -> Self {
        Self { history: History::new(), period, overbought, oversold, prev_rsi: None }
    }
    fn min_history(&self) -> usize { self.period + 1 }
    fn calc_rsi(&self) -> Option<f64> {
        if self.history.bars.len() < self.min_history() { return None; }
        let closes = self.history.closes(None);
        let (mut ag, mut al) = (0.0, 0.0);
        for i in 1..=self.period {
            let d = closes[i] - closes[i - 1];
            if d > 0.0 { ag += d; } else { al -= d; }
        }
        ag /= self.period as f64; al /= self.period as f64;
        if al == 0.0 { return Some(100.0); }
        Some(100.0 - 100.0 / (1.0 + ag / al))
    }
}
impl Strategy for RsiStrategy {
    fn name(&self) -> &str { "rsi" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("period".into(), self.period as f64);
        m.insert("overbought".into(), self.overbought);
        m.insert("oversold".into(), self.oversold);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        let rsi = match self.calc_rsi() { Some(v) => v, None => return hold(bar, "RSI 未就绪") };
        let prev = self.prev_rsi; self.prev_rsi = Some(rsi);
        if let Some(p) = prev {
            if p <= self.oversold && rsi > self.oversold {
                return make_signal(bar, Side::Buy, 0.8, format!("RSI 从 {:.1} 反弹", p));
            }
            if p >= self.overbought && rsi < self.overbought {
                return make_signal(bar, Side::Sell, 0.8, format!("RSI 从 {:.1} 回落", p));
            }
        }
        hold(bar, &format!("RSI={:.1}", rsi))
    }
    fn reset(&mut self) { self.history.reset(); self.prev_rsi = None; }
}

// ============================================================================
// RandomStrategy (冒烟测试)
// ============================================================================

pub struct RandomStrategy { buy_prob: f64, sell_prob: f64, rng: rand::rngs::StdRng }
impl RandomStrategy {
    pub fn new(seed: u64, buy_prob: f64, sell_prob: f64) -> Self {
        use rand::SeedableRng;
        Self { buy_prob, sell_prob, rng: rand::rngs::StdRng::seed_from_u64(seed) }
    }
}
impl Strategy for RandomStrategy {
    fn name(&self) -> &str { "random" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("buy_prob".into(), self.buy_prob);
        m.insert("sell_prob".into(), self.sell_prob);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        use rand::Rng;
        let r: f64 = self.rng.gen_range(0.0..1.0);
        if r < self.buy_prob { make_signal(bar, Side::Buy, 0.5, "随机买入".into()) }
        else if r > 1.0 - self.sell_prob { make_signal(bar, Side::Sell, 0.5, "随机卖出".into()) }
        else { hold(bar, "随机观望") }
    }
    fn reset(&mut self) {}
}

// ============================================================================
// MacdStrategy
// ============================================================================

pub struct MacdStrategy {
    history: History,
    fast: usize, slow: usize, signal_period: usize,
    prev_diff: Option<f64>,
}
impl MacdStrategy {
    pub fn new(fast: usize, slow: usize, signal_period: usize) -> Self {
        Self { history: History::new(), fast, slow, signal_period, prev_diff: None }
    }
    fn min_history(&self) -> usize { self.slow + self.signal_period }
}
impl Strategy for MacdStrategy {
    fn name(&self) -> &str { "macd" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("fast".into(), self.fast as f64);
        m.insert("slow".into(), self.slow as f64);
        m.insert("signal".into(), self.signal_period as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.min_history() { return hold(bar, "等待预热"); }
        let closes = self.history.closes(None);
        let n = closes.len();
        let ema_f = crate::indicators::ma::ema(&closes, self.fast);
        let ema_s = crate::indicators::ma::ema(&closes, self.slow);
        let dif: Vec<f64> = (0..n).map(|i| match (ema_f[i], ema_s[i]) {
            (Some(f), Some(s)) => f - s, _ => 0.0 }).collect();
        let dea = crate::indicators::ma::ema(&dif, self.signal_period);
        let d = dif[n - 1];
        let dea_v = dea[n - 1].unwrap_or(0.0);
        let diff = d - dea_v;
        let prev = self.prev_diff; self.prev_diff = Some(diff);
        if let Some(p) = prev {
            if p <= 0.0 && diff > 0.0 {
                return make_signal(bar, Side::Buy, 0.8, format!("MACD 金叉 DIF={:.4} DEA={:.4}", d, dea_v));
            }
            if p >= 0.0 && diff < 0.0 {
                return make_signal(bar, Side::Sell, 0.8, format!("MACD 死叉 DIF={:.4} DEA={:.4}", d, dea_v));
            }
        }
        hold(bar, &format!("MACD DIF={:.4} DEA={:.4}", d, dea_v))
    }
    fn reset(&mut self) { self.history.reset(); self.prev_diff = None; }
}

// ============================================================================
// BollingerBandsStrategy
// ============================================================================

pub struct BollingerBandsStrategy {
    history: History, period: usize, num_std: f64,
}
impl BollingerBandsStrategy {
    pub fn new(period: usize, num_std: f64) -> Self {
        Self { history: History::new(), period, num_std }
    }
    fn min_history(&self) -> usize { self.period }
}
impl Strategy for BollingerBandsStrategy {
    fn name(&self) -> &str { "bollinger" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("period".into(), self.period as f64);
        m.insert("num_std".into(), self.num_std);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.min_history() { return hold(bar, "等待预热"); }
        let closes = self.history.closes(None);
        let bb = crate::indicators::volatility::bollinger_bands(&closes, self.period, self.num_std);
        let n = closes.len();
        let lower = bb.lower[n - 1].unwrap();
        let upper = bb.upper[n - 1].unwrap();
        let mid = bb.middle[n - 1].unwrap();
        if bar.close <= lower {
            return make_signal(bar, Side::Buy, 0.8, format!("触下轨 c={:.2} lower={:.2}", bar.close, lower));
        }
        if bar.close >= upper {
            return make_signal(bar, Side::Sell, 0.8, format!("触上轨 c={:.2} upper={:.2}", bar.close, upper));
        }
        hold(bar, &format!("布林带内 mid={:.2}", mid))
    }
    fn reset(&mut self) { self.history.reset(); }
}

// ============================================================================
// SupertrendStrategy
// ============================================================================

pub struct SupertrendStrategy {
    history: History, period: usize, multiplier: f64, last_dir: i8,
}
impl SupertrendStrategy {
    pub fn new(period: usize, multiplier: f64) -> Self {
        Self { history: History::new(), period, multiplier, last_dir: 0 }
    }
}
impl Strategy for SupertrendStrategy {
    fn name(&self) -> &str { "supertrend" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("period".into(), self.period as f64);
        m.insert("multiplier".into(), self.multiplier);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.period + 1 { return hold(bar, "等待预热"); }
        let st = crate::indicators::trend::supertrend(&self.history.bars, self.period, self.multiplier);
        let n = self.history.bars.len();
        let dir = match st.direction[n - 1] { Some(d) => d, None => return hold(bar, "ST 未就绪") };
        let trend = st.trend[n - 1].unwrap();
        let prev = self.last_dir; self.last_dir = dir;
        if prev == -1 && dir == 1 { return make_signal(bar, Side::Buy, 1.0, format!("Supertrend 翻多 线={:.2}", trend)); }
        if prev == 1 && dir == -1 { return make_signal(bar, Side::Sell, 1.0, format!("Supertrend 翻空 线={:.2}", trend)); }
        hold(bar, &format!("ST 趋势中 dir={}", dir))
    }
    fn reset(&mut self) { self.history.reset(); self.last_dir = 0; }
}

// ============================================================================
// DonchianBreakoutStrategy
// ============================================================================

pub struct DonchianBreakoutStrategy {
    history: History, entry_period: usize, exit_period: usize,
}
impl DonchianBreakoutStrategy {
    pub fn new(entry_period: usize, exit_period: usize) -> Self {
        Self { history: History::new(), entry_period, exit_period }
    }
}
impl Strategy for DonchianBreakoutStrategy {
    fn name(&self) -> &str { "donchian_breakout" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("entry_period".into(), self.entry_period as f64);
        m.insert("exit_period".into(), self.exit_period as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.entry_period + 1 { return hold(bar, "等待预热"); }
        let dc = crate::indicators::trend::donchian(&self.history.bars, self.entry_period);
        let n = self.history.bars.len();
        let upper = dc.upper[n - 1].unwrap();
        let lower = dc.lower[n - 1].unwrap();
        if bar.close > upper {
            return make_signal(bar, Side::Buy, 1.0, format!("海龟突破 上轨={:.2}", upper));
        }
        if bar.close < lower {
            return make_signal(bar, Side::Sell, 1.0, format!("海龟跌破 下轨={:.2}", lower));
        }
        hold(bar, "价格在海龟通道内")
    }
    fn reset(&mut self) { self.history.reset(); }
}

// ============================================================================
// VwapReversionStrategy
// ============================================================================

pub struct VwapReversionStrategy {
    history: History, period: usize, threshold_pct: f64,
}
impl VwapReversionStrategy {
    pub fn new(period: usize, threshold_pct: f64) -> Self {
        Self { history: History::new(), period, threshold_pct }
    }
}
impl Strategy for VwapReversionStrategy {
    fn name(&self) -> &str { "vwap_reversion" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("period".into(), self.period as f64);
        m.insert("threshold_pct".into(), self.threshold_pct);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.period { return hold(bar, "等待预热"); }
        let v = crate::indicators::volume::vwap(&self.history.bars);
        let n = self.history.bars.len();
        let vwap_v = v[n - 1].unwrap();
        let dev = (bar.close - vwap_v) / vwap_v * 100.0;
        if dev < -self.threshold_pct {
            return make_signal(bar, Side::Buy, 0.7, format!("价偏 VWAP {:.2}%, 做多回归", dev));
        }
        if dev > self.threshold_pct {
            return make_signal(bar, Side::Sell, 0.7, format!("价偏 VWAP +{:.2}%, 做空回归", dev));
        }
        hold(bar, &format!("近 VWAP dev={:.2}%", dev))
    }
    fn reset(&mut self) { self.history.reset(); }
}

// ============================================================================
// KdjStrategy
// ============================================================================

pub struct KdjStrategy {
    history: History, n: usize, m1: usize, m2: usize, prev_j: Option<f64>,
}
impl KdjStrategy {
    pub fn new(n: usize, m1: usize, m2: usize) -> Self {
        Self { history: History::new(), n, m1, m2, prev_j: None }
    }
}
impl Strategy for KdjStrategy {
    fn name(&self) -> &str { "kdj" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("n".into(), self.n as f64);
        m.insert("m1".into(), self.m1 as f64);
        m.insert("m2".into(), self.m2 as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.n + 1 { return hold(bar, "等待预热"); }
        let kdj = crate::indicators::momentum::kdj(&self.history.bars, self.n, self.m1, self.m2);
        let n = self.history.bars.len();
        let j = match kdj.j[n - 1] { Some(v) => v, None => return hold(bar, "KDJ 未就绪") };
        let k = kdj.k[n - 1].unwrap();
        let d = kdj.d[n - 1].unwrap();
        let prev = self.prev_j; self.prev_j = Some(j);
        if let Some(p) = prev {
            if p < 0.0 && j >= 0.0 {
                return make_signal(bar, Side::Buy, 0.9, format!("KDJ 金叉 J 从 {:.1} 反弹 (K={:.1} D={:.1})", p, k, d));
            }
            if p > 100.0 && j <= 100.0 {
                return make_signal(bar, Side::Sell, 0.9, format!("KDJ 死叉 J 从 {:.1} 回落 (K={:.1} D={:.1})", p, k, d));
            }
        }
        hold(bar, &format!("KDJ J={:.1} K={:.1} D={:.1}", j, k, d))
    }
    fn reset(&mut self) { self.history.reset(); self.prev_j = None; }
}

// ============================================================================
// IchimokuStrategy —— 一目均衡表
// ============================================================================

pub struct IchimokuStrategy {
    history: History,
    tenkan_p: usize, kijun_p: usize, senkou_b_p: usize, displacement: usize,
}
impl IchimokuStrategy {
    pub fn new(tenkan_p: usize, kijun_p: usize, senkou_b_p: usize, displacement: usize) -> Self {
        Self { history: History::new(), tenkan_p, kijun_p, senkou_b_p, displacement }
    }
    fn min_history(&self) -> usize { self.senkou_b_p + self.displacement }
}
impl Strategy for IchimokuStrategy {
    fn name(&self) -> &str { "ichimoku" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("tenkan".into(), self.tenkan_p as f64);
        m.insert("kijun".into(), self.kijun_p as f64);
        m.insert("senkou_b".into(), self.senkou_b_p as f64);
        m.insert("displacement".into(), self.displacement as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.min_history() { return hold(bar, "等待预热"); }
        let ich = crate::indicators::trend::ichimoku(
            &self.history.bars, self.tenkan_p, self.kijun_p, self.senkou_b_p, self.displacement);
        let n = self.history.bars.len();
        if let (Some(sa), Some(sb), Some(t), Some(k)) = (ich.senkou_a[n - 1], ich.senkou_b[n - 1], ich.tenkan[n - 1], ich.kijun[n - 1]) {
            let top = sa.max(sb); let bot = sa.min(sb);
            if bar.close > top && t > k {
                return make_signal(bar, Side::Buy, 0.9, format!("云上多头 c={:.2} > 云上{:.2}, 转{}>基{}", bar.close, top, format_num(t), format_num(k)));
            }
            if bar.close < bot && t < k {
                return make_signal(bar, Side::Sell, 0.9, format!("云下空头 c={:.2} < 云下{:.2}, 转{}<基{}", bar.close, bot, format_num(t), format_num(k)));
            }
        }
        hold(bar, "云内/方向不明")
    }
    fn reset(&mut self) { self.history.reset(); }
}

// ============================================================================
// PpoStrategy
// ============================================================================

pub struct PpoStrategy {
    history: History, fast: usize, slow: usize, signal: usize, prev_hist: Option<f64>,
}
impl PpoStrategy {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self { history: History::new(), fast, slow, signal, prev_hist: None }
    }
    fn min_history(&self) -> usize { self.slow + self.signal }
}
impl Strategy for PpoStrategy {
    fn name(&self) -> &str { "ppo" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("fast".into(), self.fast as f64);
        m.insert("slow".into(), self.slow as f64);
        m.insert("signal".into(), self.signal as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.min_history() { return hold(bar, "等待预热"); }
        let closes = self.history.closes(None);
        let n = closes.len();
        let ema_f = crate::indicators::ma::ema(&closes, self.fast);
        let ema_s = crate::indicators::ma::ema(&closes, self.slow);
        let ppo: Vec<f64> = (0..n).map(|i| match (ema_f[i], ema_s[i]) {
            (Some(f), Some(s)) if s != 0.0 => (f - s) / s * 100.0, _ => 0.0 }).collect();
        let signal_line = crate::indicators::ma::ema(&ppo, self.signal);
        let hist = ppo[n - 1] - signal_line[n - 1].unwrap_or(0.0);
        let prev = self.prev_hist; self.prev_hist = Some(hist);
        if let Some(p) = prev {
            if p <= 0.0 && hist > 0.0 {
                return make_signal(bar, Side::Buy, 0.7, format!("PPO 上穿信号线 (hist={:.3})", hist));
            }
            if p >= 0.0 && hist < 0.0 {
                return make_signal(bar, Side::Sell, 0.7, format!("PPO 下穿信号线 (hist={:.3})", hist));
            }
        }
        hold(bar, &format!("PPO hist={:.3}", hist))
    }
    fn reset(&mut self) { self.history.reset(); self.prev_hist = None; }
}

// ============================================================================
// VortexStrategy
// ============================================================================

pub struct VortexStrategy {
    history: History, period: usize, prev_diff: Option<f64>,
}
impl VortexStrategy {
    pub fn new(period: usize) -> Self { Self { history: History::new(), period, prev_diff: None } }
    fn min_history(&self) -> usize { self.period + 1 }
}
impl Strategy for VortexStrategy {
    fn name(&self) -> &str { "vortex" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("period".into(), self.period as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.min_history() { return hold(bar, "等待预热"); }
        let v = crate::indicators::trend::vortex(&self.history.bars, self.period);
        let n = self.history.bars.len();
        let plus = match v.plus[n - 1] { Some(v) => v, None => return hold(bar, "Vortex 未就绪") };
        let minus = match v.minus[n - 1] { Some(v) => v, None => return hold(bar, "Vortex 未就绪") };
        let diff = plus - minus;
        let prev = self.prev_diff; self.prev_diff = Some(diff);
        if let Some(p) = prev {
            if p <= 0.0 && diff > 0.0 {
                return make_signal(bar, Side::Buy, 0.8, format!("VI+({:.3}) 上穿 VI-({:.3})", plus, minus));
            }
            if p >= 0.0 && diff < 0.0 {
                return make_signal(bar, Side::Sell, 0.8, format!("VI+({:.3}) 下穿 VI-({:.3})", plus, minus));
            }
        }
        hold(bar, "Vortex 无交叉")
    }
    fn reset(&mut self) { self.history.reset(); self.prev_diff = None; }
}

// ============================================================================
// ElderRayStrategy
// ============================================================================

pub struct ElderRayStrategy { history: History, period: usize }
impl ElderRayStrategy {
    pub fn new(period: usize) -> Self { Self { history: History::new(), period } }
    fn min_history(&self) -> usize { self.period + 1 }
}
impl Strategy for ElderRayStrategy {
    fn name(&self) -> &str { "elder_ray" }
    fn params(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("period".into(), self.period as f64);
        m
    }
    fn on_bar(&mut self, bar: &Bar) -> Signal {
        self.history.push(bar);
        if self.history.bars.len() < self.min_history() { return hold(bar, "等待预热"); }
        let closes = self.history.closes(None);
        let ema = crate::indicators::ma::ema(&closes, self.period);
        let n = closes.len();
        let e = ema[n - 1].unwrap();
        let bull = bar.high - e;
        let bear = bar.low - e;
        if bull > 0.0 && bear < 0.0 {
            return make_signal(bar, Side::Buy, 0.6, format!("Elder 多头 bull={:.2} bear={:.2}", bull, bear));
        }
        if bull < 0.0 && bear > 0.0 {
            return make_signal(bar, Side::Sell, 0.6, format!("Elder 空头 bull={:.2} bear={:.2}", bull, bear));
        }
        hold(bar, "Elder 力量均衡")
    }
    fn reset(&mut self) { self.history.reset(); }
}

fn format_num(v: f64) -> String { format!("{:.2}", v) }

// ============================================================================
// StrategyKind 枚举 + 工厂
// ============================================================================

pub enum StrategyKind {
    BuyAndHold,
    SmaCross { fast: usize, slow: usize },
    Rsi { period: usize, overbought: f64, oversold: f64 },
    Random { seed: u64, buy_prob: f64, sell_prob: f64 },
    Macd { fast: usize, slow: usize, signal: usize },
    Bollinger { period: usize, num_std: f64 },
    Supertrend { period: usize, multiplier: f64 },
    DonchianBreakout { entry_period: usize, exit_period: usize },
    VwapReversion { period: usize, threshold_pct: f64 },
    Kdj { n: usize, m1: usize, m2: usize },
    Ichimoku { tenkan: usize, kijun: usize, senkou_b: usize, displacement: usize },
    Ppo { fast: usize, slow: usize, signal: usize },
    Vortex { period: usize },
    ElderRay { period: usize },
}

pub fn create_strategy(kind: StrategyKind) -> Box<dyn Strategy> {
    match kind {
        StrategyKind::BuyAndHold => Box::new(BuyAndHoldStrategy::new()),
        StrategyKind::SmaCross { fast, slow } => Box::new(SmaCrossStrategy::new(fast, slow)),
        StrategyKind::Rsi { period, overbought, oversold } =>
            Box::new(RsiStrategy::new(period, overbought, oversold)),
        StrategyKind::Random { seed, buy_prob, sell_prob } =>
            Box::new(RandomStrategy::new(seed, buy_prob, sell_prob)),
        StrategyKind::Macd { fast, slow, signal } => Box::new(MacdStrategy::new(fast, slow, signal)),
        StrategyKind::Bollinger { period, num_std } => Box::new(BollingerBandsStrategy::new(period, num_std)),
        StrategyKind::Supertrend { period, multiplier } => Box::new(SupertrendStrategy::new(period, multiplier)),
        StrategyKind::DonchianBreakout { entry_period, exit_period } =>
            Box::new(DonchianBreakoutStrategy::new(entry_period, exit_period)),
        StrategyKind::VwapReversion { period, threshold_pct } =>
            Box::new(VwapReversionStrategy::new(period, threshold_pct)),
        StrategyKind::Kdj { n, m1, m2 } => Box::new(KdjStrategy::new(n, m1, m2)),
        StrategyKind::Ichimoku { tenkan, kijun, senkou_b, displacement } =>
            Box::new(IchimokuStrategy::new(tenkan, kijun, senkou_b, displacement)),
        StrategyKind::Ppo { fast, slow, signal } => Box::new(PpoStrategy::new(fast, slow, signal)),
        StrategyKind::Vortex { period } => Box::new(VortexStrategy::new(period)),
        StrategyKind::ElderRay { period } => Box::new(ElderRayStrategy::new(period)),
    }
}