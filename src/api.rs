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

use crate::api_validation::{self as validate, ApiError};
use crate::app_state::AppState;
use crate::data::{fetch_public_market_bars, DataFeed, SyntheticFeed};
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
use chrono::{Duration, TimeZone, Timelike, Utc};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
fn source_candle_close_after(source: &str) -> Result<Duration, ApiError> {
    match source {
        "real" | "binance" | "synthetic" => Ok(Duration::hours(1)),
        "a_share" => Ok(Duration::hours(8)),
        "us_stock" => Ok(Duration::hours(22)),
        _ => Err(validate::bad("unsupported market source")),
    }
}

use std::sync::OnceLock;
use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock};

async fn market_bars(
    state: &AppState,
    symbol: &str,
    source: &str,
    limit: usize,
) -> Result<Vec<Bar>, ApiError> {
    validate::market(symbol, source, limit, 5000)?;
    // The real alias is accepted only for old deep links; new callers use the explicit provider name.
    let source = if source == "real" { "binance" } else { source };
    if std::env::var("AXIOM_OFFLINE").as_deref() == Ok("1") {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Live market data is disabled in offline mode".into(),
        ));
    }
    let requested = if source == "synthetic" {
        limit
    } else {
        limit.saturating_add(1)
    };
    let now = Utc::now()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let since = if source == "synthetic" {
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
    } else if source == "binance" {
        now - Duration::hours(requested as i64)
    } else {
        // Daily markets need calendar runway: it includes weekends and holidays.
        now - Duration::days((requested.saturating_mul(3)) as i64)
    };
    let bars = if source == "synthetic" {
        SyntheticFeed::default().fetch_historical(symbol, since, requested)
    } else {
        fetch_public_market_bars(&state.feed, source, symbol, since, requested).await
    }
    .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    completed_market_bars(bars, source, Utc::now(), limit)
}

fn completed_market_bars(
    mut bars: Vec<Bar>,
    source: &str,
    now: chrono::DateTime<Utc>,
    limit: usize,
) -> Result<Vec<Bar>, ApiError> {
    let close_after = source_candle_close_after(source)?;
    bars.retain(|bar| bar.timestamp + close_after <= now);
    if bars.len() > limit {
        bars = bars.split_off(bars.len() - limit);
    }
    if bars.is_empty() {
        return Err(validate::bad(
            "market request returned no completed candles",
        ));
    }
    validate::bars(&bars)?;
    Ok(bars)
}

// -----------------------------------------------------------------------------
// 路由器
// -----------------------------------------------------------------------------

// SPA routes are registered below with the application router.
// /learn/book
// /data
// /backtest
// /paper
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/learn", get(serve_index))
        .route("/learn/book", get(serve_index))
        .route("/learn/concepts", get(serve_index))
        .route("/learn/build", get(serve_index))
        .route("/learn/path", get(serve_index))
        .route("/data", get(serve_index))
        .route("/backtest", get(serve_index))
        .route("/paper", get(serve_index))
        .route("/compare", get(serve_index))
        .route("/static/*file", get(serve_static))
        .nest_service(
            "/assets",
            tower_http::services::ServeDir::new("static/assets"),
        )
        .route("/api/config", get(get_config))
        .route("/api/strategies", get(get_strategies))
        .route("/api/data", get(get_data))
        .route("/api/backtest", post(post_backtest))
        .route("/api/paper/snapshot", get(get_paper_snapshot))
        .route("/api/paper/start", post(post_paper_start))
        .route("/api/paper/stop", post(post_paper_stop))
        .route("/api/paper/strategy", post(post_paper_strategy))
        .route("/api/paper/config", post(post_paper_config))
        .route("/api/paper/ws", get(ws_paper))
        .route("/api/book/pdf", get(serve_book_pdf))
        .route("/api/knowledge", get(get_knowledge))
        .route("/api/practice", get(get_practice).post(post_practice))
        .route(
            "/api/knowledge/coverage",
            get(|| async { Json(crate::book_sources::coverage()) }),
        )
        .route("/api/patterns", get(get_patterns))
        .route("/api/heikin_ashi", get(get_heikin_ashi))
        .route("/api/indicators", get(get_indicators))
        .route("/api/code_loc", get(get_code_loc))
        .route("/api/code/source", get(get_code_source))
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

async fn serve_book_pdf() -> Response {
    serve_static(axum::extract::Path(
        "book/股票交易软件专业指标全解_完整版.pdf".to_owned(),
    ))
    .await
}

async fn serve_static(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    if std::path::Path::new(&file)
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }
    let path_buf = PathBuf::from("static").join(&file);
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
            } else if file.ends_with(".pdf") {
                "application/pdf"
            } else {
                "application/octet-stream"
            };
            (
                StatusCode::OK,
                [
                    ("content-type", mime),
                    ("cache-control", "no-cache, no-store, must-revalidate"),
                    ("pragma", "no-cache"),
                    ("expires", "0"),
                ],
                content,
            )
                .into_response()
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
}

async fn get_data(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<DataQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let symbol = q
        .symbol
        .unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(200);
    let source = q.source.unwrap_or_else(|| "binance".to_string());

    let bars = market_bars(&state, &symbol, &source, limit).await?;

    let json_bars: Vec<Value> = bars
        .iter()
        .map(|b| {
            json!({
                "timestamp": b.timestamp.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                "open": b.open,
                "high": b.high,
                "low": b.low,
                "close": b.close,
                "volume": b.volume,
            })
        })
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
    bars: Option<Vec<Bar>>,
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
    validate::bounded(
        "initial_capital",
        req.initial_capital
            .unwrap_or(state.config.trading.initial_capital),
        f64::MIN_POSITIVE,
        1e15,
    )?;
    validate::bounded(
        "commission_rate",
        req.commission_rate
            .unwrap_or(state.config.trading.commission_rate),
        0.0,
        0.5,
    )?;
    validate::bounded(
        "slippage_rate",
        req.slippage_rate
            .unwrap_or(state.config.trading.slippage_rate),
        0.0,
        0.5,
    )?;
    validate::bounded(
        "stop_loss_pct",
        req.stop_loss_pct.unwrap_or(state.config.risk.stop_loss_pct),
        0.0,
        1.0,
    )?;
    validate::bounded(
        "take_profit_pct",
        req.take_profit_pct
            .unwrap_or(state.config.risk.take_profit_pct),
        0.0,
        100.0,
    )?;
    validate::bounded(
        "max_position_pct",
        req.max_position_pct
            .unwrap_or(state.config.risk.max_position_pct),
        0.0,
        1.0,
    )?;
    // 1. 构造策略
    let mut strategy = make_strategy(&req.strategy, req.params.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // 2. 拉数据
    let symbol = req
        .symbol
        .clone()
        .unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = req.limit.unwrap_or(500);
    let source = req.source.unwrap_or_else(|| "binance".to_string());

    validate::market(&symbol, &source, limit, 5000)?;
    let bars = if let Some(bars) = req.bars {
        validate::bars(&bars)?;
        bars
    } else {
        market_bars(&state, &symbol, &source, limit).await?
    };

    if bars.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "没有获取到任何 K 线".into()));
    }

    // 3. 配置引擎
    let engine_cfg = EngineConfig {
        symbol: symbol.clone(),
        initial_capital: req
            .initial_capital
            .unwrap_or(state.config.trading.initial_capital),
        commission_rate: req
            .commission_rate
            .unwrap_or(state.config.trading.commission_rate),
        slippage_rate: req
            .slippage_rate
            .unwrap_or(state.config.trading.slippage_rate),
    };
    let risk_cfg = RiskConfig {
        stop_loss_pct: req.stop_loss_pct.unwrap_or(state.config.risk.stop_loss_pct),
        take_profit_pct: req
            .take_profit_pct
            .unwrap_or(state.config.risk.take_profit_pct),
        max_position_pct: req
            .max_position_pct
            .unwrap_or(state.config.risk.max_position_pct),
    };

    // 4. 跑回测
    let engine = BacktestEngine::new(engine_cfg.clone(), risk_cfg);
    let mut result = engine.run(strategy.as_mut(), &bars);
    result.metrics = compute_metrics(&result);

    // 5. 序列化成前端友好格式
    let equity_curve: Vec<Value> = result
        .equity_curve
        .iter()
        .map(|p| {
            json!({
                "timestamp": p.timestamp.to_rfc3339(),
                "cash": p.cash,
                "position_value": p.position_value,
                "equity": p.equity,
            })
        })
        .collect();

    let trades: Vec<Value> = result
        .trades
        .iter()
        .map(|t| {
            json!({
                "symbol": t.symbol,
                "side": format!("{:?}", t.side).to_uppercase(),
                "entry_time": t.entry_time.to_rfc3339(),
                "exit_time": t.exit_time.map(|x| x.to_rfc3339()),
                "entry_price": t.entry_price,
                "exit_price": t.exit_price,
                "size": t.size,
                "pnl": t.pnl(),
                "pnl_pct": t.pnl_pct(),
                "entry_commission": t.entry_commission,
                "exit_commission": t.exit_commission,
                "commission": t.total_commission(),
            })
        })
        .collect();

    let signals: Vec<Value> = result
        .signals
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
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
        })
        .collect();

    let fills: Vec<Value> = result
        .fills
        .iter()
        .map(|f| {
            json!({
                "timestamp": f.timestamp.to_rfc3339(),
                "side": format!("{:?}", f.side).to_uppercase(),
                "size": f.size,
                "price": f.price,
                "commission": f.commission,
            })
        })
        .collect();

    Ok(Json(json!({
        "bars": bars,
        "source": source,
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
    validate::strategy(name, &p)?;
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
            StrategyKind::Rsi {
                period,
                overbought,
                oversold,
            }
        }
        "random" => {
            let buy_prob = p.get("buy_prob").copied().unwrap_or(0.05);
            let sell_prob = p.get("sell_prob").copied().unwrap_or(0.05);
            StrategyKind::Random {
                seed: 42,
                buy_prob,
                sell_prob,
            }
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
            StrategyKind::DonchianBreakout {
                entry_period,
                exit_period,
            }
        }
        "vwap_reversion" => {
            let period = p.get("period").copied().unwrap_or(20.0) as usize;
            let threshold_pct = p.get("threshold_pct").copied().unwrap_or(1.5);
            StrategyKind::VwapReversion {
                period,
                threshold_pct,
            }
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
            StrategyKind::Ichimoku {
                tenkan,
                kijun,
                senkou_b,
                displacement,
            }
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

async fn get_paper_snapshot(State(state): State<Arc<AppState>>) -> Json<PaperSnapshot> {
    let s = state.paper_state.read().await;
    Json(s.snapshot())
}

async fn post_paper_start(State(state): State<Arc<AppState>>) -> Json<Value> {
    let mut s = state.paper_state.write().await;
    s.set_running(true);
    s.log(crate::paper::PaperLogLevel::Info, "模拟盘已启动".into());
    Json(json!({"status": "started"}))
}

async fn post_paper_stop(State(state): State<Arc<AppState>>) -> Json<Value> {
    let mut s = state.paper_state.write().await;
    s.set_running(false);
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
    s.replace_strategy(strategy);
    s.log(
        crate::paper::PaperLogLevel::Info,
        format!("策略已切换为: {}", req.strategy),
    );
    Ok(Json(json!({"status": "ok", "strategy": req.strategy})))
}

#[derive(Deserialize)]
struct PaperMarketRequest {
    source: String,
    symbol: String,
    strategy: String,
    params: Option<std::collections::HashMap<String, f64>>,
}

async fn post_paper_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PaperMarketRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    crate::api_validation::market(&req.symbol, &req.source, 200, 2_000)?;
    let strategy = make_strategy(&req.strategy, req.params.as_ref())
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let mut paper = state.paper_state.write().await;
    if paper.is_running {
        return Err((
            StatusCode::CONFLICT,
            "请先停止模拟盘，再切换数据源或标的".into(),
        ));
    }
    paper.reconfigure_market(req.source.clone(), req.symbol.clone(), strategy);
    paper.log(
        crate::paper::PaperLogLevel::Info,
        format!("已切换为 {} / {}", req.source, req.symbol),
    );
    Ok(Json(json!({
        "status": "configured",
        "source": req.source,
        "symbol": req.symbol,
        "strategy": paper.strategy.name(),
    })))
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
    let symbol = q
        .symbol
        .unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(100);
    let source = q.source.unwrap_or_else(|| "binance".to_string());
    let bars = market_bars(&state, &symbol, &source, limit).await?;

    use crate::indicators::extra::detect_pattern;
    let patterns: Vec<Value> = bars
        .iter()
        .rev()
        .take(20)
        .map(|bar| {
            let p = detect_pattern(bar);
            json!({
                "timestamp": bar.timestamp.to_rfc3339(),
                "close": bar.close,
                "pattern": p.name_zh(),
                "pattern_code": format!("{:?}", p),
            })
        })
        .collect();

    Ok(Json(json!({ "symbol": symbol, "patterns": patterns })))
}

async fn get_heikin_ashi(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<PatternsQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let symbol = q
        .symbol
        .unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(200);
    let source = q.source.unwrap_or_else(|| "binance".to_string());
    let bars = market_bars(&state, &symbol, &source, limit).await?;

    let ha = crate::indicators::extra::heikin_ashi(&bars);
    let json_bars: Vec<Value> = ha
        .iter()
        .map(|b| {
            json!({
                "timestamp": b.timestamp.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                "open": b.open, "high": b.high, "low": b.low, "close": b.close,
            })
        })
        .collect();

    Ok(Json(
        json!({ "symbol": symbol, "bars": json_bars, "chart": "heikin_ashi" }),
    ))
}

// -----------------------------------------------------------------------------
// 知识库 API —— 返回 PDF 里所有概念的字典
// -----------------------------------------------------------------------------

async fn get_knowledge() -> Json<Value> {
    use crate::knowledge;
    let entries = knowledge::all_entries();
    let total = entries.len();
    // 按 category 分组
    let mut grouped: std::collections::BTreeMap<String, Vec<Value>> =
        std::collections::BTreeMap::new();
    for e in entries {
        let mut val = serde_json::to_value(&e).unwrap_or_default();
        val["source_refs"] = json!(crate::book_sources::for_concept(&e.id));
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
    let symbol = q
        .symbol
        .unwrap_or_else(|| state.config.trading.symbol.clone());
    let limit = q.limit.unwrap_or(200);
    let source = q.source.unwrap_or_else(|| "binance".to_string());
    let bars = market_bars(&state, &symbol, &source, limit).await?;

    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let requested_str = q
        .indicators
        .unwrap_or_else(|| "sma_20,ema_50,rsi_14".to_string());
    // An empty selection intentionally means "price chart only". Filter empty
    // tokens so the multi-select's “全部取消” state remains a valid request.
    let requested: Vec<&str> = requested_str
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();

    let mut series: HashMap<String, Vec<Option<f64>>> = HashMap::new();

    for ind in requested {
        let (name, period) = validate::indicator(ind)?;

        match name {
            "sma" => {
                series.insert(ind.to_string(), crate::indicators::ma::sma(&closes, period));
            }
            "ema" => {
                series.insert(ind.to_string(), crate::indicators::ma::ema(&closes, period));
            }
            "rsi" => {
                series.insert(
                    ind.to_string(),
                    crate::indicators::momentum::rsi(&closes, period),
                );
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
            "vwma" => {
                series.insert(ind.to_string(), crate::indicators::ma::vwma(&bars, period));
            }
            "atr_percent" => {
                series.insert(
                    ind.to_string(),
                    crate::indicators::volatility::atr_percent(&bars, period),
                );
            }
            "atr" => {
                series.insert(
                    ind.to_string(),
                    crate::indicators::volatility::atr(&bars, period),
                );
            }
            "obv" => {
                series.insert(ind.to_string(), crate::indicators::volume::obv(&bars));
            }
            "zscore" | "z_score" => {
                series.insert(
                    ind.to_string(),
                    crate::indicators::extra::zscore(&closes, period),
                );
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
            "stoch" | "stochastic" => {
                let s = crate::indicators::momentum::stochastic(&bars, period, 3, 3);
                series.insert("stoch_k".into(), s.k);
                series.insert("stoch_d".into(), s.d);
            }
            "williams_r" => {
                series.insert(
                    ind.to_string(),
                    crate::indicators::momentum::williams_r(&bars, period),
                );
            }
            "cci" => {
                series.insert(
                    ind.to_string(),
                    crate::indicators::momentum::cci(&bars, period),
                );
            }
            "adx" | "dmi_adx" => {
                let d = crate::indicators::trend::dmi(&bars, period);
                series.insert("adx_plus_di".into(), d.plus_di);
                series.insert("adx_minus_di".into(), d.minus_di);
                series.insert("adx_adx".into(), d.adx);
            }
            "bbi" => {
                series.insert(ind.to_string(), crate::indicators::ma::bbi(&bars));
            }
            "alligator" => {
                let a = crate::indicators::ma::alligator(&bars);
                series.insert("alligator_jaw".into(), a.jaw);
                series.insert("alligator_teeth".into(), a.teeth);
                series.insert("alligator_lips".into(), a.lips);
            }
            "ppo" => {
                let closes_f = closes.clone();
                let n = closes_f.len();
                let ema_f = crate::indicators::ma::ema(&closes_f, 12);
                let ema_s = crate::indicators::ma::ema(&closes_f, 26);
                let ppo: Vec<Option<f64>> = (0..n)
                    .map(|i| match (ema_f[i], ema_s[i]) {
                        (Some(f), Some(s)) if s != 0.0 => Some((f - s) / s * 100.0),
                        _ => None,
                    })
                    .collect();
                series.insert("ppo".into(), ppo);
            }
            "vortex" => {
                let v = crate::indicators::trend::vortex(&bars, period);
                series.insert("vortex_plus".into(), v.plus);
                series.insert("vortex_minus".into(), v.minus);
            }
            _ => {}
        }
    }

    // 同时返回原始 K 线
    let json_bars: Vec<Value> = bars
        .iter()
        .map(|b| {
            json!({
                "timestamp": b.timestamp.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
                "open": b.open, "high": b.high, "low": b.low, "close": b.close,
                "volume": b.volume,
            })
        })
        .collect();

    // 把指标序列转成 (timestamp, value) 对, 便于前端画图
    let indicator_output: HashMap<String, Vec<Option<Value>>> = series
        .iter()
        .map(|(k, v)| {
            let pairs: Vec<Option<Value>> = v
                .iter()
                .enumerate()
                .map(|(i, val)| {
                    val.filter(|x| x.is_finite())
                        .map(|x| json!({"x": bars[i].timestamp.to_rfc3339(), "y": x}))
                })
                .collect();
            (k.clone(), pairs)
        })
        .collect();

    Ok(Json(json!({
        "symbol": symbol,
        "bars": json_bars,
        "indicators": indicator_output,
        "source": source,
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
static SYMBOLS_REFRESHING: OnceLock<TokioMutex<bool>> = OnceLock::new();

fn symbols_cache() -> &'static TokioRwLock<Option<SymbolsCache>> {
    SYMBOLS_CACHE.get_or_init(|| TokioRwLock::new(None))
}

fn symbols_refreshing() -> &'static TokioMutex<bool> {
    SYMBOLS_REFRESHING.get_or_init(|| TokioMutex::new(false))
}

#[derive(serde::Deserialize)]
struct BinanceSymbol {
    symbol: String,
    status: String,
    #[serde(rename = "quoteAsset")]
    quote_asset: String,
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
    fetch_symbols_from_binance_at("https://data-api.binance.vision").await
}

async fn fetch_symbols_from_binance_at(base_url: &str) -> anyhow::Result<Vec<String>> {
    let base_url = base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    // 获取所有交易对
    let exchange_info: serde_json::Value = client
        .get(format!("{base_url}/api/v3/exchangeInfo"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let raw_symbols: Vec<BinanceSymbol> = serde_json::from_value(exchange_info["symbols"].clone())?;
    // 获取 24h 成交量排序
    let tickers: Vec<BinanceTicker> = client
        .get(format!("{base_url}/api/v3/ticker/24hr"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(select_binance_symbols(raw_symbols, tickers))
}

fn select_binance_symbols(symbols: Vec<BinanceSymbol>, tickers: Vec<BinanceTicker>) -> Vec<String> {
    let volumes: HashMap<String, f64> = tickers
        .into_iter()
        .map(|ticker| {
            let volume = ticker
                .quote_volume
                .parse::<f64>()
                .ok()
                .filter(|volume| volume.is_finite() && *volume >= 0.0)
                .unwrap_or(0.0);
            (ticker.symbol, volume)
        })
        .collect();
    let mut pairs: Vec<(String, f64)> = symbols
        .into_iter()
        .filter(|symbol| {
            symbol.status == "TRADING"
                && symbol.is_spot_trading_allowed.unwrap_or(false)
                && symbol.quote_asset == "USDT"
        })
        .map(|symbol| {
            let volume = volumes.get(&symbol.symbol).copied().unwrap_or(0.0);
            (symbol.symbol, volume)
        })
        .collect();
    pairs.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    pairs.into_iter().map(|(symbol, _)| symbol).collect()
}

async fn refresh_binance_symbols_in_background() {
    let mut refreshing = symbols_refreshing().lock().await;
    if *refreshing {
        return;
    }
    *refreshing = true;
    tokio::spawn(async {
        let fetched = fetch_symbols_from_binance().await;
        if let Ok(symbols) = fetched {
            *symbols_cache().write().await = Some(SymbolsCache {
                symbols,
                fetched_at: chrono::Utc::now(),
            });
        } else if let Err(error) = fetched {
            tracing::warn!(%error, "Binance directory refresh failed; retaining the immediately usable catalog");
        }
        *symbols_refreshing().lock().await = false;
    });
}

async fn get_cached_symbols() -> (Vec<String>, &'static str, bool) {
    if std::env::var("AXIOM_OFFLINE").as_deref() == Ok("1") {
        return (vec!["BTCUSDT".into(), "ETHUSDT".into()], "offline", false);
    }
    if let Some(cache) = symbols_cache().read().await.as_ref().cloned() {
        let age = (chrono::Utc::now() - cache.fetched_at).num_seconds();
        if age >= 300 {
            refresh_binance_symbols_in_background().await;
            return (cache.symbols, "stale", true);
        }
        return (cache.symbols, "cached", true);
    }
    // The dropdown must open immediately even when an upstream directory is
    // cold or unreachable. The full catalog replaces this seed asynchronously.
    refresh_binance_symbols_in_background().await;
    (default_symbols(), "refreshing", false)
}

fn default_symbols() -> Vec<String> {
    vec![
        "BTCUSDT",
        "ETHUSDT",
        "SOLUSDT",
        "BNBUSDT",
        "XRPUSDT",
        "DOGEUSDT",
        "ADAUSDT",
        "AVAXUSDT",
        "MATICUSDT",
        "DOTUSDT",
        "LINKUSDT",
        "TRXUSDT",
        "LTCUSDT",
        "BCHUSDT",
        "ATOMUSDT",
        "NEARUSDT",
        "APTUSDT",
        "OPUSDT",
        "ARBUSDT",
        "INJUSDT",
        "SUIUSDT",
        "SEIUSDT",
        "TIAUSDT",
        "WLDUSDT",
        "PEPEUSDT",
        "SHIBUSDT",
        "FILUSDT",
        "ICPUSDT",
        "STXUSDT",
        "RNDRUSDT",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[derive(Deserialize)]
struct SymbolsQuery {
    source: Option<String>,
    q: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

async fn get_symbols(
    axum::extract::Query(q): axum::extract::Query<SymbolsQuery>,
) -> Result<Json<Value>, ApiError> {
    let source = q.source.unwrap_or_else(|| "binance".into());
    if !crate::data::is_public_market_source(&source) {
        return Err(validate::bad("source must be binance, a_share or us_stock"));
    }
    let query = q.q.unwrap_or_default();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    if query.chars().count() > 64 {
        return Err(validate::bad("symbol query must be at most 64 characters"));
    }
    if source == "binance" {
        let (catalog, status, complete) = get_cached_symbols().await;
        let mut items: Vec<_> = catalog
            .into_iter()
            .filter(|symbol| {
                query.is_empty() || symbol.to_uppercase().contains(&query.to_uppercase())
            })
            .map(|symbol| json!({"symbol": symbol, "name": "Binance spot", "exchange": "Binance"}))
            .collect();
        items.sort_by_key(|item| item["symbol"].as_str().unwrap_or_default().to_owned());
        let total = items.len();
        let page: Vec<_> = items.into_iter().skip(offset).take(limit).collect();
        let symbols: Vec<_> = page
            .iter()
            .filter_map(|item| item["symbol"].as_str())
            .collect();
        return Ok(Json(json!({
            "symbols": symbols, "items": page, "count": symbols.len(), "total": total,
            "universe_count": total, "offset": offset, "has_more": offset + symbols.len() < total,
            "status": status, "complete": complete, "source": source
        })));
    }
    let (items, total, universe_count, status, complete) =
        crate::symbols::search(&source, &query, offset, limit)
            .await
            .map_err(|error| validate::bad(error.to_string()))?;
    let symbols: Vec<_> = items.iter().map(|item| item.symbol.as_str()).collect();
    let count = symbols.len();
    Ok(Json(json!({
        "symbols": symbols, "items": items, "count": count, "total": total,
        "universe_count": universe_count, "offset": offset, "has_more": offset + count < total,
        "status": status, "complete": complete, "source": source
    })))
}

// -----------------------------------------------------------------------------
// WebSocket:实时推送模拟盘状态
// -----------------------------------------------------------------------------

async fn ws_paper(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| ws_paper_loop(socket, state))
}

async fn ws_paper_loop(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let snapshot = state.paper_state.read().await.snapshot();
                if sender.send(Message::Text(serde_json::to_string(&snapshot).unwrap_or_default())).await.is_err() { break; }
            }
            message = receiver.next() => {
                match message {
                    Some(Ok(Message::Ping(data))) => { let sent = sender.send(Message::Pong(data)).await; if sent.is_err() { break; } }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}
// -----------------------------------------------------------------------------
// GET /api/code_loc?ref=src/path.rs::function
// 返回符号当前所在行号 (行级跳转)
// -----------------------------------------------------------------------------

async fn get_code_loc(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let reference = q.get("ref").map(String::as_str).unwrap_or("");
    match crate::code_links::resolve(reference) {
        Some(location) => {
            let mut value = json!(location);
            value["ok"] = json!(true);
            value["ref"] = json!(reference);
            Json(value)
        }
        None => Json(
            json!({"ok":false,"ref":reference,"line":null,"url":null,"error":"该实现引用不存在于当前构建，请检查知识条目与实现是否一起更新。"}),
        ),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

async fn get_code_source(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    let path = q.get("path").map(String::as_str).unwrap_or("");
    let Some(source) = crate::code_links::source(path) else {
        return (StatusCode::NOT_FOUND, "当前构建中没有该源码文件").into_response();
    };
    let line = q
        .get("line")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let end = q
        .get("end_line")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(line);
    let title = html_escape(path);
    let mut rows = String::new();
    use std::fmt::Write;
    for (i, source_line) in source.lines().enumerate() {
        let n = i + 1;
        let class = if n >= line && n <= end {
            "selected"
        } else {
            ""
        };
        let _=writeln!(rows,"<span id=\"L{n}\" class=\"row {class}\"><a href=\"#L{n}\" class=\"line\" aria-label=\"第{n}行\">{n}</a><code>{}</code></span>",html_escape(source_line));
    }
    let revision = crate::code_links::revision().unwrap_or("本地构建");
    let state = if crate::code_links::is_dirty() {
        "含本地改动 · 展示与正在运行的程序完全一致的源码快照"
    } else {
        "已提交 · 固定版本源码快照"
    };
    let html = format!(
        r##"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title} · AXIOM 实现</title><style>
    :root{{color-scheme:light dark;--bg:#f5f7fa;--fg:#202d3c;--muted:#576579;--selection:#dceafe;--border:#c4ccda}}@media(prefers-color-scheme:dark){{:root{{--bg:#151c25;--fg:#dbe4ef;--muted:#aab9cb;--selection:#243b56;--border:#44556d}}}}*{{box-sizing:border-box}}body{{margin:0;background:var(--bg);color:var(--fg);font:14px/1.65 ui-monospace,SFMono-Regular,Consolas,monospace}}header{{padding:20px 28px;border-bottom:1px solid var(--border)}}h1{{font-size:20px;margin:0 0 8px}}p{{margin:3px 0;color:var(--muted)}}pre{{margin:0;padding:18px 0;overflow:auto}}.row{{display:flex;min-width:max-content;scroll-margin-top:12px}}.line{{width:76px;flex:none;padding-right:22px;text-align:right;color:var(--muted);text-decoration:none;user-select:none}}code{{white-space:pre;padding-right:24px}}.selected,:target{{background:var(--selection)}}a:focus-visible{{outline:2px solid currentColor}}
    </style></head><body><header><h1>{title}</h1><p>第 {line}–{end} 行 · {state}</p><p>版本 {revision} · 行号来自构建时的 Rust 语法树</p></header><pre aria-label="实现源码">{rows}</pre></body></html>"##
    );
    (
        [
            ("content-type", "text/html; charset=utf-8"),
            (
                "content-security-policy",
                "default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'self'",
            ),
            ("x-content-type-options", "nosniff"),
        ],
        html,
    )
        .into_response()
}

/// Compatibility seam for existing symbol-location tests.
pub fn locate_symbol_for_test(reference: &str) -> Option<usize> {
    crate::code_links::resolve(reference).map(|location| location.line)
}

fn practice_plan(concept: &crate::practice::PracticeConcept) -> Value {
    let financial = matches!(
        concept.category.as_str(),
        "估值"
            | "现金流"
            | "盈利"
            | "财务质量"
            | "行业"
            | "股东"
            | "分析师"
            | "原书财务计算"
            | "原书行业专属指标"
    ) || matches!(
        concept.id.as_str(),
        "book_share_counts"
            | "book_float_market_cap"
            | "book_dcf"
            | "book_revenue"
            | "book_ebitda_margin"
            | "book_net_margin"
            | "book_roce"
            | "book_yoy"
            | "book_qoq"
            | "book_debt_ratio"
            | "book_de_ratio"
            | "book_net_debt"
            | "book_net_debt_ebitda"
            | "book_cash_ratio"
            | "book_cfo"
            | "book_capex"
            | "book_fcf"
            | "book_cfo_income"
            | "book_asset_turnover"
            | "book_dpo"
            | "book_ccc"
            | "book_intangibles_ratio"
            | "book_diluted_shares"
            | "book_free_float"
            | "book_adjustment"
            | "book_cape"
    );
    let derivatives = concept.category == "期权"
        || matches!(
            concept.id.as_str(),
            "book_option_value_components"
                | "book_option_moneyness"
                | "book_option_dte"
                | "book_option_volume_oi"
                | "book_iv_percentile"
                | "book_iv_smile"
                | "book_implied_move"
                | "book_rho"
                | "book_second_order_greeks"
        );
    let performance = concept.category == "风险-绩效";
    if concept.id == "book_period" {
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["data"],
            "required_datasets":["completed_ohlcv_with_source_cadence"],
            "source_policy":"real_required",
            "goal":"使用所选市场的已收盘K线核对来源约定的名义周期与相邻观测间隔：Binance为连续1小时K线（3600秒），A股和美股为日线（名义86400秒）。股票周末、节假日或停牌形成的日期空档不会被误判为多日K线；不接受手填周期或模拟数据。"
        })
    } else if concept.id == "book_annualized_volatility" {
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["data"],
            "required_datasets":["completed_ohlcv_with_source_cadence"],
            "source_policy":"real_required",
            "goal":"使用所选市场的已收盘K线计算相邻收盘价简单收益率样本标准差；加密1小时线按8760小时/年，A股和美股日线按252个交易日/年惯例。年化口径由已验证数据源决定，不接受手填。"
        })
    } else if concept.id == "book_pitfall_repainting" {
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["data"],
            "required_datasets":["server_fetched_completed_ohlcv","two_right_side_completed_bars"],
            "source_policy":"real_required",
            "goal":"由服务器从所选真实来源取得已收盘 K 线，以固定两根右侧 K 线确认局部高低点。结果分别显示回看的枢轴发生位置和 t+2 的确认位置；不接收手填 K 线或事件索引，不能把 t 时点的标签当作当时已知信息。这只演示该五根 K 线分形的确认延迟，不复制所有 ZigZag 或自动形态的专有规则。"
        })
    } else if concept.id == "rolling_correlation" {
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["backtest","paper","compare"],
            "required_datasets":["matched_result_bars","real_strategy_equity","same_period_benchmark_returns"],
            "source_policy":"result_required",
            "goal":"使用本页真实策略净值逐期收益与同时间戳标的收盘收益，按所选窗口计算滚动皮尔逊相关系数。窗口未满或任一窗口序列为常数时返回空值；它说明策略与所选标的的同期联动，不是双资产配对交易证据，也不接受手填收益数组。"
        })
    } else if concept.id == "book_r_squared" {
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["backtest","paper","compare"],
            "required_datasets":["matched_result_bars","real_strategy_equity","same_period_benchmark_returns"],
            "source_policy":"result_required",
            "goal":"使用本页结果中按相同顺序提供的净值和已收盘行情，核对相邻简单收益与基准收盘收益逐期一致；按带截距一元回归的相关系数平方计算 R²，不使用教学默认数组。"
        })
    } else if concept.id == "book_cdp" {
        json!({
            "markets":["cn_equity"],
            "modules":["data"],
            "required_datasets":["two_ordered_completed_a_share_daily_ohlcv"],
            "source_policy":"real_required",
            "goal":"只用所选A股中有序的最近两根已收盘日线：以前一根日线高低收计算最后一根已收盘日线所属交易时段的CDP、AH、AL、NH、NL；不把它标为当前自然日，不接受手填价格或小时K线。"
        })
    } else if concept.id == "book_relative_volume_at_time" {
        json!({"markets":["crypto"],"modules":["data"],"required_datasets":["continuous_completed_binance_1h_ohlcv","seven_complete_utc_history_days"],"source_policy":"real_required","goal":"以最后完整UTC小时为截止，累计当日量并与过去7个完整UTC日同一截止小时累计量均值比较；不接受手填观测量。"})
    } else if concept.id == "book_volume_24h" {
        json!({
            "markets":["crypto"],
            "modules":["data"],
            "required_datasets":["24_consecutive_completed_1h_ohlcv"],
            "source_policy":"real_required",
            "goal":"只汇总所选加密交易对最近24根连续且已收盘的1小时成交量；缺口和日线不可当作24小时数据。"
        })
    } else if concept.input_kind == "market_bars" {
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["data","backtest","compare"],
            "required_datasets":["completed_ohlcv"],
            "source_policy":"real_required",
            "goal":"用同一段已收盘真实行情观察数值、再检验策略或比较策略；不把指标本身当交易指令。"
        })
    } else if performance {
        let benchmark_dependent = matches!(
            concept.id.as_str(),
            "information_ratio" | "treynor" | "tracking_error" | "capture_ratio" | "beta" | "alpha"
        );
        let required_datasets = if benchmark_dependent {
            json!([
                "matched_result_bars",
                "real_strategy_returns",
                "same_period_benchmark_returns"
            ])
        } else if matches!(
            concept.id.as_str(),
            "total_return" | "cagr" | "max_drawdown" | "calmar"
        ) {
            json!(["matched_result_bars", "real_equity_curve"])
        } else {
            json!(["matched_result_bars", "real_return_series"])
        };
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["backtest","paper","compare"],
            "required_datasets":required_datasets,
            "source_policy":"result_required",
            "goal":"从当前回测、模拟盘或策略对比的已提供行情和结果计算绩效；需要基准的指标必须传入同频、同区间的基准收益，不能使用教学默认数组。"
        })
    } else if matches!(concept.id.as_str(), "book_etf_balances" | "book_etf_flows") {
        json!({
            "markets":["us_equity"],
            "modules":["data"],
            "required_datasets":["dated_crypto_etf_holdings_or_flows","etf_symbol"],
            "source_policy":"evidence_required",
            "goal":"核对美国上市加密资产 ETF 的官方持仓或申赎披露与报告日期；不能从币价或成交量反推持仓与净流入。"
        })
    } else if financial {
        let (required_datasets, goal) = match concept.id.as_str() {
            "book_cape" => (
                json!(["ten_annual_point_in_time_eps", "ten_annual_cpi", "market_price"]),
                "使用连续十年的已披露每股收益、同年物价指数和评估时点股价；年度缺口不能用默认教学数组填补。",
            ),
            "book_adjustment" => (
                json!(["dated_corporate_actions", "raw_market_price"]),
                "用已公布的分红、拆股等行动计算评估时点的复权因子；没有行动记录时不能把教学因子当真实价格。",
            ),
            "book_share_counts" | "book_float_market_cap" | "book_free_float" => (
                json!(["dated_share_register", "market_price"]),
                "核对相同评估时点的股本披露与行情；自由流通股和总股本的口径必须一致。",
            ),
            _ => (
                json!(["point_in_time_filing", "market_price"]),
                "将可追溯、已在评估时点公开的财报字段与对应市场价格输入公式；缺少报告时必须明确提示缺什么。",
            ),
        };
        json!({
            "markets":["cn_equity","us_equity"],
            "modules":["data"],
            "required_datasets":required_datasets,
            "source_policy":"evidence_required",
            "goal":goal
        })
    } else if derivatives {
        json!({
            "markets":["us_equity"],
            "modules":["data","compare"],
            "required_datasets":["timestamped_option_chain","underlying_price"],
            "source_policy":"evidence_required",
            "goal":"以同一报价时点的期权链和标的价格计算；没有期权链时不伪造结论。"
        })
    } else if concept.category == "衍生品与链上" {
        json!({
            "markets":["crypto"],
            "modules":["data"],
            "required_datasets":["timestamped_derivatives_or_blockchain_observations"],
            "source_policy":"evidence_required",
            "goal":"收集该交易对、交易所或链上网络的独立观测，并记录时间和口径；现货 K 线不能代替持仓、资金费率或链上数据。"
        })
    } else {
        json!({
            "markets":["crypto","cn_equity","us_equity"],
            "modules":["data"],
            "required_datasets":["documented_input_or_market_snapshot"],
            "source_policy":"evidence_required",
            "goal":"按概念的数据口径收集证据或行情快照；教学输入只用于理解公式，不能替代真实练习。"
        })
    }
}

async fn get_practice() -> Json<Value> {
    let concepts = crate::practice::catalog();
    let concepts: Vec<Value> = concepts
        .iter()
        .map(|concept| {
            let mut value = serde_json::to_value(concept).expect("practice catalog serializes");
            value["plan"] = practice_plan(concept);
            value
        })
        .collect();
    Json(
        json!({"total":concepts.len(),"concepts":concepts,"modules":["data","backtest","paper","compare"]}),
    )
}

#[derive(Deserialize)]
struct PracticeRequest {
    concept_id: String,
    module: String,
    symbol: Option<String>,
    source: Option<String>,
    limit: Option<usize>,
    bars: Option<Vec<Bar>>,
    #[serde(default = "empty_inputs")]
    inputs: Value,
}

fn source_market(source: &str) -> Option<&'static str> {
    match source {
        "real" | "binance" | "synthetic" => Some("crypto"),
        "a_share" => Some("cn_equity"),
        "us_stock" => Some("us_equity"),
        _ => None,
    }
}

fn practice_bars_are_closed(bars: &[Bar], source: &str) -> Result<(), ApiError> {
    let close_after = source_candle_close_after(source)?;
    if bars
        .last()
        .is_some_and(|bar| bar.timestamp + close_after > Utc::now())
    {
        return Err(validate::bad(
            "practice bars include an unfinished or future market candle",
        ));
    }
    Ok(())
}

fn relative_volume_requires_continuous_binance_hours(
    bars: &[Bar],
    source: &str,
) -> Result<(), ApiError> {
    if !matches!(source, "binance" | "real") {
        return Err(validate::bad("RVAT requires Binance 1-hour bars"));
    }
    let end = bars
        .last()
        .ok_or_else(|| validate::bad("RVAT needs bars"))?;
    let needed = 7 * 24 + end.timestamp.hour() as usize + 1;
    if bars.len() < needed {
        return Err(validate::bad("RVAT needs seven complete UTC history days plus the current UTC day through its cutoff hour"));
    }
    let suffix = &bars[bars.len() - needed..];
    if suffix[0].timestamp != end.timestamp - Duration::hours((needed - 1) as i64)
        || suffix
            .windows(2)
            .any(|p| p[1].timestamp - p[0].timestamp != Duration::hours(1))
    {
        return Err(validate::bad(
            "RVAT requires continuous Binance 1-hour bars in its seven-day UTC comparison window",
        ));
    }
    Ok(())
}

fn cdp_requires_daily_a_share_bars(bars: &[Bar]) -> Result<(), ApiError> {
    if bars.len() >= 2
        && bars
            .windows(2)
            .any(|pair| pair[1].timestamp - pair[0].timestamp < Duration::hours(20))
    {
        return Err(validate::bad(
            "CDP requires A-share daily bars; intraday bars are not a previous trading session",
        ));
    }
    Ok(())
}

fn annualization_basis_for_source(
    source: &str,
) -> Result<crate::book_technical::AnnualizationBasis, ApiError> {
    match source {
        "real" | "binance" => Ok(crate::book_technical::AnnualizationBasis::CryptoHourly),
        "a_share" | "us_stock" => Ok(crate::book_technical::AnnualizationBasis::EquityDaily),
        _ => Err(validate::bad("unsupported annualized-volatility source")),
    }
}

fn annualized_volatility_requires_source_cadence(
    bars: &[Bar],
    source: &str,
) -> Result<(), ApiError> {
    let intraday = bars
        .windows(2)
        .any(|pair| pair[1].timestamp - pair[0].timestamp < Duration::hours(20));
    match source {
        "real" | "binance"
            if bars
                .windows(2)
                .any(|pair| pair[1].timestamp - pair[0].timestamp != Duration::hours(1)) =>
        {
            Err(validate::bad(
                "annualized volatility requires consecutive 1-hour crypto bars",
            ))
        }
        "a_share" | "us_stock" if intraday => Err(validate::bad(
            "annualized volatility requires daily equity bars; weekends and holidays may be absent",
        )),
        "real" | "binance" | "a_share" | "us_stock" => Ok(()),
        _ => Err(validate::bad("unsupported annualized-volatility source")),
    }
}

fn plan_allows(plan: &Value, field: &str, value: &str) -> bool {
    plan[field]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(value)))
}

fn finite_input_array(inputs: &Value, key: &str) -> Result<Vec<f64>, ApiError> {
    let values = inputs[key]
        .as_array()
        .ok_or_else(|| validate::bad(format!("result_required practice needs {key}")))?;
    if values.is_empty() {
        return Err(validate::bad(format!(
            "result_required practice needs nonempty {key}"
        )));
    }
    values
        .iter()
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    validate::bad(format!("result_required practice needs finite {key}"))
                })
        })
        .collect()
}

fn is_close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-10 * (1.0 + a.abs().max(b.abs()))
}

fn validate_r_squared_observations(bars: &[Bar], inputs: &Value) -> Result<(), ApiError> {
    let strategy = finite_input_array(inputs, "strategy_returns")?;
    let benchmark = finite_input_array(inputs, "benchmark_returns")?;
    let equity = finite_input_array(inputs, "equity")?;
    if bars.len() < 3 || strategy.len() != bars.len() - 1 || benchmark.len() != bars.len() - 1 {
        return Err(validate::bad(
            "R² practice needs at least three aligned bars and two same-period returns",
        ));
    }
    let offset = if equity.len() == bars.len() {
        0
    } else if equity.len() == bars.len() + 1 {
        let initial = inputs["initial_capital"]
            .as_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or_else(|| validate::bad("R² practice needs finite positive initial_capital for the initial-equity offset"))?;
        if !is_close(equity[0], initial) {
            return Err(validate::bad(
                "R² initial_capital must match the leading equity observation",
            ));
        }
        1
    } else {
        return Err(validate::bad(
            "R² equity must have one value per supplied bar, optionally preceded by initial_capital",
        ));
    };
    if equity.iter().any(|value| *value <= 0.0) {
        return Err(validate::bad("R² equity observations must be positive"));
    }
    let points = inputs["equity_points"]
        .as_array()
        .filter(|points| points.len() == bars.len())
        .ok_or_else(|| {
            validate::bad("R² practice needs one timestamped equity point per supplied bar")
        })?;
    for (index, (bar, point)) in bars.iter().zip(points).enumerate() {
        let timestamp = point["timestamp"]
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .ok_or_else(|| validate::bad("R² equity_points need RFC3339 timestamps"))?;
        let value = point["equity"]
            .as_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or_else(|| validate::bad("R² equity_points need finite positive equity"))?;
        if timestamp != bar.timestamp {
            return Err(validate::bad(
                "R² equity_points timestamps must exactly match supplied bars",
            ));
        }
        if !is_close(value, equity[offset + index]) {
            return Err(validate::bad(
                "R² equity_points must match the supplied equity sequence",
            ));
        }
    }
    for index in 1..bars.len() {
        let expected_strategy = equity[offset + index] / equity[offset + index - 1] - 1.0;
        let expected_benchmark = bars[index].close / bars[index - 1].close - 1.0;
        if !is_close(strategy[index - 1], expected_strategy) {
            return Err(validate::bad(
                "strategy_returns must equal consecutive supplied equity returns",
            ));
        }
        if !is_close(benchmark[index - 1], expected_benchmark) {
            return Err(validate::bad(
                "benchmark_returns must equal consecutive same-timestamp bar-close returns",
            ));
        }
    }
    Ok(())
}

fn validate_rolling_correlation_observations(bars: &[Bar], inputs: &Value) -> Result<(), ApiError> {
    validate_r_squared_observations(bars, inputs)?;
    let strategy = finite_input_array(inputs, "strategy_returns")?;
    let benchmark = finite_input_array(inputs, "benchmark_returns")?;
    let series_x = finite_input_array(inputs, "series_x")?;
    let series_y = finite_input_array(inputs, "series_y")?;
    if series_x.len() != strategy.len() || series_y.len() != benchmark.len() {
        return Err(validate::bad(
            "rolling correlation aliases must have one value per verified return",
        ));
    }
    for index in 0..strategy.len() {
        if !is_close(series_x[index], strategy[index])
            || !is_close(series_y[index], benchmark[index])
        {
            return Err(validate::bad("rolling correlation aliases must exactly match verified strategy and benchmark returns"));
        }
    }
    Ok(())
}

fn validate_result_context(
    concept: &crate::practice::PracticeConcept,
    bars: &[Bar],
    inputs: &Value,
) -> Result<(), ApiError> {
    if bars.is_empty() {
        return Err(validate::bad(
            "result_required practice needs nonempty provided bars from the selected result",
        ));
    }
    if !inputs.is_object() {
        return Err(validate::bad("inputs must be an object"));
    }
    for input in &concept.inputs {
        if inputs.get(&input.key).is_none_or(Value::is_null) {
            return Err(validate::bad(format!(
                "result_required practice needs explicitly provided {}",
                input.key
            )));
        }
    }
    let benchmark_dependent = matches!(
        concept.id.as_str(),
        "information_ratio"
            | "treynor"
            | "tracking_error"
            | "capture_ratio"
            | "beta"
            | "alpha"
            | "book_r_squared"
            | "rolling_correlation"
    );
    if benchmark_dependent {
        let strategy = finite_input_array(inputs, "strategy_returns")?;
        let benchmark = finite_input_array(inputs, "benchmark_returns")?;
        if strategy.len() != benchmark.len() {
            return Err(validate::bad(
                "strategy_returns and benchmark_returns must have matching lengths",
            ));
        }
        if strategy.len() < 2 {
            return Err(validate::bad(
                "benchmark performance practice needs at least two aligned returns",
            ));
        }
        if concept.id == "book_r_squared" {
            validate_r_squared_observations(bars, inputs)?;
        } else if concept.id == "rolling_correlation" {
            validate_rolling_correlation_observations(bars, inputs)?;
        }
    } else if matches!(
        concept.id.as_str(),
        "total_return" | "cagr" | "max_drawdown" | "calmar"
    ) {
        let equity = finite_input_array(inputs, "equity")?;
        if equity.len() < 2 || equity.iter().any(|value| *value <= 0.0) {
            return Err(validate::bad(
                "result_required practice needs at least two positive equity observations",
            ));
        }
        if !matches!(equity.len(), n if n == bars.len() || n == bars.len() + 1) {
            return Err(validate::bad(
                "equity length must match provided bars (or include one initial capital observation)",
            ));
        }
    } else {
        let returns = finite_input_array(inputs, "returns")?;
        if returns.iter().any(|value| *value < -1.0) {
            return Err(validate::bad("returns must not be below -1"));
        }
    }
    Ok(())
}

async fn post_practice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PracticeRequest>,
) -> Result<Json<Value>, ApiError> {
    let symbol = req
        .symbol
        .unwrap_or_else(|| state.config.trading.symbol.clone());
    let source = req.source.unwrap_or_else(|| "binance".into());
    let limit = req.limit.unwrap_or(200);
    let registry = crate::practice::catalog();
    let concept = registry
        .iter()
        .find(|c| c.id == req.concept_id)
        .ok_or_else(|| validate::bad("未知概念"))?;
    let plan = practice_plan(concept);
    if !plan_allows(&plan, "modules", &req.module) {
        return Err(validate::bad(
            "practice module is not applicable to this concept",
        ));
    }
    validate::market(&symbol, &source, limit, 5000)?;
    let market =
        source_market(&source).ok_or_else(|| validate::bad("unsupported practice source"))?;
    if !plan_allows(&plan, "markets", market) {
        return Err(validate::bad(
            "practice source is not applicable to this concept",
        ));
    }
    if concept.id == "book_period"
        && !req
            .inputs
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
    {
        return Err(validate::bad(
            "K线周期由已验证来源和K线决定，不接受手填覆盖",
        ));
    }
    if plan["source_policy"].as_str() == Some("real_required") && source == "synthetic" {
        return Err(validate::bad(
            "real_required practice does not accept synthetic market bars",
        ));
    }
    if concept.id == "book_pitfall_repainting" && req.bars.is_some() {
        return Err(validate::bad(
            "repainting practice fetches completed source bars on the server and does not accept caller-supplied bars",
        ));
    }
    let independent = concept.input_kind != "market_bars";
    let provided_bars = req.bars.is_some();
    let bars = if let Some(bars) = req.bars {
        validate::bars(&bars)?;
        bars
    } else if independent {
        Vec::new()
    } else {
        market_bars(&state, &symbol, &source, limit).await?
    };
    if !bars.is_empty() {
        practice_bars_are_closed(&bars, &source)?;
    }
    if concept.id == "book_cdp" {
        cdp_requires_daily_a_share_bars(&bars)?;
    }
    if concept.id == "book_relative_volume_at_time" {
        relative_volume_requires_continuous_binance_hours(&bars, &source)?;
    }
    let period_summary = if concept.id == "book_period" {
        Some(crate::book::market_period_summary(&bars, &source).map_err(validate::bad)?)
    } else {
        None
    };
    let annualization = if concept.id == "book_annualized_volatility" {
        annualized_volatility_requires_source_cadence(&bars, &source)?;
        Some(annualization_basis_for_source(&source)?)
    } else {
        None
    };
    let result_required = plan["source_policy"].as_str() == Some("result_required");
    if result_required {
        // The evaluator has teaching defaults. Result modules must never fall back to them.
        if !provided_bars {
            return Err(validate::bad(
                "result_required practice needs provided bars from the selected result",
            ));
        }
        validate_result_context(concept, &bars, &req.inputs)?;
    }
    let context = if result_required {
        "provided_result_context"
    } else if independent {
        "editable_teaching_inputs"
    } else if provided_bars {
        "module_snapshot"
    } else {
        "selected_dataset"
    };
    // Result screens share one context payload across all performance concepts.
    // Keep that provenance data for boundary validation, but do not pass fields a
    // particular formula does not declare to the strict concept evaluator.
    let mut evaluator_inputs = req.inputs.clone();
    if result_required {
        if let Some(values) = evaluator_inputs.as_object_mut() {
            for key in [
                "equity",
                "equity_points",
                "returns",
                "initial_capital",
                "elapsed_days",
                "periods_per_year",
                "risk_free_annual",
                "strategy_returns",
                "benchmark_returns",
                "confidence",
            ] {
                if !concept.inputs.iter().any(|input| input.key == key) {
                    values.remove(key);
                }
            }
        }
    }
    let mut result = match period_summary {
        Some(summary) => summary,
        None => match annualization {
            Some(annualization) => crate::practice::evaluate_with_annualization(
                &req.concept_id,
                &bars,
                &evaluator_inputs,
                annualization,
            ),
            None => crate::practice::evaluate(&req.concept_id, &bars, &evaluator_inputs),
        }
        .map_err(validate::bad)?,
    };
    result["module"] = json!(req.module);
    result["symbol"] = json!(symbol);
    result["source"] = json!(source);
    result["context"] = json!(context);
    if concept.id == "book_pitfall_repainting" {
        result["bar_origin"] = json!("server_fetched_completed_source_bars");
        if let Some(notes) = result["notes"].as_array_mut() {
            notes.push(json!("本次练习由服务器从所选来源重新获取并过滤已收盘 K 线；响应 bars 含实际使用的时间戳，可能比页面图表更新。"));
        }
    }
    if result_required {
        result["provenance"] = json!("provided_result_context");
    }
    result["bars"] = json!(bars);
    Ok(Json(result))
}

fn empty_inputs() -> Value {
    json!({})
}

#[cfg(test)]
mod binance_catalog_tests {
    use super::{
        completed_market_bars, fetch_symbols_from_binance_at, practice_bars_are_closed,
        select_binance_symbols, source_candle_close_after, BinanceSymbol, BinanceTicker,
    };
    use crate::types::Bar;
    use axum::{routing::get, Json, Router};
    use chrono::{Duration, Utc};
    use serde_json::json;

    #[test]
    fn completed_candle_policy_is_shared_across_market_and_practice_boundaries() {
        let now = Utc::now();
        for (source, expected) in [("binance", 1), ("a_share", 8), ("us_stock", 22)] {
            let close_after = source_candle_close_after(source).unwrap();
            assert_eq!(close_after, Duration::hours(expected));
            let completed = Bar {
                timestamp: now - close_after - Duration::seconds(1),
                open: 1.0,
                high: 1.0,
                low: 1.0,
                close: 1.0,
                volume: 0.0,
            };
            let unfinished = Bar {
                timestamp: now - close_after + Duration::seconds(1),
                ..completed
            };
            assert!(
                practice_bars_are_closed(&[completed], source).is_ok(),
                "{source}"
            );
            assert!(
                practice_bars_are_closed(&[unfinished], source).is_err(),
                "{source}"
            );
        }
        let saturday_daily_label = Bar {
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-06T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 0.0,
        };
        assert!(practice_bars_are_closed(&[saturday_daily_label], "a_share").is_ok());
        assert!(source_candle_close_after("unknown").is_err());
    }

    #[test]
    fn completed_market_bars_keeps_the_requested_count_after_dropping_an_open_candle() {
        let now = Utc::now();
        for source in ["binance", "a_share", "us_stock"] {
            let close_after = source_candle_close_after(source).unwrap();
            let step = if source == "binance" {
                Duration::hours(1)
            } else {
                Duration::days(1)
            };
            let bars: Vec<Bar> = (0..6)
                .map(|index| Bar {
                    timestamp: now - close_after - step * (5 - index) + Duration::seconds(1),
                    open: 1.0,
                    high: 1.0,
                    low: 1.0,
                    close: 1.0,
                    volume: 1.0,
                })
                .collect();
            let selected = completed_market_bars(bars.clone(), source, now, 5).unwrap();
            assert_eq!(selected.len(), 5, "{source}");
            assert_eq!(selected[0].timestamp, bars[0].timestamp);
            assert_eq!(selected[4].timestamp, bars[4].timestamp);
            assert!(completed_market_bars(vec![bars[5]], source, now, 5).is_err());
        }
    }

    #[tokio::test]
    async fn catalog_fetches_both_public_endpoints_and_rejects_bad_responses() {
        let app = Router::new()
            .route("/api/v3/exchangeInfo", get(|| async {
                Json(json!({"symbols":[
                    {"symbol":"BTCUSDT","status":"TRADING","quoteAsset":"USDT","isSpotTradingAllowed":true},
                    {"symbol":"ETHBTC","status":"TRADING","quoteAsset":"BTC","isSpotTradingAllowed":true}
                ]}))
            }))
            .route("/api/v3/ticker/24hr", get(|| async {
                Json(json!([{"symbol":"BTCUSDT","quoteVolume":"1000"}]))
            }));
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        assert_eq!(
            fetch_symbols_from_binance_at(&base).await.unwrap(),
            vec!["BTCUSDT"]
        );
        server.abort();

        let bad = Router::new().route(
            "/api/v3/exchangeInfo",
            get(|| async { Json(json!({"symbols": "malformed"})) }),
        );
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, bad).await.unwrap();
        });
        assert!(fetch_symbols_from_binance_at(&base).await.is_err());
        server.abort();
    }

    #[test]
    fn catalog_keeps_only_tradable_spot_usdt_pairs_in_volume_order() {
        let symbols: Vec<BinanceSymbol> = serde_json::from_value(serde_json::json!([
            {"symbol":"LOWUSDT","status":"TRADING","quoteAsset":"USDT","isSpotTradingAllowed":true},
            {"symbol":"HIGHUSDT","status":"TRADING","quoteAsset":"USDT","isSpotTradingAllowed":true},
            {"symbol":"NOLIQUSDT","status":"TRADING","quoteAsset":"USDT","isSpotTradingAllowed":true},
            {"symbol":"HALTEDUSDT","status":"BREAK","quoteAsset":"USDT","isSpotTradingAllowed":true},
            {"symbol":"FUTUREUSDT","status":"TRADING","quoteAsset":"USDT","isSpotTradingAllowed":false},
            {"symbol":"ETHBTC","status":"TRADING","quoteAsset":"BTC","isSpotTradingAllowed":true}
        ])).unwrap();
        let tickers: Vec<BinanceTicker> = serde_json::from_value(serde_json::json!([
            {"symbol":"LOWUSDT","quoteVolume":"100"},
            {"symbol":"HIGHUSDT","quoteVolume":"1000"},
            {"symbol":"NOLIQUSDT","quoteVolume":"NaN"}
        ]))
        .unwrap();
        assert_eq!(
            select_binance_symbols(symbols, tickers),
            vec!["HIGHUSDT", "LOWUSDT", "NOLIQUSDT"]
        );
    }
}
