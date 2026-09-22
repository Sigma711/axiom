import { useState, useEffect, useLayoutEffect, useCallback, useRef } from 'react';
import Plotly from 'plotly.js-dist-min';
import { api, appBase, appPath, fmtPct, fmtNum, fmtMoney, preferredCodeLocationUrl } from './api';
import { performanceInputs } from './performance';
import { KnowledgeSeriesVisual } from './KnowledgeSeriesVisual';
import { SymbolPicker } from './SymbolPicker';
import { indicatorPanel, validSeries, type IndicatorPanel } from './chart';
const MARKET_SOURCE_OPTIONS = [
  { v: 'binance', l: '加密货币 · Binance' },
  { v: 'a_share', l: 'A 股 · 东方财富' },
  { v: 'us_stock', l: '美股 · Yahoo Finance' },
] as const;
const MARKET_SOURCE_LABEL: Record<string, string> = Object.fromEntries(MARKET_SOURCE_OPTIONS.map(item => [item.v, item.l]));

import type {
  TabId, SourceType, Bar, BacktestResult, StrategyMeta,
  KnowledgeResponse, KnowledgeEntry, CustomStrategy, ChartType,
  PaperSnapshot, EquityPoint, PracticeConcept, PracticeResult,
} from './types';

const CHINESE_FIELDS: Record<string, string> = {
  price: '价格', close: '收盘价', open: '开盘价', high: '最高价', low: '最低价', volume: '成交量',
  period: '周期', fast: '快线周期', slow: '慢线周期', signal: '信号周期', multiplier: '倍数',
  returns: '收益率序列', equity: '净值序列', elapsed_days: '经过天数', periods_per_year: '年化周期数',
  rsi: '相对强弱指标', value: '计算值', mean: '均值', stddev: '标准差', correlation: '相关系数',
  beta: '贝塔系数', alpha: '阿尔法', sharpe: '夏普比率', max_drawdown: '最大回撤',
  eps: '每股收益', net_income: '净利润', preferred_dividends: '优先股股息', shares: '普通股股数', weighted_shares: '加权平均普通股股数', weighted_average_shares: '加权平均普通股股数',
};
const CHINESE_UNITS: Record<string, string> = { fraction: '比例（小数）', annualized_ratio: '年化比率', price: '价格', currency: '元', share: '股', shares: '股', 'currency/share': '元/股', '元/股': '元/股', percent: '%', ratio: '比率', days: '天', bars: '根 K 线' };
function chineseUnit(unit?: string) { return unit ? (CHINESE_UNITS[unit] || unit) : ''; }
function chineseField(key: string, label?: string) {
  if (label && !/^[a-z_]+$/i.test(label)) return label;
  if (CHINESE_FIELDS[key]) return CHINESE_FIELDS[key];
  return key.replace(/_/g, ' · ');
}
function resultSentence(name: string, values: Record<string, number | null>, units?: Record<string, string>, hasSeries = false) {
  const first = Object.entries(values).find(([, value]) => value != null);
  if (!first) return `${name} 当前没有足够数据，图中的空白表示预热期或无法定义的结果。`;
  const [key, value] = first;
  const prefix = hasSeries ? '基于这段教学行情' : '给定图中的教学输入';
  const reading = hasSeries ? '曲线展示该数值随样本变化；留意水平、拐点和空白预热区。' : Object.keys(values).length > 1 ? '请结合各结果之间的关系解读。' : '这是单次计算结果，应结合它的定义和背景解读。';
  return `${prefix}，${name} 的${CHINESE_FIELDS[key] || '计算结果'}为 ${fmtNum(value, 6)}${chineseUnit(units?.[key]) ? ` ${chineseUnit(units?.[key])}` : ''}；${reading}`;
}
function plotTheme() {
  const style = getComputedStyle(document.documentElement);
  return {
    paper: style.getPropertyValue('--bg-card').trim(), plot: style.getPropertyValue('--bg-card').trim(),
    text: style.getPropertyValue('--text').trim(), grid: style.getPropertyValue('--border-soft').trim(),
    accent: style.getPropertyValue('--accent').trim(), blue: style.getPropertyValue('--accent-2').trim(),
    fill: style.getPropertyValue('--chart-fill').trim(),
    green: style.getPropertyValue('--green').trim(), red: style.getPropertyValue('--red').trim(),
    palette: ['--accent', '--accent-2', '--chart-purple', '--red', '--chart-cyan', '--green'].map(key => style.getPropertyValue(key).trim()),
  };
}


function dateAxis(grid: string) {
  return { type: 'date', gridcolor: grid, zerolinecolor: grid, tickformat: '%m-%d<br>%H:%M', hoverformat: '%Y-%m-%d %H:%M', tickangle: 0, nticks: 5, automargin: true };
}


// ===================================================================
// 顶栏
// ===================================================================
function Header({ theme, onToggleTheme }: { theme: 'light' | 'dark'; onToggleTheme: () => void }) {
  return (
    <header className="ax-header">
      <div className="ax-header-inner">
        <div className="ax-brand">
          <span className="ax-logo">◆</span>
          <span className="ax-title">AXIOM</span>
          <span className="ax-sub">从公理出发,推导你的市场观</span>
        </div>
        <button className="ax-theme-toggle" onClick={onToggleTheme} aria-label={theme === 'dark' ? '切换到浅色模式' : '切换到深色模式'} title={theme === 'dark' ? '切换到浅色模式' : '切换到深色模式'}>{theme === 'dark' ? '☀' : '☾'}</button>
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

function routeFor(tab: TabId, sub: LearnSub = 'knowledge', concept?: string) {
  const learnPaths: Record<LearnSub, string> = { knowledge: '/learn', book: '/learn/book', concepts: '/learn/concepts', build: '/learn/build', path: '/learn/path' };
  const base = tab === 'learn' ? learnPaths[sub] : `/${tab}`;
  const path = appPath(base);
  return concept ? `${path}?concept=${encodeURIComponent(concept)}` : path;
}
function readRoute() {
  const pathname = window.location.pathname.replace(/\/+$/, '') || '/';
  const path = (appBase && pathname.startsWith(appBase) ? pathname.slice(appBase.length) : pathname) || '/';
  const tab = ({ '/data': 'data', '/backtest': 'backtest', '/paper': 'paper', '/compare': 'compare' } as Record<string, TabId>)[path] || 'learn';
  const sub = ({ '/learn/book': 'book', '/learn/concepts': 'concepts', '/learn/build': 'build', '/learn/path': 'path' } as Record<string, LearnSub>)[path] || 'knowledge';
  return { tab, sub, concept: new URLSearchParams(window.location.search).get('concept') || undefined };
}

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
  label: string;
  options: readonly { v: T; l: string }[];
  value: T;
  onChange: (v: T) => void;
  minWidth?: number;
}
function Dropdown<T extends string | number>({ label, options, value, onChange, minWidth = 100 }: DropdownProps<T>) {
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
    <div className={'ax-dropdown' + (open ? ' open' : '')} ref={ref} style={{ minWidth }} onKeyDown={event => {
      if (event.key === 'Escape') { setOpen(false); ref.current?.querySelector('button')?.focus(); }
    }}>
      <button type="button" className="ax-dd-trigger" aria-label={label} aria-haspopup="listbox" aria-expanded={open} onClick={() => setOpen(o => !o)}>
        <span>{current?.l || '选择...'}</span>
      </button>
      {open && (
        <div className="ax-dd-menu" role="listbox" aria-label={label}>
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
            <button
              type="button"
              role="option"
              aria-selected={o.v === value}
              key={String(o.v)}
              className={'ax-dd-item' + (o.v === value ? ' selected' : '')}
              onClick={() => { onChange(o.v); setOpen(false); }}
            >
              {o.l}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

interface MultiSelectDropdownProps<T> {
  label: string;
  options: readonly { v: T; l: string }[];
  values: T[];
  onChange: (values: T[]) => void;
  minWidth?: number;
}
function MultiSelectDropdown<T extends string | number>({ label, options, values, onChange, minWidth = 180 }: MultiSelectDropdownProps<T>) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const ref = useRef<HTMLDivElement>(null);
  const selected = new Set(values);
  const allSelected = options.length > 0 && options.every(option => selected.has(option.v));
  const visibleOptions = options.filter(option => option.l.toLowerCase().includes(query.trim().toLowerCase()));

  useEffect(() => {
    if (!open) return;
    const handler = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const toggle = (value: T) => onChange(selected.has(value) ? values.filter(item => item !== value) : [...values, value]);
  const clearOrSelectAll = () => onChange(allSelected ? [] : options.map(option => option.v));
  const triggerLabel = values.length === 0 ? '未选择指标' : values.length === 1 ? options.find(option => option.v === values[0])?.l || '已选 1 项' : `已选 ${values.length} 项`;

  return (
    <div className={'ax-dropdown ax-multi-select' + (open ? ' open' : '')} ref={ref} style={{ minWidth }} onKeyDown={event => {
      if (event.key === 'Escape') { setOpen(false); ref.current?.querySelector<HTMLButtonElement>('.ax-dd-trigger')?.focus(); }
    }}>
      <button type="button" className="ax-dd-trigger" aria-label={label} aria-haspopup="listbox" aria-expanded={open} onClick={() => { setOpen(value => !value); setQuery(''); }}>
        <span>{triggerLabel}</span>
      </button>
      {open && (
        <div className="ax-dd-menu ax-multi-menu" role="listbox" aria-label={label} aria-multiselectable="true">
          <div className="ax-multi-actions">
            <button type="button" className="ax-multi-action" onClick={clearOrSelectAll}>{allSelected ? '全部取消' : '全选'}</button>
            <span aria-live="polite">已选 {values.length} / {options.length}</span>
          </div>
          <input className="ax-dd-search" placeholder="搜索指标..." aria-label="搜索指标" autoFocus value={query} onChange={event => setQuery(event.target.value)} />
          {visibleOptions.map(option => {
            const isSelected = selected.has(option.v);
            return <button type="button" role="option" aria-selected={isSelected} key={String(option.v)}
              className={'ax-dd-item ax-multi-option' + (isSelected ? ' selected' : '')} onClick={() => toggle(option.v)}>
              <span className="ax-multi-check" aria-hidden="true">{isSelected ? '✓' : ''}</span>{option.l}
            </button>;
          })}
          {visibleOptions.length === 0 && <p className="ax-multi-empty">没有匹配的指标</p>}
        </div>
      )}
    </div>
  );
}

const INDICATOR_OPTIONS = [
  { v: 'sma_20', l: 'SMA (20)' }, { v: 'ema_50', l: 'EMA (50)' }, { v: 'rsi_14', l: 'RSI (14)' },
  { v: 'bbands_20', l: '布林带 (20)' }, { v: 'macd', l: 'MACD' }, { v: 'vwap', l: 'VWAP' },
  { v: 'vwma_20', l: 'VWMA (20)' }, { v: 'atr_14', l: 'ATR (14)' }, { v: 'atr_percent_14', l: 'ATR % (14)' },
  { v: 'obv', l: 'OBV' }, { v: 'zscore_20', l: 'Z-Score (20)' }, { v: 'ichimoku', l: '一目均衡表' },
  { v: 'kdj', l: 'KDJ' }, { v: 'stoch_14', l: '随机指标 (14)' }, { v: 'williams_r_14', l: 'Williams %R (14)' },
  { v: 'cci_20', l: 'CCI (20)' }, { v: 'adx_14', l: 'ADX (14)' }, { v: 'bbi', l: 'BBI' },
  { v: 'alligator', l: '鳄鱼线' }, { v: 'ppo', l: 'PPO' }, { v: 'vortex_14', l: 'Vortex (14)' },
] as const;

// ===================================================================
// 学习中心: 4 个子标签
// ===================================================================
type LearnSub = 'knowledge' | 'concepts' | 'build' | 'path' | 'book';
const LEARN_SUBS: { id: LearnSub; label: string }[] = [
  { id: 'knowledge', label: '指标大全' },
  { id: 'concepts', label: '概念速览' },
  { id: 'build', label: '创建策略' },
  { id: 'book', label: '原书阅读' },
  { id: 'path', label: '学习路径' },
];

function LearnCenter({ sub, onSubChange, onPractice }: { sub: LearnSub; onSubChange: (sub: LearnSub) => void; onPractice: (conceptId: string) => void }) {
function BookReader() {
  const toc = [['封面与目录', 1], ['指标基础与均线', 6], ['趋势与动量指标', 18], ['摆动与超买超卖', 31], ['成交量与量价关系', 43], ['K 线形态', 55], ['图表与实战方法', 67], ['附录与索引', 78]] as const;
  const [tocOpen, setTocOpen] = useState(true), [page, setPage] = useState(1);
  return <div className={'ax-book-reader' + (tocOpen ? '' : ' toc-collapsed')}><aside className={'ax-book-toc' + (tocOpen ? '' : ' collapsed')}>
    <button className="ax-book-toc-toggle" type="button" aria-expanded={tocOpen} aria-label={tocOpen ? '收起原书目录' : '展开原书目录'} title={tocOpen ? '收起目录' : '展开目录'} onClick={() => setTocOpen(open => !open)}><span aria-hidden="true">{tocOpen ? '‹' : '›'}</span></button>
    {tocOpen && <nav aria-label="原书目录"><h3>原书目录</h3>{toc.map(([label, target]) => <button key={target} type="button" className={page === target ? 'active' : ''} onClick={() => setPage(target)}>{label}<small>第 {target} 页</small></button>)}</nav>}
  </aside><section className="ax-book-page" aria-label="股票交易软件专业指标全解阅读器">
    <p className="ax-practice-note">仅提供本书阅读。点击目录定位页码；阅读区会按宽度适配，并可在 PDF 内上下滚动。</p>
    <iframe key={page} title="股票交易软件专业指标全解" src={appPath(`/api/book/pdf#page=${page}&view=FitH`)} />
  </section></div>;
}

  return (
    <section className="ax-section">
      <h2>学习中心</h2>
      <p className="ax-lead">从公理出发,推导出你自己的市场观。</p>
      <nav className="ax-learn-subnav">
        {LEARN_SUBS.map(s => (
          <button key={s.id}
            className={'ax-learn-sub' + (s.id === sub ? ' active' : '')}
            onClick={() => onSubChange(s.id)}>{s.label}</button>
        ))}
      </nav>
      {sub === 'knowledge' && <KnowledgeView onPractice={onPractice} />}
      {sub === 'concepts' && <ConceptsView />}
      {sub === 'book' && <BookReader />}
      {sub === 'build' && <BuildView />}
      {sub === 'path' && <PathView />}
    </section>
  );
}

// 指标大全
function KnowledgeView({ onPractice }: { onPractice: (conceptId: string) => void }) {
  const [data, setData] = useState<KnowledgeResponse | null>(null);
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);
  const [openId, setOpenId] = useState<string>();
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

  const pageSize = 24, pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const currentPage = Math.min(page, pageCount - 1);
  const visible = filtered.slice(currentPage * pageSize, (currentPage + 1) * pageSize);
  return (
    <div>
      <div className="ax-search-bar">
        <input
          type="text"
          placeholder="搜索 概念 / 公式 / 关键词"
          value={search}
          onChange={e => { setSearch(e.target.value); setPage(0); setOpenId(undefined); }}
        />
      </div>
      <div className="ax-cat-pills">
        <button className={'ax-pill' + (!activeCat ? ' active' : '')} onClick={() => { setActiveCat(''); setPage(0); setOpenId(undefined); }}>
          全部 ({data.total})
        </button>
        {cats.map(c => (
          <button key={c} className={'ax-pill' + (activeCat === c ? ' active' : '')}
            onClick={() => { setActiveCat(c); setPage(0); setOpenId(undefined); }}>
            {c} ({data.categories[c].length})
          </button>
        ))}
      </div>
      <p style={{ color: 'var(--text-dim)', fontSize: '0.82rem', margin: '0.4rem 0 0.8rem' }}>
        {filtered.length} 条结果
      </p>
      {visible.map(e => <KbCard key={e.id} e={e} onPractice={onPractice} open={openId === e.id} onOpenChange={open => setOpenId(open ? e.id : undefined)} />)}
      {pageCount > 1 && <nav className="ax-pagination" aria-label="指标大全分页"><button disabled={currentPage === 0} onClick={() => { setPage(currentPage - 1); setOpenId(undefined); }}>上一页</button><span>第 {currentPage + 1} / {pageCount} 页</span><button disabled={currentPage + 1 >= pageCount} onClick={() => { setPage(currentPage + 1); setOpenId(undefined); }}>下一页</button></nav>}
    </div>
  );
}

function KbCard({ e, onPractice, open, onOpenChange }: { e: KnowledgeEntry; onPractice: (conceptId: string) => void; open: boolean; onOpenChange: (open: boolean) => void }) {
  // The parent keeps exactly one expensive detail panel mounted.
  return (
    <div className="ax-kb-card">
      <div className="ax-kb-header"><h4>{e.name}</h4></div>
      <div className="ax-kb-summary">{e.summary}</div>
      <div className="ax-kb-practice" aria-label={`${e.name} 实践入口`}>
        <button onClick={() => onPractice(e.id)}>在数据探索中实践</button>
      </div>
      <details key={open ? 'open' : 'closed'} open={open}>
        <summary className={`ax-kb-details${open ? ' is-open' : ''}`} aria-label={open ? `收起 ${e.name} 详情` : `展开 ${e.name} 详情`} onClick={event => { event.preventDefault(); onOpenChange(!open); }}>
          <span className="ax-kb-details-icon" aria-hidden="true">{open ? '⌃' : '⌄'}</span>
          {open ? '收起详情' : '展开详情'}
        </summary>
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
        {e.source_refs && e.source_refs.length > 0 && <Section label="书中出处">{e.source_refs.map(ref => <span key={`${ref.source_id}-${ref.pdf_page}`} className="ax-tag">{ref.title} · 第 {ref.pdf_page} 页</span>)}</Section>}
        <Section label="代码实现"><CodeLink entry={e} detailed /></Section>
        {open && <KnowledgeVisual concept={e} />}
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

function CodeLink({ entry, detailed = false }: { entry: KnowledgeEntry; detailed?: boolean }) {
  const [url, setUrl] = useState(entry.code_url || '');
  const [loading, setLoading] = useState(false);
  const resolve = async () => {
    if (url || loading) return;
    setLoading(true);
    try {
      const location = await api.getCodeLocation(entry.code_ref || entry.implementation || entry.id);
      setUrl(preferredCodeLocationUrl(location) || entry.code_url);
    } catch { setUrl(entry.code_url); } finally { setLoading(false); }
  };
  useEffect(() => { void resolve(); }, [entry.id]);
  if (url) return <a href={url} target="_blank" rel="noopener noreferrer" className={detailed ? 'ax-code-link' : 'ax-gh-btn'}>{detailed ? <><code>{entry.implementation || entry.code_ref || entry.id}</code><span className="ax-goto">打开精确源码 ↗</span></> : '↗ 源码'}</a>;
  return <button className={detailed ? 'ax-code-link ax-code-resolve' : 'ax-gh-btn'} onClick={resolve} disabled={loading}>{loading ? '定位中…' : detailed ? `定位 ${entry.implementation || entry.code_ref || entry.id}` : '↗ 定位源码'}</button>;
}

let practiceCatalogPromise: Promise<PracticeConcept[]> | undefined;
function practiceCatalog() {
  practiceCatalogPromise ??= api.listPractice().then(response => response.concepts);
  return practiceCatalogPromise;
}
const FORMULA_FALLBACK: Record<string, string> = { eps: '(净利润 − 优先股股息) ÷ 加权平均普通股股数', earnings_per_share: '(净利润 − 优先股股息) ÷ 加权平均普通股股数' };
function ScalarKnowledgeDiagram({ concept, inputs, scalar, unit, loading }: { concept: KnowledgeEntry; inputs: PracticeConcept['inputs']; scalar?: [string, number | null]; unit?: string; loading?: boolean }) {
  const formula = concept.formula || FORMULA_FALLBACK[concept.id] || `${concept.name} 的定义公式`;
  const shownInputs = inputs.length ? inputs : [{ key: 'market_bars', label: '教学行情样本', default: '已传入 K 线' }];
  const resultLabel = scalar ? (CHINESE_FIELDS[scalar[0]] || concept.name) : concept.name;
  const compact = scalar?.[1] != null ? fmtNum(scalar[1], 4).slice(0, 12) : '当前无定义';
  return <><figure className="ax-knowledge-chart ax-scalar-chart"><svg viewBox="0 0 420 116" role="img" aria-label={`${concept.name} 输入、公式和结果图解`}><rect x="10" y="25" width="112" height="66" rx="6" /><text x="22" y="50">输入</text><text x="22" y="72">{shownInputs.length} 项教学数据</text><path d="M132 58 H148" /><rect x="158" y="25" width="104" height="66" rx="6" /><text x="170" y="50">概念</text><text x="170" y="72">按公式计算</text><path d="M272 58 H288" /><rect x="298" y="25" width="112" height="66" rx="6" /><text x="310" y="50">结果</text><text x="310" y="72">{loading ? '计算中…' : compact}</text></svg><figcaption>教学输入 → 定义公式 → 可解释的数值结果</figcaption></figure><dl className="ax-knowledge-values">{shownInputs.map(input => <div key={input.key}><dt>{chineseField(input.key, input.label)} <small>({input.key})</small></dt><dd>{JSON.stringify(input.default)}</dd></div>)}<div className="ax-knowledge-formula"><dt>公式</dt><dd>{formula}</dd></div><div><dt>结果含义</dt><dd>{resultLabel}{unit ? ` · ${chineseUnit(unit)}` : ''}</dd></div></dl></>;
}
function BookChartVisual({ chart, name }: { chart: NonNullable<PracticeResult['chart']>; name: string }) {
  const bars = chart.bars.filter(bar => [bar.open, bar.high, bar.low, bar.close].every(Number.isFinite));
  if (!bars.length) return <ScalarKnowledgeDiagram concept={{ id: name, name, formula: '', category: '', summary: '', meaning: '', example: '', signals: '', pitfalls: '', related: [], code_url: '', implementation: '' }} inputs={[]} />;
  const lo = Math.min(...bars.map(bar => bar.low)), hi = Math.max(...bars.map(bar => bar.high));
  const scale = (value: number) => 90 - ((value - lo) / (hi - lo || 1)) * 74;
  const width = 396 / Math.max(bars.length, 1);
  const candleKinds = ['heikin_ashi', 'range_bars', 'tick_bars'];
  const isCandle = candleKinds.includes(chart.kind);
  const isKagi = chart.kind === 'kagi';
  const isPnf = chart.kind === 'point_figure' || chart.kind === 'point_and_figure';
  const label = ({ heikin_ashi: '平均K线', range_bars: '范围K线', tick_bars: 'Tick K线', renko: '砖形图', point_figure: '点数图', kagi: '卡吉线', three_line_break: '三线突破' } as Record<string, string>)[chart.kind] || name;
  const columns = bars.reduce<number[]>((all, bar, index) => {
    if (index === 0) return [bar.column ?? 0];
    const previous = bars[index - 1];
    return [...all, bar.column ?? (bar.direction === previous.direction ? all[index - 1] : all[index - 1] + 1)];
  }, []), minColumn = Math.min(...columns), maxColumn = Math.max(...columns);
  const columnX = (column: number) => 12 + ((column - minColumn) / (maxColumn - minColumn || 1)) * 396;
  return <figure className="ax-knowledge-chart ax-book-chart"><svg data-chart-kind={chart.kind} viewBox="0 0 420 116" role="img" aria-label={`${label}图解`}><line x1="12" y1="92" x2="408" y2="92" />{isCandle ? bars.map((bar, index) => { const x = 12 + index * width + width / 2, rise = bar.close >= bar.open; return <g key={index} className={rise ? 'up' : 'down'}><line x1={x} y1={scale(bar.high)} x2={x} y2={scale(bar.low)} /><rect x={x - Math.max(1, width * .28)} y={Math.min(scale(bar.open), scale(bar.close))} width={Math.max(2, width * .56)} height={Math.max(1, Math.abs(scale(bar.open) - scale(bar.close)))} /></g>; }) : isPnf ? bars.map((bar, index) => <text key={index} data-pnf-column={columns[index]} className={bar.direction && bar.direction < 0 ? 'down' : 'up'} x={columnX(columns[index])} y={scale(bar.close)} textAnchor="middle">{bar.direction && bar.direction < 0 ? 'O' : 'X'}</text>) : chart.kind === 'renko' || chart.kind === 'three_line_break' ? bars.map((bar, index) => { const rise = (bar.direction ?? (bar.close >= bar.open ? 1 : -1)) > 0, x = 12 + index * width; return <rect key={index} className={rise ? 'up' : 'down'} x={x} y={Math.min(scale(bar.open), scale(bar.close))} width={Math.max(2, width - 1)} height={Math.max(3, Math.abs(scale(bar.open) - scale(bar.close)))} />; }) : isKagi ? bars.map((bar, index) => { const x = columnX(columns[index]), previous = bars[index - 1], beforeStyle = previous?.line_style || 'neutral', style = bar.line_style || 'neutral', switched = bar.switch_price != null && Math.min(bar.open, bar.close) <= bar.switch_price && bar.switch_price <= Math.max(bar.open, bar.close); return <g key={index}>{index > 0 && columns[index] !== columns[index - 1] && <line data-kagi-horizontal="true" className={beforeStyle} x1={columnX(columns[index - 1])} y1={scale(previous.close)} x2={x} y2={scale(previous.close)} />}{switched ? <><line data-kagi-vertical="true" className={beforeStyle} x1={x} y1={scale(bar.open)} x2={x} y2={scale(bar.switch_price!)} /><line data-kagi-switch="true" className={style} x1={x} y1={scale(bar.switch_price!)} x2={x} y2={scale(bar.close)} /></> : <line data-kagi-vertical="true" className={style} x1={x} y1={scale(bar.open)} x2={x} y2={scale(bar.close)} />}</g>; }) : null}</svg><figcaption>{label} · {isCandle ? '每根显示开高低收' : isPnf ? 'X 为上涨列，O 为下跌列' : isKagi ? '同向段共列；横线连接转向，阴阳切换点分段显示' : chart.kind === 'renko' ? '每砖代表固定价格移动' : '按突破方向形成的实体'}</figcaption></figure>;
}

function KnowledgeVisual({ concept }: { concept: KnowledgeEntry }) {
function CandlePatternVisual({ concept }: { concept: KnowledgeEntry }) {
  const kind = concept.id.replace('k_pattern_', '');
  if (!['hammer', 'doji', 'engulfing', 'star'].includes(kind)) return null;
  const candle = (x: number, top: number, bottom: number, up: boolean, wickTop = 18, wickBottom = 98) => <g className={up ? 'up' : 'down'}><line x1={x + 14} x2={x + 14} y1={wickTop} y2={wickBottom} /><rect x={x} y={top} width="28" height={bottom - top} rx="2" /></g>;
  const drawing = kind === 'hammer' ? <g>{candle(190, 42, 58, true, 30, 105)}</g> : kind === 'doji' ? <g className="up"><line x1="204" x2="204" y1="20" y2="102" /><line x1="187" x2="221" y1="62" y2="62" strokeWidth="5" /></g> : kind === 'engulfing' ? <g>{candle(150, 44, 75, false, 30, 94)}{candle(205, 28, 88, true, 16, 103)}</g> : <g>{candle(118, 32, 82, false, 18, 96)}{candle(196, 57, 62, true, 42, 84)}{candle(274, 26, 76, true, 12, 92)}</g>;
  const caption = ({ hammer: '小实体靠近高位，长下影线显示低位买盘回收。', doji: '开盘与收盘接近，十字实体表示方向犹豫。', engulfing: '后一根实体完全包住前一根，才是吞没。', star: '大实体、星体、反向大实体组成三根确认。' } as Record<string, string>)[kind];
  return <figure className="ax-knowledge-chart ax-candle-pattern"><svg data-candle-pattern={kind} viewBox="0 0 420 116" role="img" aria-label={`${concept.name} K线形态图`}>{drawing}<line className="axis" x1="20" x2="400" y1="108" y2="108" /></svg><figcaption>{caption} 绿色为收涨，红色为收跌。</figcaption></figure>;
}

  const [result, setResult] = useState<PracticeResult | null>(null);
  const [practiceConcept, setPracticeConcept] = useState<PracticeConcept | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    Promise.all([practiceCatalog(), api.runPractice({ concept_id: concept.id, module: 'data', symbol: 'BTCUSDT', source: 'binance', limit: 80, inputs: {} })])
      .then(([catalog, value]) => { if (active) { setPracticeConcept(catalog.find(item => item.id === concept.id) || null); setResult(value); } })
      .catch(reason => { if (active) setError(String(reason)); });
    return () => { active = false; };
  }, [concept.id]);
  if (error) return <Section label="可计算示例" highlight><ScalarKnowledgeDiagram concept={concept} inputs={concept.inputs || []} /><p className="ax-practice-note">示例结果暂不可用：{error}</p></Section>;
  if (!result) return <Section label="可计算示例" highlight><ScalarKnowledgeDiagram concept={concept} inputs={concept.inputs || []} loading /><p className="ax-practice-note">正在生成与 {concept.name} 对应的示例…</p></Section>;
  const series = result.series.find(item => item.values.some(value => value != null));
  const values = series?.values.filter((value): value is number => value != null) ?? [];
  const scalar = Object.entries(result.values).find(([, value]) => value != null);
  const inputs = practiceConcept?.inputs || concept.inputs || [];
  const isCandlePattern = ['k_pattern_hammer', 'k_pattern_doji', 'k_pattern_engulfing', 'k_pattern_star'].includes(concept.id);
  return <Section label="可计算示例" highlight>
    <p className="ax-practice-note">{result.provenance === 'provided_market_bars' ? '基于已收盘的真实市场行情计算，用来观察数值变化，不代表交易建议。' : '基于可编辑教学输入计算，不代表当前币种行情。'}</p>
    {isCandlePattern ? <CandlePatternVisual concept={concept} /> : (result.chart ? <BookChartVisual chart={result.chart} name={concept.name} /> : series && values.length > 1 ? <KnowledgeSeriesVisual name={concept.name} result={result} /> : <ScalarKnowledgeDiagram concept={concept} inputs={inputs} scalar={scalar} unit={scalar ? result.units?.[scalar[0]] : undefined} />)}
    <p className="ax-practice-reading">{resultSentence(concept.name, result.values, result.units, Boolean(series && values.length > 1))}</p>
    {result.notes.slice(0, 1).map(note => <p className="ax-practice-note" key={note}>{note}</p>)}
  </Section>;
}

function PracticePanel({ module, symbol, source, limit, bars, contextInputs = {}, targetConcept }: {
  module: 'data' | 'backtest' | 'paper' | 'compare'; symbol: string; source: SourceType; limit: number; bars?: Bar[]; contextInputs?: Record<string, unknown>; targetConcept?: string;
}) {
  const [concepts, setConcepts] = useState<PracticeConcept[]>([]);
  const [conceptId, setConceptId] = useState('');
  const [inputs, setInputs] = useState<Record<string, string>>({});
  const [result, setResult] = useState<PracticeResult | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [usePageContext, setUsePageContext] = useState(true);
  const appliedTarget = useRef<string | undefined>();

  const selectConcept = (id: string, catalog = concepts) => {
    const concept = catalog.find(item => item.id === id);
    setConceptId(id);
    setInputs(Object.fromEntries((concept?.inputs ?? []).map(input => [input.key,
      JSON.stringify(contextInputs[input.key] ?? input.default)])));
    setUsePageContext((concept?.inputs ?? []).some(input => Object.prototype.hasOwnProperty.call(contextInputs, input.key)));
    setResult(null);
    setError('');
  };

  useEffect(() => {
    api.listPractice().then(data => {
      setConcepts(data.concepts);
      if (targetConcept && data.concepts.some(item => item.id === targetConcept)) {
        selectConcept(targetConcept, data.concepts);
      }
    }).catch(e => setError(`无法加载实践目录：${String(e)}`));
  }, [targetConcept]);

  useEffect(() => {
    if (targetConcept && targetConcept !== appliedTarget.current && concepts.some(item => item.id === targetConcept)) {
      appliedTarget.current = targetConcept;
      selectConcept(targetConcept);
    }
  }, [targetConcept, concepts]);

  const concept = concepts.find(item => item.id === conceptId);
  const run = async () => {
    if (!concept) return;
    if (concept.input_kind === 'market_bars' && !bars?.length) {
      setError('当前模块还没有可用行情上下文。请先加载数据、运行回测或等待模拟盘产生数据。');
      return;
    }
    const parsed: Record<string, unknown> = {};
    try {
      for (const input of concept.inputs) parsed[input.key] = usePageContext && Object.prototype.hasOwnProperty.call(contextInputs, input.key) ? contextInputs[input.key] : JSON.parse(inputs[input.key] ?? 'null');
    } catch {
      setError('输入必须是有效的 JSON 数值、数组或字符串。');
      return;
    }
    setLoading(true);
    setError('');
    try {
      setResult(await api.runPractice({ concept_id: concept.id, module, symbol, source, limit, inputs: parsed, bars: bars?.length ? bars : undefined }));
    } catch (e) {
      setError(String(e));
      setResult(null);
    } finally { setLoading(false); }
  };

  return (
    <aside className="ax-practice" aria-label="概念实践">
      <div><h3>概念实践</h3><p>{targetConcept ? '从指标大全直接进入。行情类概念使用当前数据探索的 K 线；其余概念明确使用可编辑教学输入，不冒充策略、回测或模拟盘结果。' : '从指标大全的“在数据探索中实践”进入一个有明确数据来源的练习。'}</p></div>
      {error && <div className="ax-error">{error}</div>}
      {concepts.length > 0 && <>
        <h4>{concept?.name} <small>· {concept?.category}</small></h4>
        {concept && <>
          <p className="ax-practice-note">{concept.notes}</p>
          {concept.plan && <div className="ax-practice-plan"><strong>适用范围</strong><span>{concept.plan.markets.map(m => ({crypto:'加密市场',cn_equity:'A 股',us_equity:'美股'}[m])).join('、')} · {concept.plan.modules.map(m => ({data:'数据探索',backtest:'回测',paper:'模拟盘',compare:'策略对比'}[m])).join('、')}</span><p>{concept.plan.goal}</p></div>}
          {concept.input_kind !== 'market_bars' && <p className="ax-practice-provenance">教学示例：这些可编辑输入不是 {symbol || '当前交易对'} 的实时或历史行情。</p>}
          {concept.inputs.some(input => Object.prototype.hasOwnProperty.call(contextInputs, input.key)) && <p className="ax-practice-provenance">{usePageContext ? '本页上下文已预填并用于计算。' : '已改用手动输入，运行时将覆盖本页上下文。'} <button type="button" className="ax-inline-action" onClick={() => setUsePageContext(value => !value)}>{usePageContext ? '改用手动输入' : '使用本页上下文'}</button></p>}
          {concept.inputs.length > 0 && <div className="ax-practice-inputs">{concept.inputs.map(input => <label key={input.key}>{chineseField(input.key, input.label)}
            <input value={usePageContext && Object.prototype.hasOwnProperty.call(contextInputs, input.key) ? JSON.stringify(contextInputs[input.key]) : inputs[input.key] ?? ''} disabled={usePageContext && Object.prototype.hasOwnProperty.call(contextInputs, input.key)} onChange={event => setInputs(current => ({ ...current, [input.key]: event.target.value }))} aria-label={chineseField(input.key, input.label)} />
          </label>)}</div>}
          <button className="ax-btn primary" onClick={run} disabled={loading}>{loading ? '计算中…' : '运行实践'}</button>
        </>}
      </>}
      {result && <div className="ax-practice-result">
        <p className={result.status === 'computed' ? 'positive' : 'negative'}>{result.status === 'computed' ? '已计算' : '无法计算'} · {result.provenance === 'provided_market_bars' ? '使用当前模块行情上下文' : '使用可编辑教学输入'}</p>
        {result.reason && <p>{result.reason}</p>}
        {Object.keys(result.values).length > 0 && <dl>{Object.entries(result.values).map(([key, value]) => <div key={key}><dt>{chineseField(key, key === conceptId ? concept?.name : undefined)}{result.units?.[key] ? `（${chineseUnit(result.units[key])}）` : ''}</dt><dd>{value == null ? '—' : fmtNum(value, 6)}</dd></div>)}</dl>}
        <p className="ax-practice-reading">{resultSentence(concept?.name || '该概念', result.values, result.units)}</p>
        {result.notes.map((note, index) => <p className="ax-practice-note" key={index}>{note}</p>)}
      </div>}
    </aside>
  );
}

// 概念速览
const WORKFLOW_STAGES = [
  { number: '01', group: '数据基础', title: '数据接入与治理', output: '可信 Bar 数据集', detail: '接入行情后，清洗、复权、时间对齐（并处理缺失、重复和交易日历）；记录版本，防止把未来数据带进研究。', code: 'src/data.rs · DataFeed' },
  { number: '02', group: '研究', title: '特征、指标与假设', output: '可检验特征', detail: '从 OHLCV 计算指标与特征，并把“为什么有效”写成可证伪的研究假设。', code: 'src/indicators/*' },
  { number: '03', group: '研究', title: '建模与信号', output: 'Signal', detail: '规则策略、因子模型或机器学习都只产出信号；标签、验证集和泄漏检查必须先于结果。', code: 'src/strategy.rs' },
  { number: '04', group: '组合', title: '风控与仓位', output: 'Order 或拒绝', detail: '止损、止盈、仓位上限和资金约束共同决定信号能否变成订单。', code: 'src/risk.rs · src/portfolio.rs' },
  { number: '05', group: '验证', title: '回测与稳健性', output: '可复现业绩', detail: '按时间滚动验证，计入手续费、滑点和执行延迟；检查样本外表现与极端行情。', code: 'src/engine.rs · src/metrics.rs' },
  { number: '06', group: '运行', title: '模拟执行与上线', output: 'Fill / Equity', detail: '先模拟盘，再以受限权限接入真实执行；当前项目实现的是 SimulatedBroker。', code: 'src/broker.rs · src/paper.rs' },
  { number: '07', group: '运营', title: '上线、监控与复盘', output: '下一轮假设', detail: '持续观察漂移、风险限额和成交质量，解释收益来源后再更新研究。', code: 'src/metrics.rs · src/workflows.rs' },
] as const;

type ConceptModule = typeof CONCEPT_CARDS[number];
function ModuleSketch({ title }: { title: string }) {
  return <svg className="ax-module-sketch" viewBox="0 0 520 126" role="img" aria-label={`${title} 的输入、处理和输出`}>
    <path d="M142 63H205M315 63H378" className="ax-module-line" markerEnd="url(#arrow-head)" />
    <defs><marker id="arrow-head" markerWidth="8" markerHeight="8" refX="7" refY="3" orient="auto"><path d="M0,0 L0,6 L8,3 z" /></marker></defs>
    <rect x="12" y="28" width="130" height="70" rx="14" className="ax-module-input" /><text x="77" y="57" textAnchor="middle">输入</text><text x="77" y="78" textAnchor="middle" className="ax-module-code">市场 / 状态</text>
    <rect x="205" y="20" width="110" height="86" rx="18" className="ax-module-core" /><text x="260" y="55" textAnchor="middle">{title}</text><text x="260" y="78" textAnchor="middle" className="ax-module-code">纯逻辑</text>
    <rect x="378" y="28" width="130" height="70" rx="14" className="ax-module-output" /><text x="443" y="57" textAnchor="middle">输出</text><text x="443" y="78" textAnchor="middle" className="ax-module-code">下一步所需事实</text>
  </svg>;
}

function ModuleSourceLink({ codeRef }: { codeRef: string }) {
  const [url, setUrl] = useState('');
  useEffect(() => { api.getCodeLocation(codeRef).then(location => {
    setUrl(preferredCodeLocationUrl(location));
  }).catch(() => setUrl('')); }, [codeRef]);
  return url ? <a className="ax-btn primary" href={url} target="_blank" rel="noreferrer">打开精确 GitHub 源码 ↗</a> : <span className="ax-module-source-pending">正在定位构建时源码…</span>;
}

function ConceptsView() {
  const [selected, setSelected] = useState<ConceptModule | null>(null);
  useEffect(() => {
    if (!selected) return;
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === 'Escape') setSelected(null); };
    window.addEventListener('keydown', closeOnEscape);
    return () => window.removeEventListener('keydown', closeOnEscape);
  }, [selected]);
  return <div className="ax-concepts">
    <header className="ax-concepts-intro"><p className="ax-eyebrow">从数据到运营的闭环</p><h3>量化交易完整流程</h3><p className="ax-lead">这是一条研究与运行并行的闭环：每一步都有可检查的产物，任何模型都要经过数据、验证和风控。</p></header>
    <section className="ax-workflow" aria-label="量化交易全流程">
      <div className="ax-workflow-caption"><span>研究与决策管线</span><small>真实交易时序：收盘数据 t → 信号 t → 下一根可交易时点执行</small></div>
      <ol className="ax-workflow-stages">{WORKFLOW_STAGES.map((stage, index) => <li key={stage.number} className="ax-workflow-stage">
        <span className="ax-stage-number">{stage.number}</span><span className="ax-stage-group">{stage.group}</span><h4>{stage.title}</h4><p>{stage.detail}</p><span className="ax-stage-output">{stage.output}</span><code>{stage.code}</code>{index < WORKFLOW_STAGES.length - 1 && <span className="ax-workflow-arrow" aria-hidden="true">→</span>}
      </li>)}</ol>
      <div className="ax-ai-lane"><strong>机器学习与大模型</strong><span>ML 用于特征、标签、概率与漂移检测；LLM 用于资料检索、结构化和研究辅助。两者都只能进入研究管线，必须通过泄漏检查、回测和风控，不能直接下单。</span></div>
      <div className="ax-event-loop"><span>事件循环</span><b>已收盘 K 线</b><i>→</i><b>Signal</b><i>→</i><b>下一时点 Order / Fill</b><i>→</i><b>盯市 Equity</b><i>↺</i><em>归因后更新假设</em></div>
    </section>
    <h3>真实代码模块</h3><p className="ax-lead">每张卡都对应当前仓库已实现的模块。点开可看职责、输入输出与实现约束；来源链接以构建时的提交 SHA 定位。</p>
    <div className="ax-module-grid">{CONCEPT_CARDS.map(card => <button type="button" className="ax-module-card" key={card.title} onClick={() => setSelected(card)} aria-label={`${card.title}，查看模块说明`}><span className="ax-module-path">{card.path}</span><h4>{card.title}</h4><p>{card.desc}</p><code>{card.code}</code><span className="ax-module-more">查看模块说明 <b>→</b></span></button>)}</div>
    {selected && <div className="ax-module-backdrop" onMouseDown={event => { if (event.target === event.currentTarget) setSelected(null); }}><section className="ax-module-dialog" role="dialog" aria-modal="true" aria-labelledby="module-dialog-title"><button className="ax-module-close" type="button" onClick={() => setSelected(null)} aria-label="关闭模块说明">×</button><p className="ax-eyebrow">当前项目中的真实对应</p><h3 id="module-dialog-title">{selected.title} 模块说明</h3><p className="ax-module-dialog-lead">{selected.desc}</p><ModuleSketch title={selected.title} /><div className="ax-module-facts"><div><span>代码位置</span><code>{selected.path}</code></div><div><span>接口 / 核心对象</span><code>{selected.code}</code></div><div><span>实现约束</span><p>{selected.constraint}</p></div></div><ModuleSourceLink codeRef={selected.codeRef} /></section></div>}
  </div>;
}
const CONCEPT_CARDS = [
  { title: 'K线 (Bar)', desc: '一根柱子记录一段已完成时间内的开、高、低、收与成交量，是后续计算的统一事实。', code: 'Bar { timestamp, open, high, low, close, volume }', path: 'src/types.rs', codeRef: 'src/types.rs::Bar', constraint: '只把已收盘的 Bar 交给策略，避免把尚未知道的价格当作事实。' },
  { title: '数据源 (DataFeed)', desc: '把合成、CSV 与 Binance HTTP 行情收敛为同一个数据接口，便于教学、回测和替换来源。', code: 'DataFeed / AsyncDataFeed', path: 'src/data.rs', codeRef: 'src/data.rs::DataFeed', constraint: '当前是 CSV 缓存与内存计算；持久化分析层会按查询量演进，不能把 HTTP 行情当成券商。' },
  { title: '信号 (Signal)', desc: '策略对一根 K 线的可解释想法：买、卖或观望，以及强度、原因和目标仓位。', code: 'on_bar(&Bar) -> Signal', path: 'src/strategy.rs', codeRef: 'src/strategy.rs::Strategy', constraint: 'Signal 只表达意图，不触碰现金、持仓或外部下单。' },
  { title: '风控门控', desc: '风险管理检查止损、止盈和仓位上限，必要时拒绝订单或触发强平。', code: 'RiskManager::allow_order', path: 'src/risk.rs', codeRef: 'src/risk.rs::RiskManager::allow_order', constraint: '风控不是收益预测器；它只处理明确的账户与市场约束。' },
  { title: '组合与仓位', desc: '组合模块把允许的信号换成符合资金与目标仓位约束的订单。', code: 'Portfolio::on_signal', path: 'src/portfolio.rs', codeRef: 'src/portfolio.rs::Portfolio::on_signal', constraint: '重复买入、空仓卖出和低于最小交易量的订单都会被拒绝。' },
  { title: '订单与成交', desc: '订单是给 Broker 的明确指令；成交会带回实际价格、手续费与滑点，改变现金和持仓。', code: 'Broker::place_order -> Fill', path: 'src/broker.rs', codeRef: 'src/broker.rs::Broker', constraint: '当前真实实现为 SimulatedBroker；HttpFeed 是行情源，实盘 Broker adapter 尚未实现。' },
  { title: '回测引擎', desc: '按事件顺序驱动数据、策略、风控、组合和成交，并保留 pending signal 来避免未来函数。', code: 'BacktestEngine::run', path: 'src/engine.rs', codeRef: 'src/engine.rs::BacktestEngine::run', constraint: '信号在 t 生成，在下一可交易时点执行；回测必须带成本和样本外验证。' },
  { title: '净值与指标', desc: '用净值曲线与交易记录计算收益、回撤、夏普、索提诺等，检验结果是否经得起风险调整。', code: 'compute_metrics(equity_curve, trades)', path: 'src/metrics.rs', codeRef: 'src/metrics.rs::compute_metrics', constraint: '单一收益数字不足以证明策略有效，必须同时看回撤、分布与交易成本。' },
  { title: '模拟盘生命周期', desc: '模拟盘把状态、待执行信号和行情循环组织为一段可观察的运行过程。', code: 'run_paper_loop', path: 'src/paper.rs', codeRef: 'src/paper.rs::run_paper_loop', constraint: '当前状态是单进程内存；进程重启不应被误解为已具备持久恢复。' },
  { title: '知识实践工作流', desc: '把书中概念转为有输入、结果与边界说明的教学实践，而不是未经验证的交易建议。', code: 'workflows::entries', path: 'src/workflows.rs', codeRef: 'src/workflows.rs::entries', constraint: '教学计算会说明使用市场数据还是可编辑例子，不把教学值伪装成行情结论。' },
] as const;

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
const LEARNING_PATH = [
  { title: '统一词汇与数据契约', learn: '理解 Bar、Signal、Order、Fill、Position 和 Equity 如何串成事件链。', practice: '在概念速览里打开 K线与信号模块，确认字段与含义。', codeRef: 'src/types.rs::Bar' },
  { title: '获取、校验与版本化行情', learn: '分清合成、CSV 缓存和 HTTP 行情；知道清洗、复权、时区对齐和数据版本为什么先于指标。', practice: '在数据探索切换数据源，比较同一指标在不同输入下的表现。', codeRef: 'src/data.rs::DataFeed' },
  { title: '指标与可证伪假设', learn: '指标是特征，不是交易建议；预热期为空是诚实的数据状态。', practice: '在指标大全展开一个概念，再用“在数据探索中实践”重算。', codeRef: 'src/indicators/ma.rs::sma' },
  { title: '策略只产生意图', learn: 'Strategy 读取一根已完成 K 线，输出 Signal；它不能直接改持仓。', practice: '阅读均线交叉策略，写下一个可验证的入场与退出条件。', codeRef: 'src/strategy.rs::Strategy' },
  { title: '风控、仓位与订单', learn: '组合把信号换为订单，风险模块会拒绝或强平不符合约束的订单。', practice: '在回测调整止损、止盈和仓位，观察风险调整后指标。', codeRef: 'src/portfolio.rs::Portfolio::on_signal' },
  { title: '成交模拟与防未来函数', learn: '信号在 t 产生，下一可交易时点才执行；手续费与滑点属于成交事实。', practice: '运行回测，核对交易与净值曲线是否按事件顺序变化。', codeRef: 'src/engine.rs::BacktestEngine::run' },
  { title: '业绩、样本外与稳健性', learn: '收益须和回撤、夏普、索提诺、交易成本一起阅读；样本外和滚动验证防止挑样本。', practice: '在策略对比中选择多个策略，写下不会只看收益率的判断理由。', codeRef: 'src/metrics.rs::compute_metrics' },
  { title: '模拟盘、服务与验证', learn: '模拟盘复用事件链但当前状态在单进程内存；上线前要有可复现测试和运行监控。', practice: '启动模拟盘并检查日志、持仓、净值；阅读端到端测试如何覆盖用户路径。', codeRef: 'src/paper.rs::run_paper_loop' },
  { title: '研究扩展：ML / LLM 与数据存储', learn: 'ML/LLM 只能辅助研究；随着历史数据增长，再引入 Parquet + DuckDB，持久状态优先 SQLite。', practice: '先完成数据泄漏、回测和风险检查，再评估是否需要新增存储或模型。', codeRef: 'src/workflows.rs::entries' },
] as const;
function PathView() {
  return <div><h3>学习路径</h3><p className="ax-lead">从数据契约到可复现运行，共 9 步。每步都给出真实模块、一个动手验证和可定位的 GitHub 源码。</p><ol className="ax-path ax-learning-path">{LEARNING_PATH.map((step, i) => <li key={step.title}><p className="ax-path-kicker">第 {i + 1} 步</p><h4>{step.title}</h4><p>{step.learn}</p><div className="ax-path-practice"><strong>动手验证</strong><span>{step.practice}</span></div><ModuleSourceLink codeRef={step.codeRef} /></li>)}</ol></div>;
}

// ===================================================================
// 数据探索
// ===================================================================
function DataExplore({ targetConcept, theme }: { targetConcept?: string; theme: 'light' | 'dark' }) {
  const [symbol, setSymbol] = useState('');
  const [customSymbol, setCustomSymbol] = useState('');
  const [limit, setLimit] = useState(200);
  const [source, setSource] = useState<SourceType>('binance');
  const [chartType, setChartType] = useState<ChartType>('candle');
  const [selectedIndicators, setSelectedIndicators] = useState<string[]>(['sma_20', 'rsi_14', 'bbands_20', 'macd', 'atr_14']);
  const appliedIndicators = useRef<string[]>(['sma_20', 'rsi_14', 'bbands_20', 'macd', 'atr_14']);
  const [chartData, setChartData] = useState<{ bars: Bar[]; indicators: Record<string, Array<{ x: string; y: number } | null>>; symbol: string; source: string } | null>(null);
  const [summary, setSummary] = useState<{ open: number; close: number; high: number; low: number; return_pct: number; avg_volume: number; count: number; source: string } | null>(null);
  const [patterns, setPatterns] = useState<Array<{ pattern: string; timestamp: string }>>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const chartRef = useRef<HTMLDivElement>(null);
  const requestId = useRef(0);

  useEffect(() => {
    setSymbol(previous => previous || (source === 'binance' ? 'BTCUSDT' : source === 'a_share' ? '600519' : 'AAPL'));
  }, [source]);

  const loadData = useCallback(async (requestedIndicators = appliedIndicators.current) => {
    const id = ++requestId.current;
    const sym = (customSymbol || symbol || 'BTCUSDT').toUpperCase();
    setLoading(true);
    setError('');
    try {
      const data = await api.getIndicators(sym, requestedIndicators.join(','), limit, source);
      if (chartType === 'heikin_ashi') {
        try {
          const ha = await api.getHeikinAshi(sym, limit, source);
          if (ha.bars) data.bars = ha.bars;
        } catch {}
      }
      if (id !== requestId.current) return;
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
        if (id === requestId.current) setPatterns(pat.patterns.filter(p => p.pattern !== '无特殊形态').slice(0, 5));
      } catch {}
    } catch (e) {
      if (id !== requestId.current) return;
      setError(String(e));
      setChartData(null);
      setSummary(null);
      setPatterns([]);
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }, [symbol, customSymbol, limit, source, chartType]);

  useEffect(() => { if (symbol) loadData(); }, [loadData, symbol]);

  // 画图
  useEffect(() => {
    if (!chartData) return;
    const chart = chartRef.current;
    if (!chart) return;
    if (chartData.bars.length === 0) {
      Plotly.purge(chart);
      chart.replaceChildren(Object.assign(document.createElement('p'), { className: 'ax-chart-empty', textContent: '没有可用的 K 线数据。请调整交易对、数据源或数量后重试。' }));
      return;
    }
    chart.querySelector('.ax-chart-empty')?.remove();
    const colors = plotTheme();
    const traces: any[] = [{
      x: chartData.bars.map(b => new Date(b.timestamp)),
      open: chartData.bars.map(b => b.open),
      high: chartData.bars.map(b => b.high),
      low: chartData.bars.map(b => b.low),
      close: chartData.bars.map(b => b.close),
      type: 'candlestick', name: chartData.symbol,
      increasing: { line: { color: colors.green } },
      decreasing: { line: { color: colors.red } },
    }];
    if (chartType === 'heikin_ashi') {
      traces[0].name = chartData.symbol + ' (HA)';
    }
    const usedPanels = new Set<IndicatorPanel>();
    if (chartData.indicators) {
      const palette = colors.palette;
      let ci = 0;
      for (const [name, vals] of Object.entries(chartData.indicators)) {
        const series = validSeries(vals);
        if (!series.some(Boolean)) continue;
        const panel = indicatorPanel(name);
        usedPanels.add(panel);
        traces.push({
          x: series.map(v => v ? new Date(v.x) : null),
          y: series.map(v => v ? v.y : null),
          type: 'scatter', mode: 'lines', name,
          line: { color: palette[ci++ % palette.length], width: 1.5 },
          connectgaps: false,
          yaxis: panel === 'price' ? 'y' : `y${['oscillator', 'momentum', 'volume', 'volatility'].indexOf(panel) + 2}`,
        });
      }
    }
    const panelLayout: Record<Exclude<IndicatorPanel, 'price'>, { key: string; title: string }> = {
      oscillator: { key: 'yaxis2', title: '振荡器' },
      momentum: { key: 'yaxis3', title: '动量' },
      volume: { key: 'yaxis4', title: '量能' },
      volatility: { key: 'yaxis5', title: '波动率' },
    };
    const subPanels = (Object.keys(panelLayout) as Exclude<IndicatorPanel, 'price'>[]).filter(panel => usedPanels.has(panel));
    const height = 460 + subPanels.length * 80;
    chart.style.height = `${height}px`;
    const layout: any = {
      height,
      autosize: true,
      paper_bgcolor: colors.paper, plot_bgcolor: colors.plot,
      font: { color: colors.text, family: 'system-ui', size: 11 },
      margin: { t: 30, b: 90, l: 55, r: 30 },
      xaxis: { ...dateAxis(colors.grid), anchor: 'free', position: 0, rangeslider: { visible: false } },
      yaxis: { gridcolor: colors.grid, title: '价格', domain: [subPanels.length * 0.17, 1], fixedrange: false },
      legend: { orientation: 'h', y: -0.13 },
      hovermode: 'x unified',
      hoverlabel: { bgcolor: colors.paper, bordercolor: colors.grid, font: { color: colors.text } },
    };
    for (const [index, panel] of subPanels.entries()) {
      const spec = panelLayout[panel];
      const bottom = (subPanels.length - index - 1) * 0.17;
      layout[spec.key] = { gridcolor: colors.grid, title: spec.title, domain: [bottom, bottom + 0.13], anchor: 'x', fixedrange: false };
    }
    Plotly.react(chart, traces, layout, { responsive: true, displayModeBar: false });
    return () => Plotly.purge(chart);
  }, [chartData, chartType, theme]);

  return (
    <section className="ax-section">
      <h2>数据探索</h2>
      <p className="ax-lead">真实行情：加密货币、A 股与美股；支持指标叠加和形态识别。</p>
      <div className="ax-controls">
        <label>交易对
          <SymbolPicker source={source} value={symbol} onChange={setSymbol} />
        </label>
        <label>K 线数
          <input type="number" value={limit} onChange={e => setLimit(parseInt(e.target.value) || 200)}
            min={50} max={2000} step={50} />
        </label>
        <label>数据源
          <Dropdown label="数据源" options={MARKET_SOURCE_OPTIONS}
            value={source} onChange={(v: SourceType) => setSource(v)} minWidth={140} />
        </label>
        <label>图表类型
          <Dropdown label="图表类型" options={[{v:'candle',l:'标准 K 线'},{v:'heikin_ashi',l:'Heikin Ashi'}]}
            value={chartType} onChange={(v: ChartType) => setChartType(v)} minWidth={140} />
        </label>
        <label>指标叠加
          <MultiSelectDropdown label="指标叠加" options={INDICATOR_OPTIONS} values={selectedIndicators} onChange={setSelectedIndicators} minWidth={210} />
        </label>
        <button className="ax-btn primary" onClick={() => { appliedIndicators.current = selectedIndicators; loadData(selectedIndicators); }} disabled={loading}>
          {loading ? '加载中...' : '加载数据'}
        </button>
      </div>
      {error && <div className="ax-error">{error}</div>}
      <div ref={chartRef} className="ax-chart"></div>
      {summary && (
        <div className="ax-summary">
          <table>
            <tbody>
              <tr><th>标的</th><td>{chartData?.symbol}</td><th>来源</th><td>{MARKET_SOURCE_LABEL[summary.source] || summary.source}</td></tr>
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
      <PracticePanel module="data" symbol={chartData?.symbol || symbol} source={source} limit={limit} bars={chartData?.bars} targetConcept={targetConcept} />
    </section>
  );
}

// ===================================================================
// 回测
// ===================================================================
function Backtest({ targetConcept, theme }: { targetConcept?: string; theme: 'light' | 'dark' }) {
  const [strategies, setStrategies] = useState<StrategyMeta[]>([]);
  const [strategy, setStrategy] = useState('');
  const [params, setParams] = useState<Record<string, number>>({});
  const [symbol, setSymbol] = useState('');
  const [source, setSource] = useState<SourceType>('binance');
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
    setSymbol(previous => previous || (source === 'binance' ? 'BTCUSDT' : source === 'a_share' ? '600519' : 'AAPL'));
  }, [source]);

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
    const chart = chartRef.current;
    const colors = plotTheme();
    Plotly.react(chart, [{
      x: result.equity_curve.map(p => new Date(p.timestamp)),
      y: result.equity_curve.map(p => p.equity),
      type: 'scatter', mode: 'lines', name: '净值',
      line: { color: colors.accent, width: 1.8 },
      fill: 'tozeroy', fillcolor: colors.fill,
    }], {
      paper_bgcolor: colors.paper, plot_bgcolor: colors.plot,
      font: { color: colors.text, family: 'system-ui', size: 11 },
      margin: { t: 30, b: 40, l: 60, r: 20 },
      xaxis: dateAxis(colors.grid),
      yaxis: { gridcolor: colors.grid, zerolinecolor: colors.grid, title: '净值 ($)' },
      hoverlabel: { bgcolor: colors.paper, bordercolor: colors.grid, font: { color: colors.text } },
    }, { responsive: true, displayModeBar: false });
    return () => Plotly.purge(chart);
  }, [result, theme]);

  const currentMeta = strategies.find(s => s.name === strategy);

  return (
    <section className="ax-section">
      <h2>回测</h2>
      <p className="ax-lead">在所选真实市场的数据上跑策略，观察业绩指标。</p>
      <div className="ax-controls">
        <label>策略
          <Dropdown label="策略" options={strategies.map(s => ({ v: s.name, l: s.display_name }))}
            value={strategy} onChange={setStrategy} minWidth={200} />
        </label>
        <label>交易对
          <SymbolPicker source={source} value={symbol} onChange={setSymbol} />
        </label>
        <label>数据源
          <Dropdown label="数据源" options={MARKET_SOURCE_OPTIONS}
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
              const raw = result.metrics[f.key];
              const v = typeof raw === 'number' ? raw : null;
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
          {typeof result.metrics['指标说明'] === 'object' && result.metrics['指标说明'] !== null && <details className="ax-trades ax-metric-notes">
            <summary>指标说明</summary>
            <p>显示“—”表示该指标在当前样本中无定义，不等于零。</p>
            {Object.entries(result.metrics['指标说明']).map(([key, note]) => <p key={key}><b>{key}</b>：{String(note)}</p>)}
          </details>}
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
                      <td className={t.pnl_pct >= 0 ? 'positive' : 'negative'}>{fmtPct(t.pnl_pct)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </details>
          )}
        </>
      )}
      <PracticePanel module="backtest" symbol={symbol} source={source} limit={limit} bars={result?.bars} contextInputs={performanceInputs(result?.equity_curve, capital)} targetConcept={targetConcept} />
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
function PaperTrading({ targetConcept, theme }: { targetConcept?: string; theme: 'light' | 'dark' }) {
  const [snapshot, setSnapshot] = useState<PaperSnapshot | null>(null);
  const [strategies, setStrategies] = useState<StrategyMeta[]>([]);
  const [strategy, setStrategy] = useState('');
  const [error, setError] = useState('');
  const chartRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const selectedByUser = useRef(false);

  useEffect(() => {
    api.listStrategies().then(d => {
      setStrategies(d.strategies);
      if (d.strategies.length > 0) setStrategy(current => current || d.strategies[0].name);
    }).catch(e => setError(String(e)));
    api.paperSnapshot().then(setSnapshot).catch(e => setError(String(e)));
  }, []);

  useEffect(() => {
    if (snapshot?.strategy && !selectedByUser.current) setStrategy(snapshot.strategy);
  }, [snapshot?.strategy]);

  // WebSocket
  useEffect(() => {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${proto}://${location.host}${appPath('/api/paper/ws')}`);
    wsRef.current = ws;
    ws.onmessage = (e) => {
      try { setSnapshot(JSON.parse(e.data)); } catch {}
    };
    ws.onclose = () => { wsRef.current = null; };
    return () => ws.close();
  }, []);

  useEffect(() => {
    if (!snapshot || !chartRef.current) return;
    if (snapshot.equity_curve.length === 0) return;
    const chart = chartRef.current;
    const colors = plotTheme();
    Plotly.react(chart, [{
      x: snapshot.equity_curve.map((p: EquityPoint) => new Date(p.timestamp)),
      y: snapshot.equity_curve.map((p: EquityPoint) => p.equity),
      type: 'scatter', mode: 'lines',
      line: { color: colors.accent, width: 1.8 },
      fill: 'tozeroy', fillcolor: colors.fill,
    }], {
      paper_bgcolor: colors.paper, plot_bgcolor: colors.plot,
      font: { color: colors.text, family: 'system-ui', size: 11 },
      margin: { t: 30, b: 40, l: 60, r: 20 },
      xaxis: dateAxis(colors.grid),
      yaxis: { gridcolor: colors.grid, zerolinecolor: colors.grid, title: '净值 ($)' },
      hoverlabel: { bgcolor: colors.paper, bordercolor: colors.grid, font: { color: colors.text } },
    }, { responsive: true, displayModeBar: false });
    return () => Plotly.purge(chart);
  }, [snapshot, theme]);

  const start = async () => {
    try {
      setError('');
      await api.paperStrategy(strategy);
      await api.paperStart();
      setSnapshot(await api.paperSnapshot());
    } catch (e) { setError(String(e)); }
  };
  const stop = async () => {
    try {
      setError('');
      await api.paperStop();
      setSnapshot(await api.paperSnapshot());
    } catch (e) { setError(String(e)); }
  };

  if (!snapshot) return <div className={error ? 'ax-error' : 'ax-loading'}>{error || '加载中…'}</div>;

  return (
    <section className="ax-section">
      <h2>模拟盘</h2>
      <p className="ax-lead">使用已收盘行情推进模拟账户；状态每 2 秒更新。</p>
      <div className="ax-controls">
        <label>策略
          <Dropdown label="策略" options={strategies.map(s => ({ v: s.name, l: s.display_name }))}
            value={strategy} onChange={value => { selectedByUser.current = true; setStrategy(value); }} minWidth={200} />
        </label>
        <button className="ax-btn primary" onClick={start} disabled={snapshot.is_running}>▶ 启动</button>
        <button className="ax-btn" onClick={stop} disabled={!snapshot.is_running}>■ 停止</button>
        <span className={'ax-status ' + (snapshot.is_running ? 'on' : 'off')}>
          {snapshot.is_running ? '运行中' : '已停止'}
        </span>
      </div>
      <p className="ax-practice-note">账户策略：{strategies.find(item => item.name === snapshot.strategy)?.display_name || snapshot.strategy || '—'}。启动将采用上方选择的策略；已有持仓和资金会保留。</p>
      {error && <div className="ax-error">{error}</div>}
      <div className="ax-paper-stats">
        <div className="ax-stat"><div className="ax-stat-label">现金</div><div className="ax-stat-value">{fmtMoney(snapshot.cash)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">持仓价值</div><div className="ax-stat-value">{fmtMoney(snapshot.position_value)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">总净值</div><div className="ax-stat-value">{fmtMoney(snapshot.equity)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">持仓数量</div><div className="ax-stat-value">{fmtNum(snapshot.position_size, 4)}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">成交笔数</div><div className="ax-stat-value">{snapshot.trades_count}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">交易对</div><div className="ax-stat-value">{snapshot.symbol || '—'}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">数据源</div><div className="ax-stat-value">{MARKET_SOURCE_LABEL[snapshot.source || ''] || '—'}</div></div>
        <div className="ax-stat"><div className="ax-stat-label">最新价</div><div className="ax-stat-value">{snapshot.current_bar ? fmtNum(snapshot.current_bar.close) : '—'}</div></div>
      </div>
      <div ref={chartRef} className="ax-chart"></div>
      <details className="ax-trades" open>
        <summary>运行日志</summary>
        <div className="ax-log">
          {snapshot.log.slice(0, 30).map((l: { timestamp: string; level: string; message: string }, i: number) => (
            <div className="ax-log-entry" key={i}>
              <span className="ax-log-ts">{new Date(l.timestamp).toLocaleTimeString()}</span>
              <span className={'ax-log-lvl ' + l.level.toLowerCase()}>{l.level}</span>
              <span className="ax-log-msg">{l.message}</span>
            </div>
          ))}
        </div>
      </details>
      <PracticePanel module="paper" symbol={snapshot.symbol || 'BTCUSDT'} source={snapshot.source || 'binance'} limit={snapshot.bars?.length || 200} bars={snapshot.bars} contextInputs={performanceInputs(snapshot.equity_curve, snapshot.initial_capital ?? snapshot.equity)} targetConcept={targetConcept} />
    </section>
  );
}

// ===================================================================
// 策略对比
// ===================================================================
function CompareStrategies({ targetConcept, theme }: { targetConcept?: string; theme: 'light' | 'dark' }) {
  const [strategies, setStrategies] = useState<StrategyMeta[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [customStrategies, setCustomStrategies] = useState<CustomStrategy[]>([]);
  const [symbol, setSymbol] = useState('');
  const [source, setSource] = useState<SourceType>('binance');
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
    setSymbol(previous => previous || (source === 'binance' ? 'BTCUSDT' : source === 'a_share' ? '600519' : 'AAPL'));
  }, [source]);

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
    let sharedBars: Bar[] | undefined;
    for (const item of selected) {
      let req;
      if (item.startsWith('custom_')) {
        const cs = customStrategies[parseInt(item.replace('custom_', ''))];
        req = { strategy: cs.base, params: cs.params, symbol, source, limit: 500, initial_capital: capital, stop_loss_pct: cs.sl, bars: sharedBars };
      } else {
        const meta = strategies.find(s => s.name === item);
        const params: Record<string, number> = {};
        if (meta) meta.params.forEach((p: any) => params[p.key] = p.default);
        req = { strategy: item, params, symbol, source, limit: 500, initial_capital: capital, bars: sharedBars };
      }
      try {
        const r = await api.runBacktest(req);
        sharedBars ??= r.bars;
        list.push({ name: item, result: r });
      } catch (e) { console.error('策略失败:', item, e); }
    }
    setResults(list);
    setLoading(false);
  };

  useEffect(() => {
    if (!results.length || !chartRef.current) return;
    const chart = chartRef.current;
    const themeColors = plotTheme();
    const colors = themeColors.palette;
    const names: Record<string, string> = {};
    strategies.forEach(s => { names[s.name] = s.display_name; });
    customStrategies.forEach((cs, i) => { names['custom_' + i] = '✦ ' + cs.name; });
    Plotly.react(chart, results.map((r, i) => ({
      x: r.result.equity_curve.map(p => new Date(p.timestamp)),
      y: r.result.equity_curve.map(p => p.equity),
      type: 'scatter', mode: 'lines',
      name: names[r.name] || r.name,
      line: { color: colors[i % colors.length], width: 1.8, dash: i >= colors.length ? 'dash' : 'solid' },
    })), {
      paper_bgcolor: themeColors.paper, plot_bgcolor: themeColors.plot,
      font: { color: themeColors.text, family: 'system-ui', size: 11 },
      margin: { t: 30, b: 40, l: 60, r: 20 },
      xaxis: dateAxis(themeColors.grid),
      yaxis: { gridcolor: themeColors.grid, zerolinecolor: themeColors.grid, title: '净值 ($)' },
      legend: { orientation: 'h', y: -0.15 },
      hoverlabel: { bgcolor: themeColors.paper, bordercolor: themeColors.grid, font: { color: themeColors.text } },
    }, { responsive: true, displayModeBar: false });
    return () => Plotly.purge(chart);
  }, [results, strategies, customStrategies, theme]);

  return (
    <section className="ax-section">
      <h2>策略对比</h2>
      <p className="ax-lead">勾选策略,PK 同一段历史数据上谁最强。可自定义。</p>
      <div className="ax-controls">
        <label>交易对
          <SymbolPicker source={source} value={symbol} onChange={setSymbol} />
        </label>
        <label>数据源
          <Dropdown label="数据源" options={MARKET_SOURCE_OPTIONS}
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
                <Dropdown label="基础策略" options={strategies.filter(s => s.name !== 'random').map(s => ({ v: s.name, l: s.display_name }))}
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
          <div className="ax-cmp-table" role="region" aria-label="策略业绩对比，可横向滚动" tabIndex={0}>
          <table>
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
                      const raw = r.result.metrics[f.key];
                      const v = typeof raw === 'number' ? raw : null;
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
          </div>
          <div ref={chartRef} className="ax-chart"></div>
        </>
      )}
      <PracticePanel module="compare" symbol={symbol} source={source} limit={500} bars={results[0]?.result.bars} contextInputs={performanceInputs(results[0]?.result.equity_curve, capital)} targetConcept={targetConcept} />
    </section>
  );
}

// ===================================================================
// 主应用
// ===================================================================
export default function App() {
  const [route, setRoute] = useState(readRoute);
  const { tab, sub: learnSub, concept: practiceTarget } = route;
  const navigate = (nextTab: TabId, nextSub: LearnSub = 'knowledge', concept?: string) => { window.history.pushState({}, '', routeFor(nextTab, nextSub, concept)); setRoute(readRoute()); };
  const openPractice = (conceptId: string) => navigate('data', 'knowledge', conceptId);
  const [theme, setTheme] = useState<'light' | 'dark'>(() => localStorage.getItem('axiom-theme') === 'light' ? 'light' : 'dark');
  void practiceTarget;
  void navigate;
  // URL routing owns the selected practice concept.
  // Kept as declarative routing state above.
  // No secondary practice selector is rendered.
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem('axiom-theme', theme);
  }, [theme]);

  useEffect(() => {
    const onPopState = () => setRoute(readRoute());
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  }, []);
  return (
    <div className="ax-app">
      <Header theme={theme} onToggleTheme={() => setTheme(current => current === 'dark' ? 'light' : 'dark')} />
      <TabBar active={tab} onChange={nextTab => navigate(nextTab)} />
      <main className="ax-main">
        {tab === 'learn' && <LearnCenter sub={learnSub} onSubChange={nextSub => navigate('learn', nextSub)} onPractice={openPractice} />}
        {tab === 'data' && <DataExplore targetConcept={practiceTarget} theme={theme} />}
        {tab === 'backtest' && <Backtest targetConcept={practiceTarget} theme={theme} />}
        {tab === 'paper' && <PaperTrading targetConcept={practiceTarget} theme={theme} />}
        {tab === 'compare' && <CompareStrategies targetConcept={practiceTarget} theme={theme} />}
      </main>
      <footer className="ax-footer">
        AXIOM · 仅供学习,不构成任何投资建议
      </footer>
    </div>
  );
}
