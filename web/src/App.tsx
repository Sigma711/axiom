import { useState, useEffect, useCallback, useRef } from 'react';
import { api, fmtPct, fmtNum, fmtMoney, fmtPctSigned } from './api';
import type {
  TabId, SourceType, Bar, BacktestResult, StrategyMeta,
  KnowledgeResponse, KnowledgeEntry, CustomStrategy, ChartType,
} from './types';

// ===================================================================
// 顶栏
// ===================================================================
function Header() {
  return (
    <header className="ax-header">
      <div className="ax-header-inner">
        <div className="ax-brand">
          <span className="ax-logo">◆</span>
          <span className="ax-title">AXIOM</span>
          <span className="ax-sub">从公理出发,推导你的市场观</span>
        </div>
      </div>
    </header>
  );
}

// ===================================================================
// 标签栏
// ===================================================================
const TABS: { id: TabId; label: string }[] = [
  { id: 'learn', label: '学习中心' },
  { id: 'data', label: '数据探索' },
  { id: 'backtest', label: '回测' },
  { id: 'paper', label: '模拟盘' },
  { id: 'compare', label: '策略对比' },
];

function TabBar({ active, onChange }: { active: TabId; onChange: (t: TabId) => void }) {
  return (
    <nav className="ax-tabs">
      <div className="ax-tabs-inner">
        {TABS.map(t => (
          <button
            key={t.id}
            className={'ax-tab' + (active === t.id ? ' active' : '')}
            onClick={() => onChange(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
    </nav>
  );
}

// ===================================================================
// 通用:下拉选择器
// ===================================================================
interface DropdownProps<T> {
  options: { v: T; l: string }[];
  value: T;
  onChange: (v: T) => void;
  minWidth?: number;
}
function Dropdown<T extends string | number>({ options, value, onChange, minWidth = 100 }: DropdownProps<T>) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const current = options.find(o => o.v === value) || options[0];

  return (
    <div className="ax-dropdown" ref={ref} style={{ minWidth }}>
      <button className="ax-dd-trigger" onClick={() => setOpen(o => !o)}>
        <span>{current?.l || '选择...'}</span>
        <span className="ax-dd-arrow">▾</span>
      </button>
      {open && (
        <div className="ax-dd-menu">
          {options.length > 5 && (
            <input className="ax-dd-search" placeholder="搜索..." autoFocus
              onChange={e => {
                const q = e.target.value.toLowerCase();
                e.target.dataset.filtered = q;
                const items = (e.target.parentElement?.querySelectorAll('.ax-dd-item') || []) as NodeListOf<HTMLElement>;
                items.forEach(it => {
                  const label = (it.textContent || '').toLowerCase();
                  (it as HTMLElement).style.display = label.includes(q) ? '' : 'none';
                });
              }}
            />
          )}
          {options.map(o => (
            <div
              key={String(o.v)}
              className={'ax-dd-item' + (o.v === value ? ' selected' : '')}
              onClick={() => { onChange(o.v); setOpen(false); }}
            >
              {o.l}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ===================================================================
// 学习中心: 4 个子标签
// ===================================================================
type LearnSub = 'knowledge' | 'concepts' | 'build' | 'path';
const LEARN_SUBS: { id: LearnSub; label: string }[] = [
  { id: 'knowledge', label: '指标大全' },
  { id: 'concepts', label: '概念速览' },
  { id: 'build', label: '创建策略' },
  { id: 'path', label: '学习路径' },
];

function LearnCenter() {
  const [sub, setSub] = useState<LearnSub>('knowledge');
  return (
    <section className="ax-section">
      <h2>学习中心</h2>
      <p className="ax-lead">从公理出发,推导出你自己的市场观。</p>
      <nav className="ax-learn-subnav">
        {LEARN_SUBS.map(s => (
          <button key={s.id}
            className={'ax-learn-sub' + (s.id === sub ? ' active' : '')}
            onClick={() => setSub(s.id)}>{s.label}</button>
        ))}
      </nav>
      {sub === 'knowledge' && <KnowledgeView />}
      {sub === 'concepts' && <ConceptsView />}
      {sub === 'build' && <BuildView />}
      {sub === 'path' && <PathView />}
    </section>
  );
}

// 指标大全
function KnowledgeView() {
  const [data, setData] = useState<KnowledgeResponse | null>(null);
  const [search, setSearch] = useState('');
  const [activeCat, setActiveCat] = useState('');

  useEffect(() => { api.listKnowledge().then(setData).catch(console.error); }, []);

  if (!data) return <div className="ax-loading">加载中…</div>;
  const cats = Object.keys(data.categories);
  const list = activeCat ? data.categories[activeCat] : cats.flatMap(c => data.categories[c]);
  const q = search.trim().toLowerCase();
  const filtered = q ? list.filter(e =>
    (e.name || '').toLowerCase().includes(q) ||
    (e.id || '').toLowerCase().includes(q) ||
    (e.summary || '').toLowerCase().includes(q) ||
    (e.meaning || '').toLowerCase().includes(q) ||
    (e.example || '').toLowerCase().includes(q)
  ) : list;

  return (
    <div>
      <div className="ax-search-bar">
        <input
          type="text"
          placeholder="搜索 概念 / 公式 / 关键词"
          value={search}
          onChange={e => setSearch(e.target.value)}
        />
      </div>
      <div className="ax-cat-pills">
        <button className={'ax-pill' + (!activeCat ? ' active' : '')} onClick={() => setActiveCat('')}>
          全部 ({data.total})
        </button>
        {cats.map(c => (
          <button key={c} className={'ax-pill' + (activeCat === c ? ' active' : '')}
            onClick={() => setActiveCat(c)}>
            {c} ({data.categories[c].length})
          </button>
        ))}
      </div>
      <p style={{ color: 'var(--text-dim)', fontSize: '0.82rem', margin: '0.4rem 0 0.8rem' }}>
        {filtered.length} 条结果
      </p>
      {filtered.map(e => <KbCard key={e.id} e={e} />)}
    </div>
  );
}

function KbCard({ e }: { e: KnowledgeEntry }) {
  return (
    <div className="ax-kb-card">
      <div className="ax-kb-header">
        <h4>{e.name}</h4>
        <a href={e.code_url} target="_blank" rel="noopener" className="ax-gh-btn">↗ GitHub</a>
      </div>
      <div className="ax-kb-summary">{e.summary}</div>
      <details>
        <summary className="ax-kb-details">展开详情</summary>
        <Section label="公式"><code>{e.formula || 'N/A'}</code></Section>
        <Section label="含义">{e.meaning}</Section>
        {e.example && <Section label="例子" highlight>{e.example}</Section>}
        <Section label="信号解读">{e.signals}</Section>
        <Section label="常见误区" danger>{e.pitfalls}</Section>
        {e.related && e.related.length > 0 && (
          <Section label="关联概念">
            {e.related.map((r, i) => (
              <span key={i} className="ax-tag">{r}</span>
            ))}
          </Section>
        )}
        <Section label="代码实现"><code className="ax-code-link">{e.implementation}</code></Section>
      </details>
    </div>
  );
}

function Section({ label, children, danger, highlight }: { label: string; children: React.ReactNode; danger?: boolean; highlight?: boolean }) {
  return (
    <div className={'ax-section-block' + (danger ? ' danger' : '') + (highlight ? ' highlight' : '')}>
      <div className="ax-section-label">{label}</div>
      <div>{children}</div>
    </div>
  );
}

// 概念速览
function ConceptsView() {
  return (
    <div>
      <h3>量化交易完整流程</h3>
      <p className="ax-lead">从原始行情到最终下单,一个量化系统走完下面 7 步。</p>
      <pre className="ax-flow-diagram">
{`              原始数据         指标计算           交易信号          风控过滤        仓位计算       模拟执行        业绩评估

  [OHLCV  K线] → [SMA/MACD/...] → [BUY/SELL/HOLD] → [止损/仓位] → [买多少] → [模拟券商] → [收益/回撤/夏普]
        ↑              ↑                 ↑                ↑             ↑            ↑              ↑
   Binance API     纯函数           只产生想法       防止黑天鹅    Kelly/固定      手续费/滑点      数字不撒谎
   CSV 文件      无副作用          不下单                          比例          与实盘接口一致    看穿策略`}
      </pre>
      <h3>每个概念对应一个代码模块</h3>
      <div className="ax-grid">
        {CONCEPT_CARDS.map(c => (
          <div className="ax-card" key={c.title}>
            <h4>{c.title}</h4>
            <p>{c.desc}</p>
            <code>{c.code}</code>
          </div>
        ))}
      </div>
    </div>
  );
}
const CONCEPT_CARDS = [
  { title: 'K线 (Bar)', desc: '一根柱子记录一段时间的开/高/低/收/成交量。', code: '{ ts, open, high, low, close, volume }' },
  { title: '信号 (Signal)', desc: '策略看到行情后的想法:BUY / SELL / HOLD。', code: '{ side, strength, reason }' },
  { title: '订单 (Order)', desc: '给券商的具体指令:方向、数量、价格。', code: '{ side, size, price, type }' },
  { title: '成交 (Fill)', desc: '订单实际被执行的回报,扣手续费和滑点。', code: '{ size, price, commission }' },
  { title: '持仓 (Position)', desc: '你现在持有多少某个标的,以及平均成本。', code: '{ size, avg_entry_price }' },
  { title: '净值 (Equity)', desc: '总身家:现金 + 持仓按当前价估值。', code: 'cash + position × price' },
  { title: '风控 (Risk)', desc: '止损 / 止盈 / 仓位上限 — 活下来比赚得多重要。', code: '{ stop_loss, take_profit, max_pos }' },
  { title: '指标 (Metrics)', desc: '总收益 / 最大回撤 / 夏普比率。', code: '{ total_return, max_dd, sharpe }' },
  { title: '回测 / 模拟盘 / 实盘', desc: '同一套策略,换不同券商 adapter,就能在不同阶段跑。', code: 'SimulatedBroker / HttpFeed' },
];

// 创建策略
function BuildView() {
  return (
    <div>
      <h3>创建自己的策略 - 5 步教学</h3>
      <p className="ax-lead">从"想赚钱"到"能稳定赚钱"需要走完下面 5 步。</p>
      <ol className="ax-path">
        {BUILD_STEPS.map((s, i) => (
          <li key={i}>
            <h4>{i + 1}. {s.title}</h4>
            <p>{s.desc}</p>
            {s.detail && <div className="ax-path-detail">{s.detail}</div>}
          </li>
        ))}
      </ol>
      <h3 style={{ marginTop: '2rem' }}>常见陷阱</h3>
      <div className="ax-grid">
        {PITFALLS.map(p => (
          <div className="ax-card danger" key={p.title}>
            <h4>{p.title}</h4>
            <p>{p.desc}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
const BUILD_STEPS = [
  { title: '形成可验证的市场假设', desc: '每个好策略背后都有一句可验证的话。', detail: '❌ "我觉得 BTC 要涨" — 不可验证\n✅ "BTC 在 20 日新高 + 成交量放大 → 接下来 5 天继续上涨概率 > 60%" — 可验证' },
  { title: '选择指标来验证假设', desc: '用指标验证假设,不是凭感觉猜。', detail: '趋势 → SMA/EMA/MACD\n超跌 → RSI/KDJ/Bollinger\n量能 → OBV/CMF/MFI\n突破 → Donchian/Bollinger' },
  { title: '定义无歧义信号', desc: '代码里写"价格跌很多"是行不通的。', detail: '✅ "RSI 从 < 30 反弹到 > 30 → BUY"\n✅ "SMA5 上穿 SMA20 → BUY"\n❌ "感觉要涨了"' },
  { title: '写代码', desc: '打开 src/strategy.rs,复制最像的策略骨架。', detail: '参考 SmaCrossStrategy 或 RsiStrategy 的实现' },
  { title: '测试和迭代', desc: '跑回测前问自己:样本外验证?参数来源?心理承受力?', detail: '夏普 > 1 合格,索提诺 > 1.5,最大回撤 < 20%,胜率 × 盈亏比 > 1' },
];
const PITFALLS = [
  { title: '低 PE ≠ 便宜', desc: '周期股 PE 低可能是利润即将下滑。' },
  { title: 'RSI 超买 ≠ 卖出', desc: '强趋势中 RSI 长期 > 70,反向交易会被反复止损。' },
  { title: '金叉 ≠ 领先信号', desc: '金叉是滞后信号,等它发生时趋势已走了一段。' },
  { title: '多个振荡器 ≠ 多重确认', desc: 'RSI/KDJ/Stochastic 本质都是动量指标。' },
  { title: '主力净流入 ≠ 真现金流入', desc: '只是统计主动买入单,可拆单伪装。' },
  { title: '放量 ≠ 看多', desc: '放量可发生在任何位置,需结合价格位置判断。' },
];

// 学习路径
function PathView() {
  return (
    <div>
      <h3>源码阅读顺序</h3>
      <p className="ax-lead">按这个顺序读代码,最快搞懂整个系统。</p>
      <ol className="ax-path">
        {[
          { title: 'src/types.rs', desc: '词汇表 (Bar/Order/Fill/Position/Trade)' },
          { title: 'src/broker.rs', desc: '模拟券商 (理解 seam 设计)' },
          { title: 'src/strategy.rs', desc: '策略 (理解"为什么只产生 Signal 不下单")' },
          { title: 'src/portfolio.rs + src/risk.rs', desc: '仓位与风控' },
          { title: 'src/engine.rs', desc: '主循环 (读懂了就懂整个事件流)' },
          { title: 'src/metrics.rs', desc: '13 个业绩指标' },
          { title: 'src/indicators/*.rs', desc: '100+ 指标库' },
          { title: 'src/api.rs', desc: 'REST + WebSocket handlers' },
        ].map((s, i) => (
          <li key={i}>
            <h4>{s.title}</h4>
            <p>{s.desc}</p>
          </li>
        ))}
      </ol>
    </div>
  );
}

// ===================================================================
// 数据探索
// ===================================================================
function DataExplore() {
  const [symbols, setSymbols] = useState<string[]>([]);
  const [symbol, setSymbol] = useState('');
  const [customSymbol, setCustomSymbol] = useState('');
  const [limit, setLimit] = useState(200);
  const [source, setSource] = useState<SourceType>('real');
  const [chartType, setChartType] = useState<ChartType>('candle');
  const [indicatorSet, setIndicatorSet] = useState('sma_20,rsi_14,bbands_20,macd,atr_14');
  const [chartData, setChartData] = useState<{ bars: Bar[]; indicators: Record<string, Array<{ x: string; y: number } | null>>; symbol: string; source: string } | null>(null);
  const [summary, setSummary] = useState<{ open: number; close: number; high: number; low: number; return_pct: number; avg_volume: number; count: number; source: string } | null>(null);
  const [patterns, setPatterns] = useState<Array<{ pattern: string; timestamp: string }>>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const chartRef = useRef<HTMLDivElement>(null);
  const plotlyRef = useRef<any>(null);

  useEffect(() => {
    api.listSymbols().then(d => {
      setSymbols(d.symbols);
      if (d.symbols.includes('BTCUSDT')) setSymbol('BTCUSDT');
      else if (d.symbols.length > 0) setSymbol(d.symbols[0]);
    }).catch(e => setError(String(e)));
  }, []);

  const loadData = useCallback(async () => {
    const sym = (customSymbol || symbol || 'BTCUSDT').toUpperCase();
    setLoading(true);
    setError('');
    try {
      const data = await api.getIndicators(sym, indicatorSet, limit, source);
      if (chartType === 'heikin_ashi') {
        try {
          const ha = await api.getHeikinAshi(sym, limit, source);
          if (ha.bars) data.bars = ha.bars;
        } catch {}
      }
      setChartData(data);

      // 摘要
      if (data.bars.length > 0) {
        const closes = data.bars.map(b => b.close);
        const first = closes[0], last = closes[closes.length - 1];
        const high = Math.max(...data.bars.map(b => b.high));
        const low = Math.min(...data.bars.map(b => b.low));
        setSummary({
          open: first, close: last, high, low,
          return_pct: (last / first - 1) * 100,
          avg_volume: data.bars.reduce((s, b) => s + b.volume, 0) / data.bars.length,
          count: data.bars.length,
          source: data.source,
        });
      }

      // 形态
      try {
        const pat = await api.getPatterns(sym, Math.min(limit, 100), source);
        setPatterns(pat.patterns.filter(p => p.pattern !== '无特殊形态').slice(0, 5));
      } catch {}
    } catch (e) {
      setError(String(e));
      setChartData(null);
    } finally {
      setLoading(false);
    }
  }, [symbol, customSymbol, limit, source, indicatorSet, chartType]);

  useEffect(() => { if (symbol) loadData(); }, [loadData, symbol]);

  // 画图
  useEffect(() => {
    if (!chartData) return;
    if (!chartRef.current) return;
    const Plotly = (window as any).Plotly;
    if (!Plotly) return;
    const traces: any[] = [{
      x: chartData.bars.map(b => new Date(b.timestamp)),
      open: chartData.bars.map(b => b.open),
      high: chartData.bars.map(b => b.high),
      low: chartData.bars.map(b => b.low),
      close: chartData.bars.map(b => b.close),
      type: 'candlestick', name: chartData.symbol,
      increasing: { line: { color: '#3fb950' } },
      decreasing: { line: { color: '#f85149' } },
    }];
    if (chartType === 'heikin_ashi') {
      traces[0].name = chartData.symbol + ' (HA)';
    }
    if (chartData.indicators) {
      const colors = ['#d29922', '#58a6ff', '#a371f7', '#ff7b72', '#56d4dd', '#7ee787'];
      let ci = 0;
      for (const [name, vals] of Object.entries(chartData.indicators)) {
        traces.push({
          x: vals.map((v: any) => v ? new Date(v.x) : null),
          y: vals.map((v: any) => v ? v.y : null),
          type: 'scatter', mode: 'lines', name,
          line: { color: colors[ci++ % colors.length], width: 1.5 },
          connectgaps: false,
        });
      }
    }
    Plotly.react(chartRef.current, traces, {
      paper_bgcolor: '#1c2128', plot_bgcolor: '#1c2128',
      font: { color: '#e6edf3', family: 'system-ui', size: 11 },
      margin: { t: 30, b: 40, l: 50, r: 20 },
      xaxis: { gridcolor: '#21262d' },
      yaxis: { gridcolor: '#21262d' },
      legend: { orientation: 'h', y: -0.15 },
    }, { responsive: true, displayModeBar: false });
  }, [chartData, chartType]);

  return (
    <section className="ax-section">
      <h2>数据探索</h2>
      <p className="ax-lead">真实 Binance 数据 + 指标叠加 + 形态识别。</p>
      <div className="ax-controls">
        <label>交易对
          <Dropdown options={symbols.map(s => ({ v: s, l: s }))} value={symbol} onChange={setSymbol} minWidth={120} />
        </label>
        <label>自定义
          <input type="text" value={customSymbol} onChange={e => setCustomSymbol(e.target.value)}
            placeholder="下拉里没有?手动输入" />
        </label>
        <label>K 线数
          <input type="number" value={limit} onChange={e => setLimit(parseInt(e.target.value) || 200)}
            min={50} max={2000} step={50} />
        </label>
        <label>数据源
          <Dropdown options={[{v:'real',l:'真实 (Binance)'},{v:'synthetic',l:'合成 (随机)'}]}
            value={source} onChange={(v: SourceType) => setSource(v)} minWidth={140} />
        </label>
        <label>图表类型
          <Dropdown options={[{v:'candle',l:'标准 K 线'},{v:'heikin_ashi',l:'Heikin Ashi'}]}
            value={chartType} onChange={(v: ChartType) => setChartType(v)} minWidth={140} />
        </label>
        <label>指标叠加
          <input type="text" value={indicatorSet} onChange={e => setIndicatorSet(e.target.value)} style={{ width: 220 }} />
        </label>
        <button className="ax-btn primary" onClick={loadData} disabled={loading}>
          {loading ? '加载中...' : '加载数据'}
        </button>
      </div>
      {error && <div className="ax-error">{error}</div>}
      <div ref={chartRef} className="ax-chart"></div>
      {summary && (
        <div className="ax-summary">
          <table>
            <tbody>
              <tr><th>币种</th><td>{chartData?.symbol}</td><th>来源</th><td>{summary.source}</td></tr>
              <tr><th>K 线数</th><td>{summary.count}</td><th>首价</th><td>{fmtNum(summary.open)}</td></tr>
              <tr><th>末价</th><td>{fmtNum(summary.close)}</td><th>累计涨跌</th>
                <td style={{ color: summary.return_pct >= 0 ? 'var(--green)' : 'var(--red)' }}>
                  {summary.return_pct.toFixed(2)}%
                </td>
              </tr>
              <tr><th>期间最高</th><td>{fmtNum(summary.high)}</td><th>期间最低</th><td>{fmtNum(summary.low)}</td></tr>
              <tr><th>平均成交量</th><td>{fmtNum(summary.avg_volume)}</td><th></th><td></td></tr>
            </tbody>
          </table>
        </div>
      )}
      {patterns.length > 0 && (
        <div className="ax-patterns">
          <span style={{ color: 'var(--text-dim)', fontSize: '0.82rem' }}>最近形态:</span>
          {patterns.map((p, i) => (
            <span key={i} className="ax-pattern-tag">{p.pattern} · {new Date(p.timestamp).toLocaleDateString()}</span>
          ))}
        </div>
      )}
    </section>
  );
}

// ===================================================================
// 回测
// ===================================================================
function Backtest() {
  const [strategies, setStrategies] = useState<StrategyMeta[]>([]);
  const [strategy, setStrategy] = useState('');
  const [params, setParams] = useState<Record<string, number>>({});
  const [symbol, setSymbol] = useState('');
  const [symbols, setSymbols] = useState<string[]>([]);
  const [source, setSource] = useState<SourceType>('real');
  const [limit, setLimit] = useState(500);
  const [capital, setCapital] = useState(10000);
  const [sl, setSl] = useState(0);
  const [tp, setTp] = useState(0);
  const [mp, setMp] = useState(95);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<BacktestResult | null>(null);
  const [error, setError] = useState('');
  const chartRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.listStrategies().then(d => {
      setStrategies(d.strategies);
      if (d.strategies.length > 0) {
        setStrategy(d.strategies[0].name);
        const p: Record<string, number> = {};
        d.strategies[0].params.forEach((x: any) => p[x.key] = x.default);
        setParams(p);
      }
    });
    api.listSymbols().then(d => {
      setSymbols(d.symbols);
      setSymbol('BTCUSDT');
    });
  }, []);

  // 切策略时加载默认参数
  useEffect(() => {
    if (!strategy) return;
    const m = strategies.find(s => s.name === strategy);
    if (m) {
      const p: Record<string, number> = {};
      m.params.forEach((x: any) => p[x.key] = x.default);
      setParams(p);
    }
  }, [strategy, strategies]);

  const run = async () => {
    if (!strategy) return;
    setLoading(true);
    setError('');
    try {
      const r = await api.runBacktest({
        strategy, params, symbol, source, limit, initial_capital: capital,
        stop_loss_pct: sl / 100, take_profit_pct: tp / 100, max_position_pct: mp / 100,
      });
      setResult(r);
    } catch (e) {
      setError(String(e));
      setResult(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!result || !chartRef.current) return;
    const Plotly = (window as any).Plotly;
    if (!Plotly) return;
    Plotly.react(chartRef.current, [{
      x: result.equity_curve.map(p => new Date(p.timestamp)),
      y: result.equity_curve.map(p => p.equity),
      type: 'scatter', mode: 'lines', name: '净值',
      line: { color: '#d29922', width: 1.8 },
      fill: 'tozeroy', fillcolor: 'rgba(210, 153, 34, 0.05)',
    }], {
      paper_bgcolor: '#1c2128', plot_bgcolor: '#1c2128',
      font: { color: '#e6edf3', family: 'system-ui', size: 11 },
      margin: { t: 30, b: 40, l: 60, r: 20 },
      xaxis: { gridcolor: '#21262d' },
      yaxis: { gridcolor: '#21262d', title: '净值 ($)' },
    }, { responsive: true, displayModeBar: false });
  }, [result]);

  const currentMeta = strategies.find(s => s.name === strategy);

  return (
    <section className="ax-section">
      <h2>回测</h2>
      <p className="ax-lead">在真实 Binance 数据上跑策略,看业绩指标。</p>
      <div className="ax-controls">
        <label>策略
          <Dropdown options={strategies.map(s => ({ v: s.name, l: s.display_name }))}
            value={strategy} onChange={setStrategy} minWidth={200} />
        </label>
        <label>交易对
          <Dropdown options={symbols.map(s => ({ v: s, l: s }))}
            value={symbol} onChange={setSymbol} minWidth={120} />
        </label>
        <label>数据源
          <Dropdown options={[{v:'real',l:'真实 (Binance)'},{v:'synthetic',l:'合成 (随机)'}]}
            value={source} onChange={(v: SourceType) => setSource(v)} minWidth={140} />
        </label>
        <label>K 线数
          <input type="number" value={limit} onChange={e => setLimit(parseInt(e.target.value) || 500)} min={50} max={2000} step={50} />
        </label>
        <label>资金
          <input type="number" value={capital} onChange={e => setCapital(parseFloat(e.target.value) || 10000)} step={1000} />
        </label>
        <label>止损 %
          <input type="number" value={sl} onChange={e => setSl(parseFloat(e.target.value) || 0)} step={0.5} min={0} max={50} />
        </label>
        <label>止盈 %
          <input type="number" value={tp} onChange={e => setTp(parseFloat(e.target.value) || 0)} step={0.5} min={0} max={200} />
        </label>
        <label>仓位 %
          <input type="number" value={mp} onChange={e => setMp(parseFloat(e.target.value) || 95)} step={1} min={1} max={100} />
        </label>
        <button className="ax-btn primary" onClick={run} disabled={loading}>
          {loading ? '运行中...' : '运行回测'}
        </button>
      </div>
      {currentMeta && currentMeta.params.length > 0 && (
        <div className="ax-controls">
          {currentMeta.params.map(p => (
            <label key={p.key}>{p.label}
              <input type="number" value={params[p.key] ?? p.default}
                onChange={e => setParams({...params, [p.key]: parseFloat(e.target.value) || p.default})}
                min={p.min} max={p.max} step={0.1} />
            </label>
          ))}
        </div>
      )}
      {error && <div className="ax-error">{error}</div>}
      {result && (
        <>
          <div className="ax-metrics">
            {METRIC_FIELDS.map(f => {
              const v = result.metrics[f.key];
              let cls = '';
              if (f.sign && typeof v === 'number') {
                cls = v > 0 ? 'positive' : v < 0 ? 'negative' : '';
              }
              return (
                <div className="ax-metric" key={f.key}>
                  <div className="ax-metric-label">{f.label}</div>
                  <div className={'ax-metric-value ' + cls}>{f.fmt(v)}</div>
                </div>
              );
            })}
          </div>
          <div ref={chartRef} className="ax-chart"></div>
          {result.trades.length > 0 && (
            <details className="ax-trades">
              <summary>交易记录 ({result.trades.length} 笔)</summary>
              <table>
                <thead><tr><th>开仓</th><th>方向</th><th>开仓价</th><th>数量</th><th>平仓</th><th>平仓价</th><th>盈亏</th><th>收益率</th></tr></thead>
                <tbody>
                  {result.trades.slice(-20).map((t, i) => (
                    <tr key={i}>
                      <td>{new Date(t.entry_time).toLocaleString()}</td>
                      <td>{t.side}</td>
                      <td>{fmtNum(t.entry_price)}</td>
                      <td>{fmtNum(t.size, 4)}</td>
                      <td>{t.exit_time ? new Date(t.exit_time).toLocaleString() : '—'}</td>
                      <td>{t.exit_price ? fmtNum(t.exit_price) : '—'}</td>
                      <td className={t.pnl >= 0 ? 'positive' : 'negative'}>{fmtMoney(t.pnl)}</td>
                      <td className={t.pnl_pct >= 0 ? 'positive' : 'negative'}>{t.pnl_pct.toFixed(2)}%</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </details>
          )}
        </>
      )}
    </section>
  );
}
const METRIC_FIELDS: { key: string; label: string; sign?: boolean; fmt: (v: any) => string }[] = [
  { key: '总收益率', label: '总收益', sign: true, fmt: v => v == null ? '—' : fmtPct(v) },
  { key: '年化收益率', label: '年化', sign: true, fmt: v => v == null ? '—' : fmtPct(v) },
  { key: '最大回撤_pct', label: '回撤', fmt: v => v == null ? '—' : fmtPct(v) },
  { key: '夏普比率', label: '夏普', sign: true, fmt: v => v == null ? '—' : fmtNum(v, 2) },
  { key: '索提诺比率', label: '索提诺', sign: true, fmt: v => v == null ? '—' : fmtNum(v, 2) },
  { key: 'Calmar比率', label: 'Calmar', sign: true, fmt: v => v == null ? '—' : fmtNum(v, 2) },
  { key: '交易笔数', label: '笔数', fmt: v => v == null ? '—' : String(v) },
  { key: '胜率', label: '胜率', fmt: v => v == null ? '—' : fmtPct(v) },
  { key: '最终净值', label: '净值', fmt: v => v == null ? '—' : fmtMoney(v) },
];

// ===================================================================
// 模拟盘
// ===================================================================
function PaperTrading() {
  const [snapshot, setSnapshot] = useState<PaperSnapshot | null>(null);
  const [strategies, setStrategies] = useState<StrategyMeta[]>([]);
  const [strategy, setStrategy] = useState('');
  const [error, setError] = useState('');
  const chartRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    api.listStrategies().then(d => {
      setStrategies(d.strategies);
      if (d.strategies.length > 0) setStrategy(d.strategies.find(s => s.name === 'sma_cross')?.name || d.strategies[0].name);
    });
    api.paperSnapshot().then(setSnapshot).catch(console.error);
  }, []);

  // WebSocket
  useEffect(() => {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${proto}://${location.host}/api/paper/ws`);
    wsRef.current = ws;
    ws.onmessage = (e) => {
      try { setSnapshot(JSON.parse(e.data)); } catch {}
    };
    ws.onclose = () => { wsRef.current = null; };
    return () => ws.close();
  }, []);

  useEffect(() => {
    if (!snapshot || !chartRef.current) return;
    const Plotly = (window as any).Plotly;
    if (!Plotly) return;
    if (snapshot.equity_curve.length === 0) return;
    Plotly.react(chartRef.current, [{
      x: snapshot.equity_curve.map(p => new Date(p.timestamp)),
      y: snapshot.equity_curve.map(p => p.equity),
      type: 'scatter', mode: 'lines',
      line: { color: '#d29922', width: 1.8 },
      fill: 'tozeroy', fillcolor: 'rgba(210, 153, 34, 0.05)',
    }], {
      paper_bgcolor: '#1c2128', plot_bgcolor: '#1c2128',
      font: { color: '#e6edf3', family: 'system-ui', size: 11 },
      margin: { t: 30, b: 40, l: 60, r: 20 },
      xaxis: { gridcolor: '#21262d' },
      yaxis: { gridcolor: '#21262d', title: '净值 ($)' },
    }, { responsive: true, displayModeBar: false });
  }, [snapshot]);

  const start = async () => {
    try {
      await api.paperStrategy(strategy);
      await api.paperStart();
    } catch (e) { setError(String(e)); }
  };
  const stop = async () => {
    try { await api.paperStop(); } catch (e) { setError(String(e)); }
  };

  if (!snapshot) return <div className="ax-loading">加载中…</div>;

  return (
    <section className="ax-section">
      <h2>模拟盘</h2>
      <p className="ax-lead">实时数据 + 实时策略,5 秒一次推送。</p>
      <div className="ax-controls">
        <label>策略
          <Dropdown options={strategies.map(s => ({ v: s.name, l: s.display_name }))}
            value={strategy} onChange={setStrategy} minWidth={200} />
        </label>
        <button className="ax-btn primary" onClick={start} disabled={snapshot.is_running}>▶ 启动</button>
        <button className="ax-btn" onClick={stop} disabled={!snapshot.is_running}>⏸ 停止</button>
        <span className={'ax-status ' + (snapshot.is_running ? 'on' : 'off')}>
          {snapshot.is_running ? '🟢 运行中' : '⏹️ 未运行'}
        </span>
      </div>
      {error && <div className="ax-error">{error}</div>}
      <div className="ax-paper-stats">
        <div className="ax-stat"><div className="ax-stat-label">现金</div><div className="ax-stat-value">{fmtMoney(snapshot.cash)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">持仓价值</div><div className="ax-stat-value">{fmtMoney(snapshot.position_value)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">总净值</div><div className="ax-stat-value">{fmtMoney(snapshot.equity)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">持仓数量</div><div className="ax-stat-value">{fmtNum(snapshot.position_size, 4)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">成交笔数</div><div className="ax-stat-value">{snapshot.trades_count}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">最新价</div><div className="ax-stat-value">{snapshot.current_bar ? fmtNum(snapshot.current_bar.close) : '—'}</div></div>
      </div>
      <div ref={chartRef} className="ax-chart"></div>
      <details className="ax-trades" open>
        <summary>运行日志</summary>
        <div className="ax-log">
          {snapshot.log.slice(0, 30).map((l, i) => (
            <div className="ax-log-entry" key={i}>
              <span className="ax-log-ts">{new Date(l.timestamp).toLocaleTimeString()}</span>
              <span className={'ax-log-lvl ' + l.level.toLowerCase()}>{l.level}</span>
              <span className="ax-log-msg">{l.message}</span>
            </div>
          ))}
        </div>
      </details>
    </section>
  );
}

// ===================================================================
// 策略对比
// ===================================================================
function CompareStrategies() {
  const [strategies, setStrategies] = useState<StrategyMeta[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [customStrategies, setCustomStrategies] = useState<CustomStrategy[]>([]);
  const [symbol, setSymbol] = useState('');
  const [symbols, setSymbols] = useState<string[]>([]);
  const [source, setSource] = useState<SourceType>('real');
  const [capital, setCapital] = useState(10000);
  const [results, setResults] = useState<Array<{ name: string; result: BacktestResult }>>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [showCustomForm, setShowCustomForm] = useState(false);
  const [customName, setCustomName] = useState('');
  const [customBase, setCustomBase] = useState('sma_cross');
  const [customParams, setCustomParams] = useState('{"fast": 8, "slow": 30}');
  const [customSL, setCustomSL] = useState(0);
  const chartRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.listStrategies().then(d => {
      setStrategies(d.strategies);
      // 默认选除了 random 之外的全部
      const sel = new Set<string>();
      d.strategies.forEach(s => { if (s.name !== 'random') sel.add(s.name); });
      setSelected(sel);
    });
    api.listSymbols().then(d => {
      setSymbols(d.symbols);
      setSymbol('BTCUSDT');
    });
  }, []);

  const toggle = (name: string) => {
    setSelected(s => {
      const ns = new Set(s);
      if (ns.has(name)) ns.delete(name);
      else ns.add(name);
      return ns;
    });
  };
  const selectAll = (on: boolean) => {
    if (on) {
      const ns = new Set<string>();
      strategies.forEach(s => { if (s.name !== 'random') ns.add(s.name); });
      customStrategies.forEach((_, i) => ns.add('custom_' + i));
      setSelected(ns);
    } else {
      setSelected(new Set());
    }
  };
  const selectDefault = () => {
    const ns = new Set<string>();
    strategies.forEach(s => { if (s.name !== 'random') ns.add(s.name); });
    setSelected(ns);
  };

  const addCustom = () => {
    if (!customName.trim()) return;
    let params: Record<string, number> = {};
    try { params = JSON.parse(customParams); } catch (e) { alert('参数 JSON 格式错误'); return; }
    const cs: CustomStrategy = { name: customName, base: customBase, params, sl: customSL / 100 };
    setCustomStrategies([...customStrategies, cs]);
    setSelected(s => new Set([...s, 'custom_' + (customStrategies.length)]));
    setShowCustomForm(false);
    setCustomName('');
  };

  const run = async () => {
    if (selected.size < 2) {
      setError('请至少选 2 个策略');
      return;
    }
    setLoading(true);
    setError('');
    setResults([]);
    const list: Array<{ name: string; result: BacktestResult }> = [];
    for (const item of selected) {
      let req;
      if (item.startsWith('custom_')) {
        const cs = customStrategies[parseInt(item.replace('custom_', ''))];
        req = { strategy: cs.base, params: cs.params, symbol, source, limit: 500, initial_capital: capital, stop_loss_pct: cs.sl };
      } else {
        const meta = strategies.find(s => s.name === item);
        const params: Record<string, number> = {};
        if (meta) meta.params.forEach((p: any) => params[p.key] = p.default);
        req = { strategy: item, params, symbol, source, limit: 500, initial_capital: capital };
      }
      try {
        const r = await api.runBacktest(req);
        list.push({ name: item, result: r });
      } catch (e) { console.error('策略失败:', item, e); }
    }
    setResults(list);
    setLoading(false);
  };

  useEffect(() => {
    if (!results.length || !chartRef.current) return;
    const Plotly = (window as any).Plotly;
    if (!Plotly) return;
    const colors = ['#d29922', '#58a6ff', '#a371f7', '#ff7b72', '#56d4dd', '#7ee787', '#ffa657', '#e91e63', '#607d8b'];
    const names: Record<string, string> = {};
    strategies.forEach(s => { names[s.name] = s.display_name; });
    customStrategies.forEach((cs, i) => { names['custom_' + i] = '✦ ' + cs.name; });
    Plotly.react(chartRef.current, results.map((r, i) => ({
      x: r.result.equity_curve.map(p => new Date(p.timestamp)),
      y: r.result.equity_curve.map(p => p.equity),
      type: 'scatter', mode: 'lines',
      name: names[r.name] || r.name,
      line: { color: colors[i % colors.length], width: 1.8 },
    })), {
      paper_bgcolor: '#1c2128', plot_bgcolor: '#1c2128',
      font: { color: '#e6edf3', family: 'system-ui', size: 11 },
      margin: { t: 30, b: 40, l: 60, r: 20 },
      xaxis: { gridcolor: '#21262d' },
      yaxis: { gridcolor: '#21262d', title: '净值 ($)' },
      legend: { orientation: 'h', y: -0.15 },
    }, { responsive: true, displayModeBar: false });
  }, [results, strategies, customStrategies]);

  return (
    <section className="ax-section">
      <h2>策略对比</h2>
      <p className="ax-lead">勾选策略,PK 同一段历史数据上谁最强。可自定义。</p>
      <div className="ax-controls">
        <label>交易对
          <Dropdown options={symbols.map(s => ({ v: s, l: s }))}
            value={symbol} onChange={setSymbol} minWidth={120} />
        </label>
        <label>数据源
          <Dropdown options={[{v:'real',l:'真实 (Binance)'},{v:'synthetic',l:'合成 (随机)'}]}
            value={source} onChange={(v: SourceType) => setSource(v)} minWidth={140} />
        </label>
        <label>资金
          <input type="number" value={capital} onChange={e => setCapital(parseFloat(e.target.value) || 10000)} step={1000} />
        </label>
        <button className="ax-btn primary" onClick={run} disabled={loading}>
          {loading ? '运行中...' : '跑对比'}
        </button>
      </div>

      <div className="ax-pool">
        <h4>选择要对比的策略 (至少 2 个)</h4>
        <div className="ax-pool-grid">
          {strategies.map(s => (
            <label key={s.name} className={'ax-pool-item' + (selected.has(s.name) ? ' checked' : '')}
              onClick={() => toggle(s.name)}>
              <input type="checkbox" checked={selected.has(s.name)} onChange={() => {}} />
              <span>{s.display_name}</span>
            </label>
          ))}
          {customStrategies.map((cs, i) => (
            <label key={i} className={'ax-pool-item custom checked'}
              onClick={() => toggle('custom_' + i)}>
              <input type="checkbox" checked={selected.has('custom_' + i)} onChange={() => {}} />
              <span>✦ {cs.name}</span>
            </label>
          ))}
        </div>
        <div className="ax-pool-actions">
          <button onClick={() => selectAll(true)}>全选</button>
          <button onClick={() => selectAll(false)}>全不选</button>
          <button onClick={selectDefault}>默认</button>
          <button onClick={() => setShowCustomForm(s => !s)}>+ 自定义策略</button>
        </div>
        {showCustomForm && (
          <div className="ax-custom-form">
            <div className="ax-form-row">
              <label>策略名称
                <input type="text" value={customName} onChange={e => setCustomName(e.target.value)} placeholder="我的 SMA 策略" />
              </label>
              <label>基础策略
                <Dropdown options={strategies.filter(s => s.name !== 'random').map(s => ({ v: s.name, l: s.display_name }))}
                  value={customBase} onChange={setCustomBase} minWidth={180} />
              </label>
            </div>
            <div className="ax-form-row">
              <label>参数 (JSON)
                <input type="text" value={customParams} onChange={e => setCustomParams(e.target.value)} placeholder='{"fast": 8}' />
              </label>
              <label>止损 %
                <input type="number" value={customSL} onChange={e => setCustomSL(parseFloat(e.target.value) || 0)} step={0.5} min={0} max={50} />
              </label>
            </div>
            <div className="ax-form-actions">
              <button className="ax-btn primary" onClick={addCustom}>保存并加入</button>
              <button onClick={() => setShowCustomForm(false)}>取消</button>
            </div>
          </div>
        )}
      </div>

      {error && <div className="ax-error">{error}</div>}
      {loading && <div className="ax-loading">运行中 ({selected.size} 个策略)...</div>}

      {results.length > 0 && (
        <>
          <table className="ax-cmp-table">
            <thead>
              <tr>
                <th>策略</th>
                {METRIC_FIELDS.map(f => <th key={f.key}>{f.label}</th>)}
              </tr>
            </thead>
            <tbody>
              {results.map(r => {
                const meta = strategies.find(s => s.name === r.name);
                const cs = customStrategies.find((_, i) => 'custom_' + i === r.name);
                const displayName = cs ? '✦ ' + cs.name : (meta?.display_name || r.name);
                return (
                  <tr key={r.name}>
                    <td><b>{displayName}</b></td>
                    {METRIC_FIELDS.map(f => {
                      const v = r.result.metrics[f.key];
                      let cls = '';
                      if (f.sign && typeof v === 'number') {
                        cls = v > 0 ? 'positive' : v < 0 ? 'negative' : '';
                      }
                      return <td key={f.key} className={cls}>{f.fmt(v)}</td>;
                    })}
                  </tr>
                );
              })}
            </tbody>
          </table>
          <div ref={chartRef} className="ax-chart"></div>
        </>
      )}
    </section>
  );
}

// ===================================================================
// 主应用
// ===================================================================
export default function App() {
  const [tab, setTab] = useState<TabId>('learn');

  return (
    <div className="ax-app">
      <Header />
      <TabBar active={tab} onChange={setTab} />
      <main className="ax-main">
        {tab === 'learn' && <LearnCenter />}
        {tab === 'data' && <DataExplore />}
        {tab === 'backtest' && <Backtest />}
        {tab === 'paper' && <PaperTrading />}
        {tab === 'compare' && <CompareStrategies />}
      </main>
      <footer className="ax-footer">
        AXIOM · 仅供学习,不构成任何投资建议
      </footer>
    </div>
  );
}
