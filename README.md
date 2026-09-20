# AXIOM

从公理出发，推导你的市场观。Rust + React 的量化学习工作台，包含知识大全、数据探索、回测、模拟盘和策略对比。

## 启动与开发

主分支为 `main`。使用稳定版 Rust（rustup）、Node.js 24 和 GNU Make；WSL/Linux 为持续集成验证环境。

```sh
git clone git@github.com:Sigma711/axiom.git
cd axiom
make setup
make browser-deps  # 首次运行浏览器测试时安装系统依赖
make serve        # 构建前后端并在 http://localhost:8080 启动
```

所有构建、检查和测试统一通过 Make：

| 命令 | 用途 |
| --- | --- |
| `make dev` | 前端热更新（后端另用 `make run-debug`） |
| `make build` | 前端静态资源和 Rust release 构建 |
| `make build-debug` / `make run-debug` | 构建 / 运行开发版后端 |
| `make fmt` | 格式化 Rust |
| `make lint` | 格式、严格 Clippy 和 TypeScript 检查 |
| `make test` | 全部 Rust、前端单元、浏览器与真实服务端到端测试 |
| `make test-one TEST=broker_invariants_test` | 一个 Rust 集成测试套件 |
| `make test-web` | 前端类型和单元测试 |
| `make test-e2e` | 浏览器交互、异常状态、主题与图表测试 |
| `make test-e2e-real` | 启动隔离的真实 Rust 服务并从浏览器操作 |
| `make test-visual` | 两组浏览器视觉检查 |
| `make ci` | 完整质量门禁：格式、lint、构建和所有测试 |
| `make tools` / `make coverage` | 安装覆盖率工具 / 生成 LCOV 与 HTML |

`AXIOM_PORT=8081 make serve` 可更换端口。测试服务固定使用 18080，前端测试使用 18181，与日常 8080 服务隔离。生成的 `static/` 资源不进入 Git；从干净检出启动时必须先构建，避免旧页面与新接口混用。

## 知识、原书与实践

`docs/book/source-manifest.json` 记录用户提供的《股票交易软件专业指标全解_完整版.pdf》的目录、正文编号条目、附录和 TradingView 清单；其余映射文件将来源关联到可执行概念。`GET /api/knowledge/coverage` 返回覆盖矩阵、页码和源文件摘要。清单包含别名和重复条目，因此来源条目数量不等于独立概念数量。

知识与实践共享同一个注册表。每个概念都有公式、含义、输入示例、结果图解、使用边界和代码引用；四个操作模块均可选择全部概念。市场指标使用当前模块明确显示的数据快照；财务、期权、链上等需要外部数据的概念使用可编辑且明确标注的教学输入，不从价格 K 线伪造财报或链上数据。专有指标仅核验导入信号的可观察条件，资料未公开的算法不会伪称复现。

代码定位由 Rust AST 在构建时生成。发布版本链接到 **GitHub 对应提交的精确实现行范围**，不把行号写死在知识条目里。代码移动后，重新构建会重新定位；旧版本链接固定到旧提交。未提交的开发版本提供嵌入源码预览，以免把尚未发布的代码误链到 GitHub 的旧行；交付使用提交后的干净构建。

## 计算与交易约定

- 小时 K 线仅使用已收盘数据；HTTP 数据支持分页、严格 OHLCV/时间校验及本地 CSV 缓存。
- 策略在当前 K 线收盘产生意图，最早在下一根 K 线开盘成交。末根信号不会偷偷按同根收盘价成交。
- 回测和模拟盘使用一致的交易、仓位、费用及风控逻辑。暂停、切换策略和重复行情均有回归测试。
- 策略对比复用同一份行情快照与初始资金；合成数据可复现，真实数据明确标出来源。
- 收益指标计入手续费，年化频率来自实际时间间隔。样本不足、零波动、没有已平仓交易等导致的未定义结果返回 `null` 和原因，在页面显示 `—`，不冒充零值。
- 图表区分价格、百分数、资金和无量纲序列，保留预热期空值；所有时间序列指标都应避免未来数据影响历史结果。

## 测试与 CI

GitHub Actions 在 `main` 推送和 PR 上执行 `make ci`、`make coverage`，失败会阻止门禁通过，浏览器证据和覆盖率报告作为构建产物保存。测试包括独立手算数值、异常/边界输入、时序因果性、账本守恒、成交费用、行情分页、模拟盘生命周期、真实 HTTP/WebSocket，以及从浏览器操作四个模块。知识覆盖测试验证来源映射、知识与实践目录一致、每项代码引用和所有概念在四个模块的执行契约。

浏览器检查包括桌面/窄屏、浅色/深色、键盘与鼠标交互、图表数据/坐标/可读性、展开收起状态和代码链接。自动检查用于防止已知错误回归；视觉质量还需要结合实际截图审阅。测试并不构成交易盈利或外部市场数据永远可用的保证。

## 目录

- `src/indicators/`、`src/strategy.rs`：指标与策略。
- `src/practice/`、`src/book*`、`src/supplement.rs`、`src/workflows.rs`：可执行教学内容和原书映射。
- `src/engine.rs`、`src/broker.rs`、`src/portfolio.rs`、`src/paper.rs`：交易和账户状态。
- `build_support/source_map.rs`、`src/code_links.rs`：语义代码定位。
- `web/src/`、`web/e2e/`：界面与浏览器测试。
- `tests/`：Rust 公共接口测试；`coverage/`：生成的覆盖率报告。

MIT，见 [LICENSE](LICENSE)。仅供学习，不构成投资建议；模拟盘不会发送真实交易所订单。
