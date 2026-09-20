//! REST API + WebSocket handlers。
//!
//! 端点:
//!   GET  /                      → 静态首页
//!   GET  /static/*              → CSS / JS / 图片
//!   GET  /api/config            → 当前配置
//!   GET  /api/strategies        → 可用策略列表
//!   POST /api/backtest          → 跑回测,返回结果 JSON
//!   GET  /api/paper/snapshot    → 模拟盘当前快照
//!   POST /api/paper/start       → 启动模拟盘
//!   POST /api/paper/stop        → 停止模拟盘
//!   POST /api/paper/strategy    → 切换策略
//!   GET  /api/paper/ws          → WebSocket:实时推送模拟盘状态

use crate::app_state::AppState;
use crate::data::{AsyncDataFeed, DataFeed, HttpFeed, SyntheticFeed};
use crate::engine::{BacktestEngine, EngineConfig};
use crate::metrics::compute_metrics;
use crate::paper::PaperSnapshot;
use crate::risk::RiskConfig;
use crate::strategy::{create_strategy, StrategyKind};
use crate::types::{Bar, Side};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::{Duration, Utc};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock as TokioRwLock;

// -----------------------------------------------------------------------------
// 路由器
// -----------------------------------------------------------------------------

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/static/*file", get(serve_static))
        .route("/api/config", get(get_config))
        .route("/api/strategies", get(get_strategies))
        .route("/api/data", get(get_data))
        .route("/api/backtest", post(post_backtest))
        .route("/api/paper/snapshot", get(get_paper_snapshot))
        .route("/api/paper/start", post(post_paper_start))
        .route("/api/paper/stop", post(post_paper_stop))
        .route("/api/paper/strategy", post(post_paper_strategy))
        .route("/api/paper/ws", get(ws_paper))
        .route("/api/knowledge", get(get_knowledge))
        .route("/api/patterns", get(get_patterns))
        .route("/api/heikin_ashi", get(get_heikin_ashi))
        .route("/api/indicators", get(get_indicators))
        .route("/api/code_loc", get(get_code_loc))
        .route("/api/symbols", get(get_symbols))
        .with_state(state)
}

// -----------------------------------------------------------------------------
// 静态文件
// -----------------------------------------------------------------------------

async fn serve_index() -> Response {
    match tokio::fs::read("static/index.html").await {
        Ok(content) => (
            StatusCode::OK,
            [
                ("content-type", "text/html; charset=utf-8"),
                ("cache-control", "no-cache, no-store, must-revalidate"),
                ("pragma", "no-cache"),
                ("expires", "0"),
            ],
            content,
        )
            .into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            "index.html not found. Did you run from project root?",
        )
            .into_response(),
    }
}

async fn serve_static(
    axum::extract::Path(file): axum::extract::Path<String>,
) -> Response {
    // 修复: 改用 std::fs::read + 手动拼接
    let mut path = format!("static/{}", file);
    if file.starts_with('/') {
        path = format!("static{}", file);
    }
    let path_buf = PathBuf::from(&path);
    match tokio::fs::read(&path_buf).await {
        Ok(content) => {
            let mime = if file.ends_with(".css") {
                "text/css; charset=utf-8"
            } else if file.ends_with(".js") {
                "application/javascript; charset=utf-8"
            } else if file.ends_with(".html") {
                "text/html; charset=utf-8"
            } else if file.ends_with(".json") {
                "application/json"
            } else if file.ends_with(".svg") {
                "image/svg+xml"
            } else if file.ends_with(".png") {
                "image/png"
            } else {
                "application/octet-stream"
            };
            (StatusCode::OK, [
                ("content-type", mime),
                ("cache-control", "no-cache, no-store, must-revalidate"),
                ("pragma", "no-cache"),
                ("expires", "0"),
            ], content).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

// -----------------------------------------------------------------------------
// GET /api/config
// -----------------------------------------------------------------------------

async fn get_config(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(serde_json::to_value(&state.config).unwrap_or(json!({})))
}

// -----------------------------------------------------------------------------
// GET /api/strategies
// -----------------------------------------------------------------------------

async fn get_strategies() -> Json<Value> {
    Json(json!({
        "strategies": [
            {
                "name": "buy_and_hold",
                "display_name": "买入持有 (基准)",
                "description": "第一根 K 线买入,然后一直持有。用作基准对比。",
                "params": []
            },
            {
                "name": "sma_cross",
                "display_name": "双均线交叉",
                "description": "经典策略:快线上穿慢线买入,下穿卖出。",
                "params": [
                    {"key": "fast", "label": "快线周期", "default": 5, "min": 2, "max": 50},
                    {"key": "slow", "label": "慢线周期", "default": 20, "min": 5, "max": 200}
                ]
            },
            {
                "name": "rsi",
                "display_name": "RSI 超买超卖",
                "description": "RSI 离开超卖区买入,离开超买区卖出。",
                "params": [
                    {"key": "period", "label": "周期", "default": 14, "min": 5, "max": 50},
                    {"key": "overbought", "label": "超买阈值", "default": 70, "min": 50, "max": 90},
                    {"key": "oversold", "label": "超卖阈值", "default": 30, "min": 10, "max": 50}
                ]
            },
            {
                "name": "random",
                "display_name": "随机 (冒烟测试)",
                "description": "随机买卖,用来验证回测引擎跑得通。真用会亏钱 :)",
                "params": []
            },
            {
                "name": "macd",
                "display_name": "MACD 金叉死叉",
                "description": "DIF 上穿 DEA 买入, 下穿卖出。趋势跟随型。",
                "params": [
                    {"key": "fast", "label": "快 EMA", "default": 12, "min": 5, "max": 50},
                    {"key": "slow", "label": "慢 EMA", "default": 26, "min": 10, "max": 100},
                    {"key": "signal", "label": "信号线", "default": 9, "min": 5, "max": 30}
                ]
            },
            {
                "name": "bollinger",
                "display_name": "Bollinger Bands 布林带",
                "description": "价格触及下轨买入(均值回归), 触及上轨卖出。",
                "params": [
                    {"key": "period", "label": "周期", "default": 20, "min": 10, "max": 50},
                    {"key": "num_std", "label": "标准差倍数", "default": 2.0, "min": 1.0, "max": 3.0}
                ]
            },
            {
                "name": "supertrend",
                "display_name": "Supertrend 超级趋势",
                "description": "趋势跟踪指标, 翻多买、翻空卖。",
                "params": [
                    {"key": "period", "label": "ATR 周期", "default": 10, "min": 5, "max": 30},
                    {"key": "multiplier", "label": "倍数", "default": 3.0, "min": 1.0, "max": 5.0}
                ]
            },
            {
                "name": "donchian_breakout",
                "display_name": "Donchian 海龟突破",
                "description": "经典海龟交易法: 突破 N 日高点买入, 跌破卖出。",
                "params": [
                    {"key": "entry_period", "label": "入场周期", "default": 20, "min": 5, "max": 100},
                    {"key": "exit_period", "label": "出场周期", "default": 10, "min": 3, "max": 50}
                ]
            },
            {
                "name": "vwap_reversion",
                "display_name": "VWAP 回归",
                "description": "价格偏离 VWAP 超过阈值时反向开仓。",
                "params": [
                    {"key": "period", "label": "回看周期", "default": 20, "min": 5, "max": 100},
                    {"key": "threshold_pct", "label": "偏离阈值 %", "default": 1.5, "min": 0.5, "max": 10.0}
                ]
            },
            {
                "name": "kdj",
                "display_name": "KDJ 中国市场指标",
                "description": "J 值从超卖区反弹买入, 从超买区回落卖出。",
                "params": [
                    {"key": "n", "label": "RSV 周期", "default": 9, "min": 5, "max": 30},
                    {"key": "m1", "label": "K 平滑", "default": 3, "min": 2, "max": 10},
                    {"key": "m2", "label": "D 平滑", "default": 3, "min": 2, "max": 10}
                ]
            },
            {
                "name": "ichimoku",
                "display_name": "一目均衡表 Ichimoku",
                "description": "云上 + 转换线 > 基准线买入; 云下卖出。完整云带、转换、基准、迟行。",
                "params": [
                    {"key": "tenkan", "label": "转换线周期", "default": 9, "min": 5, "max": 30},
                    {"key": "kijun", "label": "基准线周期", "default": 26, "min": 10, "max": 60},
                    {"key": "senkou_b", "label": "先行带B周期", "default": 52, "min": 20, "max": 120},
                    {"key": "displacement", "label": "位移", "default": 26, "min": 10, "max": 60}
                ]
            },
            {
                "name": "ppo",
                "display_name": "PPO 百分比价格振荡器",
                "description": "类似 MACD 但用百分比,跨品种可比。柱体上穿 0 买入。",
                "params": [
                    {"key": "fast", "label": "快 EMA", "default": 12, "min": 5, "max": 50},
                    {"key": "slow", "label": "慢 EMA", "default": 26, "min": 10, "max": 100},
                    {"key": "signal", "label": "信号线", "default": 9, "min": 5, "max": 30}
                ]
            },
            {
                "name": "vortex",
                "display_name": "Vortex 涡旋指标",
                "description": "VI+ 上穿 VI- 买入, 下穿卖出。趋势强度直观。",
                "params": [
                    {"key": "period", "label": "周期", "default": 14, "min": 5, "max": 50}
                ]
            },
            {
                "name": "elder_ray",
                "display_name": "Elder Ray 多空力量",
                "description": "Bull Power > 0 且 Bear Power < 0 → 多头占优。Dr. Elder 的多空力量分离指标。",
                "params": [
                    {"key": "period", "label": "EMA 周期", "default": 13, "min": 5, "max": 50}
                ]
            }
        ]
    }))
}

// -----------------------------------------------------------------------------
// GET /api/data
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct DataQuery {
    symbol: Option<String>,
    limit: Option<usize>,
    source: Option<String>, // "real" / "synthetic"
    chart: Option<String>,   // "candle" (默认) / "heikin_ashi"
}

async fn get_data(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<DataQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let symbol = q.symbol.unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(200).min(2000);
    let source = q.source.unwrap_or_else(|| "real".to_string());

    let since = Utc::now() - Duration::days(state.config.backtest.lookback_days as i64);

    let bars: Vec<Bar> = if source == "synthetic" {
        let feed = SyntheticFeed::default();
        feed.fetch_historical(&symbol, since, limit)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        let feed = HttpFeed::new(state.data_cache_dir.clone());
        feed.fetch_historical_async(&symbol, since, limit).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    let json_bars: Vec<Value> = bars
        .iter()
        .map(|b| json!({
            "timestamp": b.timestamp.to_rfc3339(),
            "open": b.open,
            "high": b.high,
            "low": b.low,
            "close": b.close,
            "volume": b.volume,
            "chart_type": "candle",
        }))
        .collect();

    Ok(Json(json!({
        "symbol": symbol,
        "count": bars.len(),
        "bars": json_bars,
        "source": source,
    })))
}

// -----------------------------------------------------------------------------
// POST /api/backtest
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct BacktestRequest {
    strategy: String,
    params: Option<std::collections::HashMap<String, f64>>,
    symbol: Option<String>,
    source: Option<String>, // "real" / "synthetic"
    limit: Option<usize>,
    initial_capital: Option<f64>,
    commission_rate: Option<f64>,
    slippage_rate: Option<f64>,
    stop_loss_pct: Option<f64>,
    take_profit_pct: Option<f64>,
    max_position_pct: Option<f64>,
}

async fn post_backtest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BacktestRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // 1. 构造策略
    let mut strategy = make_strategy(&req.strategy, req.params.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // 2. 拉数据
    let symbol = req.symbol.clone()
        .unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = req.limit.unwrap_or(500).min(5000);
    let source = req.source.unwrap_or_else(|| "real".to_string());

    let since = Utc::now() - Duration::days(state.config.backtest.lookback_days as i64);
    let bars = if source == "synthetic" {
        SyntheticFeed::default()
            .fetch_historical(&symbol, since, limit)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        let feed = HttpFeed::new(state.data_cache_dir.clone());
        feed.fetch_historical_async(&symbol, since, limit).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    if bars.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "没有获取到任何 K 线".into()));
    }

    // 3. 配置引擎
    let engine_cfg = EngineConfig {
        symbol: symbol.clone(),
        initial_capital: req.initial_capital.unwrap_or(state.config.trading.initial_capital),
        commission_rate: req.commission_rate.unwrap_or(state.config.trading.commission_rate),
        slippage_rate: req.slippage_rate.unwrap_or(state.config.trading.slippage_rate),
    };
    let risk_cfg = RiskConfig {
        stop_loss_pct: req.stop_loss_pct.unwrap_or(state.config.risk.stop_loss_pct),
        take_profit_pct: req.take_profit_pct.unwrap_or(state.config.risk.take_profit_pct),
        max_position_pct: req.max_position_pct.unwrap_or(state.config.risk.max_position_pct),
    };

    // 4. 跑回测
    let engine = BacktestEngine::new(engine_cfg.clone(), risk_cfg);
    let mut result = engine.run(strategy.as_mut(), &bars);
    result.metrics = compute_metrics(&result);

    // 5. 序列化成前端友好格式
    let equity_curve: Vec<Value> = result.equity_curve.iter().map(|p| json!({
        "timestamp": p.timestamp.to_rfc3339(),
        "cash": p.cash,
        "position_value": p.position_value,
        "equity": p.equity,
    })).collect();

    let price_series: Vec<Value> = result.equity_curve.iter().map(|p| json!({
        "timestamp": p.timestamp.to_rfc3339(),
        "price": p.equity - p.cash + p.position_value, // 当前价 = (equity - cash)/size 近似
    })).collect();
    let _ = price_series; // 暂未直接使用

    let trades: Vec<Value> = result.trades.iter().map(|t| json!({
        "symbol": t.symbol,
        "side": format!("{:?}", t.side).to_uppercase(),
        "entry_time": t.entry_time.to_rfc3339(),
        "exit_time": t.exit_time.map(|x| x.to_rfc3339()),
        "entry_price": t.entry_price,
        "exit_price": t.exit_price,
        "size": t.size,
        "pnl": t.pnl(),
        "pnl_pct": t.pnl_pct() * 100.0,
        "commission": t.total_commission(),
    })).collect();

    let signals: Vec<Value> = result.signals.iter().enumerate().filter_map(|(i, s)| {
        if s.side != Side::Hold {
            Some(json!({
                "i": i,
                "timestamp": s.timestamp.to_rfc3339(),
                "side": format!("{:?}", s.side).to_uppercase(),
                "strength": s.strength,
                "reason": s.reason,
            }))
        } else {
            None
        }
    }).collect();

    let fills: Vec<Value> = result.fills.iter().map(|f| json!({
        "timestamp": f.timestamp.to_rfc3339(),
        "side": format!("{:?}", f.side).to_uppercase(),
        "size": f.size,
        "price": f.price,
        "commission": f.commission,
    })).collect();

    Ok(Json(json!({
        "config": result.config,
        "metrics": result.metrics,
        "equity_curve": equity_curve,
        "trades": trades,
        "signals": signals,
        "fills": fills,
    })))
}

fn make_strategy(
    name: &str,
    params: Option<&std::collections::HashMap<String, f64>>,
) -> anyhow::Result<Box<dyn crate::strategy::Strategy>> {
    let p = params.cloned().unwrap_or_default();
    let kind = match name {
        "buy_and_hold" => StrategyKind::BuyAndHold,
        "sma_cross" => {
            let fast = p.get("fast").copied().unwrap_or(5.0) as usize;
            let slow = p.get("slow").copied().unwrap_or(20.0) as usize;
            StrategyKind::SmaCross { fast, slow }
        }
        "rsi" => {
            let period = p.get("period").copied().unwrap_or(14.0) as usize;
            let overbought = p.get("overbought").copied().unwrap_or(70.0);
            let oversold = p.get("oversold").copied().unwrap_or(30.0);
            StrategyKind::Rsi { period, overbought, oversold }
        }
        "random" => {
            let buy_prob = p.get("buy_prob").copied().unwrap_or(0.05);
            let sell_prob = p.get("sell_prob").copied().unwrap_or(0.05);
            StrategyKind::Random { seed: 42, buy_prob, sell_prob }
        }
        "macd" => {
            let fast = p.get("fast").copied().unwrap_or(12.0) as usize;
            let slow = p.get("slow").copied().unwrap_or(26.0) as usize;
            let signal = p.get("signal").copied().unwrap_or(9.0) as usize;
            StrategyKind::Macd { fast, slow, signal }
        }
        "bollinger" => {
            let period = p.get("period").copied().unwrap_or(20.0) as usize;
            let num_std = p.get("num_std").copied().unwrap_or(2.0);
            StrategyKind::Bollinger { period, num_std }
        }
        "supertrend" => {
            let period = p.get("period").copied().unwrap_or(10.0) as usize;
            let multiplier = p.get("multiplier").copied().unwrap_or(3.0);
            StrategyKind::Supertrend { period, multiplier }
        }
        "donchian_breakout" => {
            let entry_period = p.get("entry_period").copied().unwrap_or(20.0) as usize;
            let exit_period = p.get("exit_period").copied().unwrap_or(10.0) as usize;
            StrategyKind::DonchianBreakout { entry_period, exit_period }
        }
        "vwap_reversion" => {
            let period = p.get("period").copied().unwrap_or(20.0) as usize;
            let threshold_pct = p.get("threshold_pct").copied().unwrap_or(1.5);
            StrategyKind::VwapReversion { period, threshold_pct }
        }
        "kdj" => {
            let n = p.get("n").copied().unwrap_or(9.0) as usize;
            let m1 = p.get("m1").copied().unwrap_or(3.0) as usize;
            let m2 = p.get("m2").copied().unwrap_or(3.0) as usize;
            StrategyKind::Kdj { n, m1, m2 }
        }
        "ichimoku" => {
            let tenkan = p.get("tenkan").copied().unwrap_or(9.0) as usize;
            let kijun = p.get("kijun").copied().unwrap_or(26.0) as usize;
            let senkou_b = p.get("senkou_b").copied().unwrap_or(52.0) as usize;
            let displacement = p.get("displacement").copied().unwrap_or(26.0) as usize;
            StrategyKind::Ichimoku { tenkan, kijun, senkou_b, displacement }
        }
        "ppo" => {
            let fast = p.get("fast").copied().unwrap_or(12.0) as usize;
            let slow = p.get("slow").copied().unwrap_or(26.0) as usize;
            let signal = p.get("signal").copied().unwrap_or(9.0) as usize;
            StrategyKind::Ppo { fast, slow, signal }
        }
        "vortex" => {
            let period = p.get("period").copied().unwrap_or(14.0) as usize;
            StrategyKind::Vortex { period }
        }
        "elder_ray" => {
            let period = p.get("period").copied().unwrap_or(13.0) as usize;
            StrategyKind::ElderRay { period }
        }
        _ => anyhow::bail!("未知策略: {}", name),
    };
    Ok(create_strategy(kind))
}

// -----------------------------------------------------------------------------
// 模拟盘 API
// -----------------------------------------------------------------------------

async fn get_paper_snapshot(
    State(state): State<Arc<AppState>>,
) -> Json<PaperSnapshot> {
    let s = state.paper_state.read().await;
    Json(s.snapshot())
}

async fn post_paper_start(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let mut s = state.paper_state.write().await;
    s.is_running = true;
    s.log(crate::paper::PaperLogLevel::Info, "模拟盘已启动".into());
    Json(json!({"status": "started"}))
}

async fn post_paper_stop(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let mut s = state.paper_state.write().await;
    s.is_running = false;
    s.log(crate::paper::PaperLogLevel::Info, "模拟盘已停止".into());
    Json(json!({"status": "stopped"}))
}

#[derive(Deserialize)]
struct PaperStrategyRequest {
    strategy: String,
    params: Option<std::collections::HashMap<String, f64>>,
}

async fn post_paper_strategy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PaperStrategyRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let strategy = make_strategy(&req.strategy, req.params.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let mut s = state.paper_state.write().await;
    s.strategy = strategy;
    s.log(crate::paper::PaperLogLevel::Info,
          format!("策略已切换为: {}", req.strategy));
    Ok(Json(json!({"status": "ok", "strategy": req.strategy})))
}

// -----------------------------------------------------------------------------
// K线形态识别 + Heikin Ashi 端点
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct PatternsQuery {
    symbol: Option<String>,
    limit: Option<usize>,
    source: Option<String>,
}

async fn get_patterns(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<PatternsQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let symbol = q.symbol.unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(100).min(500);
    let source = q.source.unwrap_or_else(|| "real".to_string());
    let since = Utc::now() - Duration::days(30);

    let bars: Vec<Bar> = if source == "synthetic" {
        SyntheticFeed::default()
            .fetch_historical(&symbol, since, limit)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        HttpFeed::new(state.data_cache_dir.clone())
            .fetch_historical_async(&symbol, since, limit).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    use crate::indicators::extra::detect_pattern;
    let patterns: Vec<Value> = bars.iter().rev().take(20).map(|bar| {
        let p = detect_pattern(bar);
        json!({
            "timestamp": bar.timestamp.to_rfc3339(),
            "close": bar.close,
            "pattern": p.name_zh(),
            "pattern_code": format!("{:?}", p),
        })
    }).collect();

    Ok(Json(json!({ "symbol": symbol, "patterns": patterns })))
}

async fn get_heikin_ashi(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<PatternsQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let symbol = q.symbol.unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(200).min(1000);
    let source = q.source.unwrap_or_else(|| "real".to_string());
    let since = Utc::now() - Duration::days(state.config.backtest.lookback_days as i64);

    let bars: Vec<Bar> = if source == "synthetic" {
        SyntheticFeed::default()
            .fetch_historical(&symbol, since, limit)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        HttpFeed::new(state.data_cache_dir.clone())
            .fetch_historical_async(&symbol, since, limit).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    let ha = crate::indicators::extra::heikin_ashi(&bars);
    let json_bars: Vec<Value> = ha.iter().map(|b| json!({
        "timestamp": b.timestamp.to_rfc3339(),
        "open": b.open, "high": b.high, "low": b.low, "close": b.close,
    })).collect();

    Ok(Json(json!({ "symbol": symbol, "bars": json_bars, "chart": "heikin_ashi" })))
}

// -----------------------------------------------------------------------------
// 知识库 API —— 返回 PDF 里所有概念的字典
// -----------------------------------------------------------------------------

async fn get_knowledge() -> Json<Value> {
    use crate::knowledge;
    let entries = knowledge::all_entries();
    let total = entries.len();
    // 按 category 分组
    let mut grouped: std::collections::BTreeMap<String, Vec<Value>> = std::collections::BTreeMap::new();
    for e in entries {
        let val = serde_json::to_value(&e).unwrap_or_default();
        grouped.entry(e.category.clone()).or_default().push(val);
    }
    Json(json!({
        "total": total,
        "categories": grouped,
    }))
}

// -----------------------------------------------------------------------------
// 指标计算端点 —— 用于在数据探索页面上叠加指标层
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct IndicatorQuery {
    symbol: Option<String>,
    limit: Option<usize>,
    source: Option<String>,
    indicators: Option<String>, // 逗号分隔, 如 "sma_20,ema_50,rsi_14,bbands_20,macd"
}

async fn get_indicators(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<IndicatorQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let symbol = q.symbol.unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(200).min(2000);
    let source = q.source.unwrap_or_else(|| "real".to_string());
    let since = Utc::now() - Duration::days(state.config.backtest.lookback_days as i64);

    let bars = if source == "synthetic" {
        SyntheticFeed::default()
            .fetch_historical(&symbol, since, limit)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        HttpFeed::new(state.data_cache_dir.clone())
            .fetch_historical_async(&symbol, since, limit)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let requested_str = q.indicators
        .unwrap_or_else(|| "sma_20,ema_50,rsi_14".to_string());
    let requested: Vec<&str> = requested_str.split(',').map(|s| s.trim()).collect();

    let mut series: HashMap<String, Vec<Option<f64>>> = HashMap::new();

    for ind in requested {
        let parts: Vec<&str> = ind.split('_').collect();
        if parts.is_empty() { continue; }
        let name = parts[0];
        let period: usize = parts.get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(match name {
                "sma" | "ema" | "rsi" | "vwma" => 14,
                "bbands" => 20,
                "macd" => 12,
                _ => 14,
            });

        match name {
            "sma" => {
                series.insert(ind.to_string(), crate::indicators::ma::sma(&closes, period));
            }
            "ema" => {
                series.insert(ind.to_string(), crate::indicators::ma::ema(&closes, period));
            }
            "rsi" => {
                series.insert(ind.to_string(), crate::indicators::momentum::rsi(&closes, period));
            }
            "bbands" => {
                let bb = crate::indicators::volatility::bollinger_bands(&closes, period, 2.0);
                series.insert(format!("{}_upper", ind), bb.upper);
                series.insert(format!("{}_middle", ind), bb.middle);
                series.insert(format!("{}_lower", ind), bb.lower);
            }
            "macd" => {
                let m = crate::indicators::trend::macd(&closes, 12, 26, 9);
                series.insert(format!("{}_dif", ind), m.dif);
                series.insert(format!("{}_dea", ind), m.dea);
                series.insert(format!("{}_hist", ind), m.hist);
            }
            "vwap" => {
                series.insert(ind.to_string(), crate::indicators::volume::vwap(&bars));
            }
            "atr" => {
                series.insert(ind.to_string(), crate::indicators::volatility::atr(&bars, period));
            }
            "obv" => {
                series.insert(ind.to_string(), crate::indicators::volume::obv(&bars));
            }
            "zscore" => {
                series.insert(ind.to_string(), crate::indicators::extra::zscore(&closes, 20));
            }
            "ichimoku" => {
                let ich = crate::indicators::trend::ichimoku(&bars, 9, 26, 52, 26);
                series.insert("ichimoku_tenkan".into(), ich.tenkan);
                series.insert("ichimoku_kijun".into(), ich.kijun);
                series.insert("ichimoku_senkou_a".into(), ich.senkou_a);
                series.insert("ichimoku_senkou_b".into(), ich.senkou_b);
                series.insert("ichimoku_chikou".into(), ich.chikou);
            }
            "kdj" => {
                let k = crate::indicators::momentum::kdj(&bars, 9, 3, 3);
                series.insert("kdj_k".into(), k.k);
                series.insert("kdj_d".into(), k.d);
                series.insert("kdj_j".into(), k.j);
            }
            "stoch" => {
                let s = crate::indicators::momentum::stochastic(&bars, 14, 3, 3);
                series.insert("stoch_k".into(), s.k);
                series.insert("stoch_d".into(), s.d);
            }
            "williams_r" => {
                series.insert(ind.to_string(), crate::indicators::momentum::williams_r(&bars, 14));
            }
            "cci" => {
                series.insert(ind.to_string(), crate::indicators::momentum::cci(&bars, 20));
            }
            "adx" => {
                let d = crate::indicators::trend::dmi(&bars, 14);
                series.insert("adx_plus_di".into(), d.plus_di);
                series.insert("adx_minus_di".into(), d.minus_di);
                series.insert("adx_adx".into(), d.adx);
            }
            "bbi" => {
                series.insert(ind.to_string(), crate::indicators::ma::bbi(&bars));
            }
            "alligator" => {
                let r = crate::indicators::ma::rma(&closes, 13);
                let e = crate::indicators::ma::ema(&closes, 8);
                series.insert("alligator_jaw".into(), r);
                series.insert("alligator_teeth".into(), e);
                series.insert("alligator_lips".into(), crate::indicators::ma::ema(&closes, 5));
            }
            "ppo" => {
                let closes_f = closes.clone();
                let n = closes_f.len();
                let ema_f = crate::indicators::ma::ema(&closes_f, 12);
                let ema_s = crate::indicators::ma::ema(&closes_f, 26);
                let ppo: Vec<Option<f64>> = (0..n).map(|i| match (ema_f[i], ema_s[i]) {
                    (Some(f), Some(s)) if s != 0.0 => Some((f - s) / s * 100.0),
                    _ => None,
                }).collect();
                series.insert("ppo".into(), ppo);
            }
            "vortex" => {
                let v = crate::indicators::trend::vortex(&bars, 14);
                series.insert("vortex_plus".into(), v.plus);
                series.insert("vortex_minus".into(), v.minus);
            }
            _ => {}
        }
    }

    // 同时返回原始 K 线
    let json_bars: Vec<Value> = bars.iter().map(|b| json!({
        "timestamp": b.timestamp.to_rfc3339(),
        "open": b.open, "high": b.high, "low": b.low, "close": b.close,
        "volume": b.volume,
    })).collect();

    // 把指标序列转成 (timestamp, value) 对, 便于前端画图
    let indicator_output: HashMap<String, Vec<Option<Value>>> = series.iter()
        .map(|(k, v)| {
            let pairs: Vec<Option<Value>> = v.iter().enumerate().map(|(i, val)| {
                val.map(|x| json!({"x": bars[i].timestamp.to_rfc3339(), "y": x}))
            }).collect();
            (k.clone(), pairs)
        })
        .collect();

    Ok(Json(json!({
        "symbol": symbol,
        "bars": json_bars,
        "indicators": indicator_output,
    })))
}

// -----------------------------------------------------------------------------
// 交易对列表端点 —— 从 Binance 拉所有可交易对
// -----------------------------------------------------------------------------

#[derive(Clone)]
struct SymbolsCache {
    symbols: Vec<String>,
    fetched_at: chrono::DateTime<chrono::Utc>,
}

static SYMBOLS_CACHE: OnceLock<TokioRwLock<Option<SymbolsCache>>> = OnceLock::new();

fn symbols_cache() -> &'static TokioRwLock<Option<SymbolsCache>> {
    SYMBOLS_CACHE.get_or_init(|| TokioRwLock::new(None))
}

#[derive(serde::Deserialize)]
struct BinanceSymbol {
    symbol: String,
    status: String,
    #[serde(rename = "quoteAsset")]
    quote_asset: String,
    #[serde(rename = "baseAsset")]
    base_asset: String,
    #[serde(rename = "isSpotTradingAllowed")]
    is_spot_trading_allowed: Option<bool>,
}

#[derive(serde::Deserialize)]
struct BinanceTicker {
    symbol: String,
    #[serde(rename = "quoteVolume")]
    quote_volume: String,
}

async fn fetch_symbols_from_binance() -> anyhow::Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    // 获取所有交易对
    let exchange_info: serde_json::Value = client
        .get("https://api.binance.com/api/v3/exchangeInfo")
        .send().await?
        .json().await?;
    let raw_symbols: Vec<BinanceSymbol> = serde_json::from_value(
        exchange_info["symbols"].clone()
    )?;
    // 获取 24h 成交量排序
    let tickers: Vec<BinanceTicker> = client
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .send().await?
        .json().await?;
    // 过滤 USDT 现货可交易对,按成交量排序
    let mut pairs: Vec<(String, f64)> = raw_symbols.into_iter()
        .filter(|s| s.status == "TRADING"
                  && s.is_spot_trading_allowed.unwrap_or(false)
                  && s.quote_asset == "USDT")
        .filter_map(|s| {
            let vol = tickers.iter()
                .find(|t| t.symbol == s.symbol)
                .and_then(|t| t.quote_volume.parse::<f64>().ok())
                .unwrap_or(0.0);
            Some((s.symbol, vol))
        })
        .collect();
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(pairs.into_iter().map(|(s, _)| s).collect())
}

async fn get_cached_symbols() -> Vec<String> {
    // 检查缓存 (5 分钟过期)
    {
        let cache = symbols_cache().read().await;
        if let Some(c) = cache.as_ref() {
            let age = (chrono::Utc::now() - c.fetched_at).num_seconds();
            if age < 300 {
                return c.symbols.clone();
            }
        }
    }
    // 重新拉取
    match fetch_symbols_from_binance().await {
        Ok(symbols) => {
            let mut cache = symbols_cache().write().await;
            *cache = Some(SymbolsCache {
                symbols: symbols.clone(),
                fetched_at: chrono::Utc::now(),
            });
            symbols
        }
        Err(e) => {
            tracing::warn!("拉取 Binance 交易对失败: {}; 使用 fallback", e);
            // Fallback: 常见 30 个 USDT 交易对
            default_symbols()
        }
    }
}

fn default_symbols() -> Vec<String> {
    vec![
        "BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT",
        "DOGEUSDT", "ADAUSDT", "AVAXUSDT", "MATICUSDT", "DOTUSDT",
        "LINKUSDT", "TRXUSDT", "LTCUSDT", "BCHUSDT", "ATOMUSDT",
        "NEARUSDT", "APTUSDT", "OPUSDT", "ARBUSDT", "INJUSDT",
        "SUIUSDT", "SEIUSDT", "TIAUSDT", "WLDUSDT", "PEPEUSDT",
        "SHIBUSDT", "FILUSDT", "ICPUSDT", "STXUSDT", "RNDRUSDT",
    ].into_iter().map(String::from).collect()
}

async fn get_symbols() -> Json<Value> {
    let symbols = get_cached_symbols().await;
    Json(json!({
        "symbols": symbols,
        "count": symbols.len(),
        "source": if symbols.len() > 30 { "binance" } else { "fallback" },
    }))
}

// -----------------------------------------------------------------------------
// WebSocket:实时推送模拟盘状态
// -----------------------------------------------------------------------------

async fn ws_paper(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| ws_paper_loop(socket, state))
}

async fn ws_paper_loop(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        interval.tick().await;
        let snapshot = {
            let s = state.paper_state.read().await;
            s.snapshot()
        };
        let msg = serde_json::to_string(&snapshot).unwrap_or_default();
        if sender.send(Message::Text(msg)).await.is_err() {
            break;
        }
        // 监听客户端关闭
        if let Some(Ok(Message::Close(_))) = receiver.next().await {
            break;
        }
    }
}
// -----------------------------------------------------------------------------
// GET /api/code_loc?ref=src/path.rs::function
// 返回符号当前所在行号 (行级跳转)
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct CodeLocQuery {
    ref_: Option<String>,
}

async fn get_code_loc(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let raw = q.get("ref").cloned().unwrap_or_default();
    // 不允许任意路径 — 必须是 src/ 开头
    if !raw.starts_with("src/") {
        return Json(json!({
            "ok": false,
            "error": "ref 必须以 src/ 开头",
            "line": null,
            "url": null,
        }));
    }
    // 解析 path::symbol
    let parts: Vec<&str> = raw.split("::").collect();
    if parts.len() < 2 {
        return Json(json!({
            "ok": false,
            "error": "格式必须是 src/path.rs::symbol",
            "line": null,
            "url": null,
        }));
    }
    let path = parts[0];
    let sym = parts[1];
    let line = locate_symbol(path, sym);
    let url = match line {
        Some(n) => json!({
            "ok": true,
            "ref": raw,
            "path": path,
            "symbol": sym,
            "line": n,
            "url": format!("https://github.com/Sigma711/axiom/blob/main/{}#L{}", path, n),
        }),
        None => json!({
            "ok": false,
            "ref": raw,
            "path": path,
            "symbol": sym,
            "line": null,
            "url": null,
            "error": format!("符号 {} 不存在于 {}", sym, path),
        }),
    };
    Json(url)
}

/// 在文件中定位符号 (fn / struct / enum) 的行号
fn locate_symbol(path: &str, sym: &str) -> Option<usize> {
    let content = std::fs::read_to_string(path).ok()?;
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // 匹配 pub fn / fn / pub struct / struct / pub enum / enum / pub const / const
        let patterns = [
            format!("pub fn {} ", sym),
            format!("pub fn {}(", sym),
            format!("pub fn {}<", sym),
            format!("pub fn {}\t", sym),
            format!("fn {} ", sym),
            format!("fn {}(", sym),
            format!("pub struct {} ", sym),
            format!("pub struct {}{{", sym),
            format!("struct {} ", sym),
            format!("pub enum {} ", sym),
            format!("enum {} ", sym),
            format!("pub const {} ", sym),
            format!("const {} ", sym),
        ];
        if patterns.iter().any(|p| trimmed.starts_with(p)) {
            return Some(i + 1);
        }
    }
    None
}


/// 测试辅助: 解析 "src/path::sym" 格式
pub fn locate_symbol_for_test(ref_str: &str) -> Option<usize> {
    let parts: Vec<&str> = ref_str.split("::").collect();
    if parts.len() < 2 { return None; }
    locate_symbol(parts[0], parts[1])
}
