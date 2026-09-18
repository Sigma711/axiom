# AXIOM

> 从公理出发，推导出你自己的市场观。

![tests](https://img.shields.io/badge/tests-38%2F38-brightgreen)
![strategies](https://img.shields.io/badge/strategies-14-orange)
![indicators](https://img.shields.io/badge/indicators-100%2B-blue)
![knowledge](https://img.shields.io/badge/knowledge-169-purple)
![license](https://img.shields.io/badge/license-MIT-green)
![frontend](https://img.shields.io/badge/frontend-React+TypeScript-61dafb)

面向零基础到进阶的量化交易教学工程。**后端 Rust + axum**，**前端 React + TypeScript**。

## 快速开始

```bash
git clone git@github.com:Sigma711/axiom.git
cd axiom

# 后端需要 Rust 1.70+, 前端需要 Node 18+
cargo --version; node --version

# 1. 后端 (8080)
cargo run --release

# 2. 前端 (开发用 5173, 生产构建输出到 ../static/)
cd web
npm install
npm run build   # 构建产物在 ../static/

# 打开浏览器
open http://localhost:8080
```

## 路线图

### 已完成 ✅

- **后端核心**：Rust + axum + tokio + WebSocket
- **策略**：14 个（买入持有 / 均线交叉 / RSI / MACD / BBands / Supertrend / Donchian / VWAP / KDJ / Ichimoku / PPO / Vortex / Elder Ray / 随机）
- **指标**：100+ 个（均线 / 趋势 / 动量 / 波动率 / 成交量 / 统计 / 基本面 / 期权 / 宽度 / 股东 / 扩展）
- **业绩指标**：13 个（夏普 / 索提诺 / Calmar / VaR / CVaR / 最大回撤 / 胜率 / 盈亏比 …）
- **风险模型**：固定仓位 / 波动率目标 / Kelly 部分 / 最大持仓限制 / 最大回撤止损
- **知识库**：169 个 PDF 概念 entry，**0 个 placeholder**
  - 每个 entry 含: id / 类别 / 名称 / 摘要 / 公式 / 含义 / 例子 / 信号 / 误区 / 代码深链接
  - `code_ref` 字段：`src/path.rs::function` 格式（稳定引用，不随行号变化）
  - `diagram` 字段：关键概念提供 ASCII 图示（RSI / MACD / Bollinger / Ichimoku / KDJ / OBV / Sharpe / Drawdown）
  - **集成测试 `tests/knowledge_test.rs`** 自动校验：占位符检测 + 文件路径校验 + 图示覆盖
- **数据源**：Binance 公开 K 线 API（无需 key，REST）
- **前端**：React 18 + TypeScript 5 + Vite 5，构建产物输出到 `static/`，Rust 后端直接服务
  - 5 个标签页：行情 / 回测 / 对比 / 模拟盘 / 知识库
  - 自研 `<AxDropdown>` 组件，避开原生 `<select>` 弹层被遮挡问题
  - 知识库面板支持搜索 / 分类筛选 / 图示展示
- **CI**：GitHub Actions（cargo test / clippy）

### 未完成 📋

- 实盘接入（ccxt / Binance 官方 SDK）
- 多交易所支持（OKX / Bybit / Coinbase）
- 协整 / Hurst 指数 / Ornstein-Uhlenbeck 等统计套利指标
- 单元测试覆盖率提升（当前仅核心逻辑覆盖）
- Docker 镜像打包
- 用户认证 / 多账户管理

## 项目结构

```
axiom/
├── src/                     # Rust 后端
│   ├── knowledge.rs         # 169 个知识库 entry
│   ├── strategy.rs          # 14 个策略
│   ├── indicators/          # 11 个指标模块
│   ├── metrics.rs / risk.rs # 业绩与风险
│   ├── engine.rs            # 回测引擎
│   ├── paper.rs             # 模拟盘
│   ├── api.rs               # HTTP API
│   └── diagrams.rs          # ASCII 图示库
├── web/                     # React + TypeScript 前端
│   ├── src/
│   │   ├── App.tsx          # 主应用（5 个标签页）
│   │   ├── types.ts         # 类型定义
│   │   ├── api.ts           # API 客户端
│   │   └── styles.css       # 样式
│   ├── vite.config.ts       # 输出到 ../static/
│   └── package.json
├── tests/
│   ├── integration_test.rs  # 端到端测试
│   └── knowledge_test.rs    # 知识库自动校验
├── static/                  # 前端构建产物（gitignore）
└── .github/workflows/       # CI
```

## 开发

```bash
# 运行所有测试 (38 个)
cargo test

# 仅跑知识库校验
cargo test --test knowledge_test -- --nocapture

# 前端热更新
cd web && npm run dev

# 后端日志
RUST_LOG=info cargo run
```

## License

MIT - 详见 [LICENSE](LICENSE)。

> ⚠️ **风险提示**：本项目仅供学习，不构成任何投资建议。量化有风险，实盘需谨慎。