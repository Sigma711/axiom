# AXIOM

> 从公理出发，推导出你自己的市场观。

![tests](https://img.shields.io/badge/tests-35%2F35-brightgreen)
![strategies](https://img.shields.io/badge/strategies-14-orange)
![indicators](https://img.shields.io/badge/indicators-100%2B-blue)
![license](https://img.shields.io/badge/license-MIT-green)
![ci](https://img.shields.io/badge/CI-passing-brightgreen)

一个面向零基础到进阶的**量化交易教学工程**。**后端 Rust + axum**，**前端 HTML/JS**（计划迁到 React+TypeScript）。  
不是教人变成 trader，是教人**看懂 trader 在做什么**。

---

## 快速开始

```bash
git clone git@github.com:Sigma711/axiom.git
cd axiom

# 装 Rust 工具链 (1.70+): https://rustup.rs
cargo build --release
cargo run --release
# → 打开 http://localhost:8080
```

> **Windows 用户**：如果你装 Rust MSVC 工具链遇到链接错误，推荐用 WSL：
> ```bash
> wsl -d Ubuntu
> sudo apt install -y rustc cargo build-essential pkg-config libssl-dev
> cd /mnt/c/Users/<you>/axiom
> cargo run --release
> ```

---

## 这个工程涵盖什么

### 5 个标签页（HTML/JS 前端）

| 标签 | 功能 |
|---|---|
| **学习中心** | 4 个子页：📖 指标大全（169 个 PDF 概念，每条带 GitHub 源码深链接）/ 📚 概念速览 / 🛠 创建策略 5 步教学 / 🎓 学习路径 |
| **数据探索** | 选币种（Binance 493 个 USDT 现货对）+ 选数据源（真实/合成）+ 选 K 线图类型（标准/Heikin Ashi）+ 叠加指标（15 种）+ 形态识别（5 种：十字星/锤子线/流星等）|
| **回测** | 14 个策略 + 自定义止损/止盈/仓位 + 12 个业绩指标（总收益/夏普/索提诺/Calmar/VaR/CVaR/最大回撤/偏度/峰度等）|
| **模拟盘** | 实时跑策略（WebSocket 推送，5 秒一次）+ 7 个状态卡（现金/持仓/净值/价格等）+ 完整日志 |
| **策略对比** | 9 种预制策略自由勾选 + 自定义策略表单（JSON 参数） + 对比表 + 叠加净值图 |

### 14 个交易策略（src/strategy.rs）

`买入持有` · `双均线交叉` · `RSI` · `随机(冒烟)` · `MACD` · `Bollinger` · `Supertrend` · `Donchian 海龟` · `VWAP 回归` · `KDJ` · `Ichimoku 一目均衡表` · `PPO` · `Vortex` · `Elder Ray` · `随机`

### 100+ 个技术指标（src/indicators/）

| 模块 | 指标 |
|---|---|
| `ma.rs` 均线 | SMA, EMA, WMA, RMA, HMA, DEMA, TEMA, VWMA, BBI, BIAS, Alligator 三线 |
| `trend.rs` 趋势 | MACD, DMI/ADX, Aroon, Parabolic SAR, Supertrend, Donchian, Keltner, **Ichimoku 云图**, Vortex, ZigZag, Fractal, MA Envelope, Linear Regression, PP&O |
| `momentum.rs` 动量 | RSI, **Stochastic 完整版**, KDJ (中国经典), **StochRSI**, CCI, Williams %R, MTM, ROC, CMO, TSI, **Ultimate Oscillator**, KST, DPO, **Fisher Transform**, RVI, DeMarker, **Awesome/Accelerator Oscillator** (Bill Williams), Elder Ray, Balance of Power, Coppock, **PSY/ARBR/CR/OSC** (中国市场) |
| `volatility.rs` 波动率 | True Range, ATR, ATR%, HV, Bollinger Bands, **BBands-Keltner Squeeze**, Keltner, Donchian (同 trend), ADR, Chaikin Vol, Mass Index, Ulcer Index |
| `volume.rs` 成交量 | OBV, ADL, CMF, MFI, **VWAP**, **Anchored VWAP**, Chaikin Osc, PVT, Force Index, EMV/EOM, Klinger, NVI, PVI, Vol Osc, VROC, VR, WVAD |
| `statistics.rs` 统计 | Linear Regression, Pearson Correlation, **Rolling Correlation**, Z-Score, Beta, Alpha, Percentile Rank, Skewness, Kurtosis |
| `extra.rs` 额外 | **Heikin Ashi 平滑 K 线**, **5 种 K 线形态识别** (Doji/Hammer/Shooting Star/Marubozu/Engulfing), Z-Score |
| `fundamental.rs` 基本面 | **Black-Scholes 期权定价 + 完整 Greeks (Delta/Gamma/Theta/Vega/Rho)**, IV Rank, IV Percentile, Put/Call Ratio, **DuPont 分解**, ROE, ROA, Altman Z, Piotroski F-Score, Beneish M-Score, Accrual Ratio, 各类估值比率 (PE/PB/PS/PEG/EV-EBITDA) |
| `breadth.rs` 市场宽度 | ADL 累积派发线, TRIN Arms Index, **McClellan Oscillator**, New High/Low 比, **Breadth Thrust 10 日跳变检测** |
| `shareholder.rs` 股东 | F-Score, M-Score, 商誉/净资产比, 股权质押率, 解禁市值, 做空比例, Days to Cover, **Piotroski 9 因子质量评分** |

### 13 个业绩指标（src/metrics.rs）

总收益 · 年化收益 · 波动率 · **夏普** · **索提诺** · **Calmar** · 信息比率 · Treynor · 最大回撤 · 跟踪误差 · 捕获率 · **VaR (95%)** · **CVaR (95%)** · 偏度 · 峰度

---

## 架构

```
                          ┌─────────────────┐
                          │   HTTP / WS      │
                          │   axum 0.7      │
                          └────────┬────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
              ┌─────▼─────┐ ┌──────▼──────┐ ┌─────▼─────┐
              │ /api/...  │ │ /api/paper  │ │ /static   │
              │  REST     │ │  + WebSocket│ │  HTML/JS  │
              └─────┬─────┘ └──────┬──────┘ └───────────┘
                    │              │
                    │         ┌────▼────┐
                    │         │ Paper   │ ←  async loop (5s poll)
                    │         │ State   │
                    │         └────┬────┘
                    │              │
                    └──────────────┼──────────────┐
                                   │              │
                         ┌─────────▼────────┐  ┌───▼────┐
                         │ BacktestEngine / │  │ Binance │
                         │    策略        │  │  API    │
                         └─────────┬────────┘  └────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                    │
        ┌─────▼─────┐        ┌──────▼──────┐      ┌─────▼─────┐
        │  Strategy │        │  Portfolio  │      │  Broker  │
        │  (trait) │        │  + Risk     │      │ (trait)  │
        └─────┬─────┘        └──────┬──────┘      └─────┬─────┘
              │                    │                    │
              └────────────────────┼────────────────────┘
                                   │
                         ┌─────────▼────────┐
                         │    Indicators    │
                         │   (100+ 指标)     │
                         └──────────────────┘
```

**设计原则**：
- **Strategy 只产生 Signal，不直接下单** — 信号和执行解耦
- **Broker 是 seam** — 同一接口后面能换不同实现（SimulatedBroker 回测 / SimulatedBroker 模拟盘 / 实盘待扩展 ccxt）
- **Portfolio 负责仓位大小** — 策略说"买"，组合决定"买多少"
- **Risk 是兜底** — 不管策略怎么说，风控说了算
- **DataFeed trait** — 同一接口可换 Binance / CSV / 合成数据

---

## 学习路径

打开 `http://localhost:8080` 后,建议从**学习中心**开始:

1. **📖 指标大全** —— 169 个 PDF 概念,每个都带 GitHub 源码深链接
2. **📚 概念速览** —— 完整量化交易流程图
3. **🛠 创建策略** —— 5 步教学 + 6 个常见陷阱
4. **🎓 学习路径** —— 源码阅读顺序

然后进实战:

5. **📈 数据探索** —— 选个币,选指标叠加
6. **🔬 回测** —— 选个策略,跑 500 根 K 线
7. **⚖️ 策略对比** —— 勾几个 PK 一下
8. **📡 模拟盘** —— 实时跑,看推送

---

## 目录结构

```
axiom/
├── Cargo.toml              依赖定义
├── LICENSE                 MIT
├── README.md               本文件
├── .gitignore
├── .github/workflows/      GitHub Actions CI
│   └── rust.yml            cargo test/clippy
├── src/
│   ├── main.rs             启动入口
│   ├── lib.rs              模块声明
│   ├── api.rs              REST + WebSocket handlers
│   ├── app_state.rs
│   ├── config.rs           YAML 配置
│   ├── data.rs             K 线数据源
│   ├── strategy.rs         14 个策略
│   ├── broker.rs           模拟券商
│   ├── portfolio.rs        仓位管理
│   ├── risk.rs             风控
│   ├── engine.rs           回测引擎
│   ├── metrics.rs          13 个业绩指标
│   ├── paper.rs            模拟盘 (后台 tokio 任务)
│   ├── knowledge.rs        169 个 PDF 概念字典
│   ├── types.rs            共享数据类型
│   └── indicators/
│       ├── mod.rs
│       ├── ma.rs           移动平均
│       ├── trend.rs        趋势 (含 Ichimoku 云图)
│       ├── momentum.rs     动量 (含 KDJ)
│       ├── volatility.rs   波动率 (含 BBands Squeeze)
│       ├── volume.rs       成交量
│       ├── statistics.rs   统计
│       ├── extra.rs        Heikin Ashi + 形态识别
│       ├── fundamental.rs  基本面 + DuPont + Altman Z
│       ├── options.rs      Black-Scholes + Greeks
│       ├── breadth.rs      市场宽度 (ADL/TRIN/McClellan)
│       └── shareholder.rs F-Score + M-Score
├── static/
│   ├── index.html
│   ├── app.js
│   └── style.css
├── tests/
│   └── integration_test.rs
├── examples/               Rust 可执行示例
└── scripts/                Python 测试脚本
```

---

## 端点 (REST + WebSocket)

| 端点 | 说明 |
|---|---|
| `GET /` | 前端首页 |
| `GET /api/strategies` | 14 个策略元数据(参数+默认值) |
| `GET /api/symbols` | 493 个 Binance USDT 交易对(按成交量排序,5 分钟缓存) |
| `GET /api/data?symbol=X&limit=N&source=real` | K 线数据 |
| `GET /api/indicators?symbol=X&indicators=macd,bbands_20,zscore` | 指标数据(可多个) |
| `GET /api/patterns?symbol=X&limit=N` | 最近 K 线形态识别 |
| `GET /api/heikin_ashi?symbol=X&limit=N` | Heikin Ashi 平滑 K 线 |
| `POST /api/backtest` | 跑回测,返回完整结果 |
| `GET /api/paper/snapshot` | 模拟盘当前状态 |
| `POST /api/paper/start` / `/stop` / `/strategy` | 模拟盘控制 |
| `WS /api/paper/ws` | 模拟盘 WebSocket 推送 (2s 一次) |
| `GET /api/knowledge` | 169 个 PDF 概念字典 |

---

## 测试

```bash
cargo test              # 35 个测试 (24 单元 + 11 集成)
cargo build --release   # release 构建
cargo clippy            # lint 检查
```

GitHub Actions (`.github/workflows/rust.yml`):
- 每次 push/PR 自动跑 `cargo test`
- 缓存 cargo registry 加速
- 检查 `cargo fmt` 和 `cargo clippy`

---

## 路线图

- [x] 14 个策略 (买入持有/均线/RSI/MACD/BBands/Supertrend/海龟/VWAP/KDJ/Ichimoku/PPO/Vortex/Elder Ray)
- [x] 100+ 技术指标 + K线形态识别 + Heikin Ashi
- [x] 13 个业绩指标(夏普/索提诺/Calmar/VaR/CVaR 等)
- [x] 169 个 PDF 概念字典,带 GitHub 源码深链接,**0 个 placeholder**
- [x] GitHub Actions CI (cargo test/clippy)
- [x] **前端迁移到 React + TypeScript** (`web/` 目录, Vite 构建)
- [ ] 实盘接入 (ccxt / Binance official SDK)
- [ ] 多交易所支持 (OKX, Bybit, Coinbase)
- [ ] 协整 / Hurst 等统计套利指标

---

## License

MIT — 详见 [LICENSE](LICENSE)。

**仅供学习，不构成任何投资建议。量化有风险，实盘需谨慎。**
