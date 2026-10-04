import { useEffect, useMemo, useState } from 'react';
import { appPath } from './api';
import './MarketBreadthVisual.css';

export type BreadthConceptId = 'ad_line' | 'trin' | 'mcclellan' | 'new_high_low' | 'tick' | 'breadth_thrust' | 'bullish_percent' | 'up_down_volume';
type BreadthPracticeId = BreadthConceptId | 'book_mcclellan_sum';
export interface BreadthSeriesPoint { date: string; value: number; eligible_members: number; }
export interface BreadthObservation { date: string; eligible_members: number; advances: number; declines: number; unchanged: number; up_volume: number; down_volume: number; new_highs: number | null; new_lows: number | null; }
export interface BreadthConcept { id: BreadthPracticeId; status: 'available' | 'unavailable'; latest: number | null; triggered: boolean | null; unit: string; series: BreadthSeriesPoint[]; reason: string | null; inputs: Record<string, unknown>; definition: string; }
export interface MarketBreadthSnapshot {
  schema_version: number;
  as_of: string;
  universe: { id: string; name: string; member_count: number; constituents_as_of: string; constituents_source: string; symbols: string[]; scope_note: string; calendar: string; missing_policy: string; };
  coverage: { latest_eligible_members: number; total_members: number; minimum_required: number; };
  source: { retrieval: string; retrieved_at?: string; cache_status?: string; members: Array<{ symbol: string; provider: string; endpoint: string; price_basis: string; corporate_actions?: string; split_event_dates?: string[] }>; };
  observations: BreadthObservation[];
  concepts: BreadthConcept[];
}

const META: Record<BreadthPracticeId, { name: string; short: string; explanation: string }> = {
  ad_line: { name: 'Advance–Decline Line 腾落线', short: 'A–D Line', explanation: '把每日上涨家数减下跌家数累加，观察指数表面之下的参与广度。' },
  trin: { name: 'TRIN / Arms Index', short: 'TRIN', explanation: '将涨跌家数比与涨跌成交量比相除，比较家数与资金方向是否一致。' },
  mcclellan: { name: 'McClellan Oscillator', short: 'McClellan', explanation: '用净上涨家数的 19 日与 39 日 EMA 之差衡量市场宽度动量。' },
  book_mcclellan_sum: { name: 'McClellan Summation Index 麦克莱伦累积指数', short: 'McClellan Sum', explanation: '从第一天有定义的 McClellan Oscillator 开始逐日累加，观察宽度动量的累积方向。' },
  new_high_low: { name: '新高 / 新低比', short: 'NH / NL', explanation: '比较同一成员集合中的阶段新高与阶段新低数量。' },
  tick: { name: 'TICK 指数', short: 'TICK', explanation: '同一时刻上跳价成员数减下跳价成员数，必须来自同步逐笔成交。' },
  breadth_thrust: { name: 'Zweig Breadth Thrust', short: 'Thrust', explanation: '观察上涨家数占比的 10 日 EMA 是否在 10 个观察内从低于 0.4 升至高于 0.615。' },
  bullish_percent: { name: 'Bullish Percent Index', short: 'BPI', explanation: '统计按明确点数图买入信号定义处于多头的成员占比。' },
  up_down_volume: { name: '上涨 / 下跌成交量比', short: 'Up / Down Vol', explanation: '汇总上涨成员与下跌成员的原始成交量后求比率。' },
};

const ids: BreadthConceptId[] = ['ad_line', 'trin', 'mcclellan', 'new_high_low', 'tick', 'breadth_thrust', 'bullish_percent', 'up_down_volume'];
const pct = (value: number) => `${(value * 100).toFixed(1)}%`;
const number = (value: number) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 4 }).format(value);
const universeLabel = (snapshot: MarketBreadthSnapshot) => snapshot.universe.id === 'dow_30_2024_11_08' ? '2024-11-08 道指 30 股固定篮子' : snapshot.universe.name;
const utc = (value: string) => { const date = new Date(value); return Number.isNaN(date.valueOf()) ? value : `${date.toISOString().slice(0, 16).replace('T', ' ')} UTC`; };
const unitLabel = (unit: string) => ({ issues: '家数', ratio: '比率', fraction: '比例', percent: '百分比' } as Record<string, string>)[unit] || unit;
const unavailableReason = (concept: BreadthConcept) => concept.id === 'tick'
  ? '日线 OHLCV 没有同一时刻的逐笔 uptick / downtick，日间涨跌不能冒充交易所 TICK。'
  : concept.id === 'bullish_percent'
    ? '只有完成明确的 close-only 对数 1% box、3-box reversal 点数图状态后才能计算；未形成信号的成员不能当作卖出。'
    : concept.reason || '数据源没有提供完成该定义所需的字段。';
const observationCoverage = (item: BreadthObservation, members: number) => item.eligible_members / (members || 1);

function TrendChart({ concept, observations, members }: { concept: BreadthConcept; observations: BreadthObservation[]; members: number }) {
  const byDate = new Map(concept.series.map(point => [point.date, point]));
  const series = (concept.series.length ? observations.map(observation => {
    const point = byDate.get(observation.date);
    return { date: observation.date, value: point?.value ?? null, coverage: observationCoverage(observation, members) };
  }) : []).slice(-60);
  const finite = series.filter((point): point is typeof point & { value: number } => point.value != null && Number.isFinite(point.value));
  if (!finite.length) return <div className="ax-breadth-empty" role="status">接口没有返回可绘制的历史序列；当前值仍保留其数据时点。</div>;
  const lo = Math.min(...finite.map(point => point.value));
  const hi = Math.max(...finite.map(point => point.value));
  const pad = (hi - lo || Math.max(Math.abs(hi), 1)) * .12;
  const min = lo - pad, max = hi + pad;
  const x = (index: number) => 48 + index / Math.max(series.length - 1, 1) * 624;
  const y = (value: number) => 164 - (value - min) / (max - min || 1) * 124;
  const segments: string[] = [];
  let current = '';
  series.forEach((point, index) => {
    if (point.value == null || !Number.isFinite(point.value)) { if (current) segments.push(current); current = ''; return; }
    current += `${current ? ' L' : 'M'}${x(index).toFixed(2)} ${y(point.value).toFixed(2)}`;
  });
  if (current) segments.push(current);
  const missing = series.filter(point => point.value == null).length;
  return <figure className="ax-breadth-chart" aria-label={`${META[concept.id].name}趋势图`}>
    <div className="ax-breadth-chart-scroll"><svg viewBox="0 0 720 210" role="img" aria-label={`${META[concept.id].name}趋势图`}>
      <title>{`${META[concept.id].name}，${series.length} 个日期，缺测 ${missing} 个点`}</title>
      {[min, (min + max) / 2, max].map(value => <g key={value}><line className="ax-breadth-grid" x1="48" x2="672" y1={y(value)} y2={y(value)} /><text x="40" y={y(value) + 4} textAnchor="end">{number(value)}</text></g>)}
      {segments.map((path, index) => <path key={path} data-breadth-path-segment={index} className="ax-breadth-line" d={path} />)}
      {finite.map(point => { const index = series.indexOf(point); if (index !== 0 && index !== series.length - 1 && index % 10 !== 0) return null; return <circle key={point.date} data-breadth-point={point.date} className="ax-breadth-point" cx={x(index)} cy={y(point.value)} r="4"><title>{`${point.date} · ${number(point.value)}${point.coverage == null ? '' : ` · 覆盖率 ${pct(point.coverage)}`}`}</title></circle>; })}
      {series.map((point, index) => point.value != null ? null : <g key={point.date} data-breadth-missing={point.date}><line className="ax-breadth-missing" x1={x(index) - 5} x2={x(index) + 5} y1="96" y2="106" /><line className="ax-breadth-missing" x1={x(index) - 5} x2={x(index) + 5} y1="106" y2="96" /></g>)}
      <text className="ax-breadth-date" x="48" y="194">{series[0]?.date}</text><text className="ax-breadth-date" x="672" y="194" textAnchor="end">{series[series.length - 1]?.date}</text>
    </svg></div>
    <figcaption>{series.length} 个交易日 · {missing ? `缺测 ${missing} 个点（×），折线不跨越缺口` : '序列无缺测点'}<span className="ax-breadth-mobile-hint"> · 窄屏可左右滑动图表</span></figcaption>
  </figure>;
}

type TickInputs = { sample_at?: string; market?: string; definition?: string; up_ticks?: number; down_ticks?: number; unchanged_ticks?: number; members?: Array<{ symbol: string; previous_price: number; latest_price: number; direction: 'up' | 'down' | 'unchanged'; source_url: string }> };
export interface MarketTickSnapshot extends Required<Omit<TickInputs, 'members'>> { universe: string[]; net_tick: number; members: NonNullable<TickInputs['members']>; }
function TickMemberVisual({ concept }: { concept: BreadthConcept }) {
  const tick = concept.inputs as TickInputs;
  if (!tick.members?.length) return null;
  return <figure className="ax-breadth-tick" aria-label="三成员逐笔价格方向">
    <div className="ax-breadth-tick-score"><span>同一 UTC 截止时点</span><strong>{concept.latest == null ? 'N/A' : number(concept.latest)}</strong><small>{tick.sample_at || '时点未返回'}</small></div>
    <div className="ax-breadth-tick-members">{tick.members.map(member => <a key={member.symbol} href={member.source_url} target="_blank" rel="noreferrer" data-tick-direction={member.direction}><b>{member.symbol}</b><span>{number(member.previous_price)} → {number(member.latest_price)}</span><i>{member.direction === 'up' ? '↑ 上跳' : member.direction === 'down' ? '↓ 下跳' : '— 持平'}</i></a>)}</div>
    <figcaption>{tick.up_ticks ?? 0} 上跳 − {tick.down_ticks ?? 0} 下跳 = {concept.latest ?? 'N/A'}；{tick.unchanged_ticks ?? 0} 个持平。固定三币篮子仅作真实逐笔定义演示，不是 NYSE TICK。</figcaption>
  </figure>;
}

export function MarketTickSnapshotView({ snapshot }: { snapshot: MarketTickSnapshot }) {
  const concept: BreadthConcept = { id: 'tick', status: 'available', latest: snapshot.net_tick, triggered: null, unit: 'issues', series: [], reason: null, definition: snapshot.definition, inputs: { ...snapshot } };
  return <section className="ax-breadth-shell" data-breadth-selected="tick" aria-label="TICK 指数真实市场宽度实践">
    <header className="ax-breadth-header"><div><span className="ax-breadth-kicker">市场宽度 · 真实逐笔快照</span><h3>{META.tick.name}</h3><p>{META.tick.explanation}</p></div><div className="ax-breadth-value available"><small>截至 {utc(snapshot.sample_at)}</small><strong>{number(snapshot.net_tick)}</strong><span>家数</span></div></header>
    <TickMemberVisual concept={concept} />
    <details className="ax-breadth-provenance"><summary>来源与计算边界</summary><div><p><b>市场：</b>{snapshot.market}</p><p><b>定义：</b>{snapshot.definition}</p><ul>{snapshot.members.map(member => <li key={member.symbol}><a href={member.source_url} target="_blank" rel="noreferrer">{member.symbol} · Binance 聚合成交 ↗</a></li>)}</ul></div></details>
  </section>;
}

function BullishPercentVisual({ concept }: { concept: BreadthConcept }) {
  const buy = Number(concept.inputs.buy_signals ?? 0), eligible = Number(concept.inputs.eligible_signals ?? 0);
  if (!eligible || concept.latest == null) return null;
  const percent = Math.max(0, Math.min(100, concept.latest));
  return <figure className="ax-breadth-bpi" aria-label="点数图多头状态占比"><div className="ax-breadth-bpi-bar"><span style={{ width: `${percent}%` }} /><i style={{ left: `${percent}%` }} /></div><div className="ax-breadth-bpi-counts"><b>{buy} 个明确买入状态</b><span>{eligible - buy} 个明确非买入状态</span></div><figcaption>close-only 对数 1% box、3-box reversal；{buy}/{eligible} = {number(percent)}%。未形成明确状态的成员不进入分母。</figcaption></figure>;
}

export function MarketBreadthSnapshotView({ conceptId, snapshot }: { conceptId: string; snapshot: MarketBreadthSnapshot }) {
  const selectedId: BreadthPracticeId = conceptId === 'book_mcclellan_sum' ? conceptId : ids.includes(conceptId as BreadthConceptId) ? conceptId as BreadthConceptId : 'ad_line';
  const oscillator = snapshot.concepts.find(item => item.id === 'mcclellan');
  let sum = 0;
  const summed = oscillator?.series.map(point => ({ ...point, value: (sum += point.value) })) ?? [];
  const concept: BreadthConcept | undefined = selectedId === 'book_mcclellan_sum' ? {
    id: selectedId,
    status: summed.length ? 'available' : 'unavailable',
    latest: summed[summed.length - 1]?.value ?? null,
    triggered: null,
    unit: 'issues',
    series: summed,
    reason: summed.length ? null : oscillator?.reason ?? '净上涨家数的 39 日均线尚未完成预热。',
    inputs: { source_concept: 'mcclellan', initial_value: 0 },
    definition: '从本次返回窗口中第一天有定义的真实 McClellan Oscillator 开始累加；初值为 0。',
  } : snapshot.concepts.find(item => item.id === selectedId);
  const unavailable = snapshot.concepts.filter(item => item.status === 'unavailable').length;
  const coverages = snapshot.observations.map(item => observationCoverage(item, snapshot.universe.member_count)).filter(Number.isFinite);
  const minimumCoverage = coverages.length ? Math.min(...coverages) : null;
  const latestObservation = snapshot.observations[snapshot.observations.length - 1];
  const tickInputs = concept?.inputs as TickInputs | undefined;
  const displayedAsOf = selectedId === 'tick' && tickInputs?.sample_at ? tickInputs.sample_at.replace('T', ' ').replace('Z', ' UTC') : snapshot.as_of;
  if (!concept) return <section className="ax-breadth-shell ax-breadth-error" role="alert">市场宽度接口没有返回 {META[selectedId].name}。</section>;
  return <section className="ax-breadth-shell" data-breadth-selected={selectedId} aria-label={`${META[selectedId].name}真实市场宽度实践`}>
    <header className="ax-breadth-header">
      <div><span className="ax-breadth-kicker">市场宽度 · 真实快照</span><h3>{META[selectedId].name}</h3><p>{META[selectedId].explanation}</p></div>
      <div className={`ax-breadth-value ${concept.status}`}><small>{concept.status === 'available' ? `截至 ${displayedAsOf}` : '数据边界'}</small><strong>{concept.status === 'available' && concept.latest != null ? number(concept.latest) : 'N/A'}</strong><span>{concept.status === 'available' ? unitLabel(concept.unit) || '指标值' : '当前不可计算'}</span></div>
    </header>
    <div className="ax-breadth-concepts" aria-label="八项市场宽度审计">
      {ids.map(id => { const item = snapshot.concepts.find(value => value.id === id); const active = id === selectedId; return <div key={id} data-breadth-concept={id} aria-current={active || undefined} className={`ax-breadth-concept ${active ? 'active' : ''} ${item?.status || 'missing'}`}><span>{META[id].short}</span><b>{item?.status === 'available' ? item.latest == null ? '有序列' : number(item.latest) : item?.status === 'unavailable' ? '不可算' : '未返回'}</b></div>; })}
    </div>
    {concept.status === 'unavailable' ? <div className="ax-breadth-unavailable" role="note"><span>UNAVAILABLE</span><h4>当前不可计算</h4><p>{unavailableReason(concept)}</p><p>这里保留 N/A，不把缺失证据写成 0。</p></div> : selectedId === 'tick' ? <TickMemberVisual concept={concept} /> : selectedId === 'bullish_percent' ? <BullishPercentVisual concept={concept} /> : <TrendChart concept={concept} observations={snapshot.observations} members={snapshot.universe.member_count} />}
    <div className="ax-breadth-audit">
      <article><span>样本宇宙</span><strong>{universeLabel(snapshot)}</strong><p>{snapshot.universe.member_count} 个固定成员 · 成分基准日 {snapshot.universe.constituents_as_of}</p></article>
      <article><span>最近覆盖</span><strong>{latestObservation ? `${latestObservation.eligible_members} / ${snapshot.universe.member_count}` : '未返回'}</strong><p>{minimumCoverage == null ? '接口没有返回逐日覆盖率' : `最低日覆盖率 ${pct(minimumCoverage)}`} · 门槛 {snapshot.coverage.minimum_required}/{snapshot.coverage.total_members}</p></article>
      <article><span>样本边界</span><strong>{snapshot.observations.length} 个有效交易日</strong><p>行情截至 {snapshot.as_of} · 固定成员回溯，不代表历史官方成分。</p></article>
    </div>
    <details className="ax-breadth-provenance"><summary>来源、缺失规则与计算边界</summary><div>{selectedId === 'tick' && tickInputs?.members?.length ? <><p><b>逐笔来源：</b>{tickInputs.market} · 同一截止时点 {tickInputs.sample_at}</p><ul>{tickInputs.members.map(member => <li key={member.symbol}><a href={member.source_url} target="_blank" rel="noreferrer">{member.symbol} · Binance 聚合成交 ↗</a></li>)}</ul></> : <><p><b>成员表：</b><a href={snapshot.universe.constituents_source} target="_blank" rel="noreferrer">S&amp;P DJI 成分变更公告 ↗</a> · {snapshot.universe.symbols.length} 个代码</p><p><b>行情：</b>{snapshot.source.retrieval}{snapshot.source.retrieved_at ? ` · 抓取于 ${utc(snapshot.source.retrieved_at)}` : ' · 接口未返回统一抓取时间'}{snapshot.source.cache_status ? ` · ${snapshot.source.cache_status}` : ''}。</p><ul>{snapshot.source.members.slice(0, 3).map(member => <li key={member.symbol}><a href={member.endpoint} target="_blank" rel="noreferrer">{member.symbol} · {member.provider} ↗</a> · {member.price_basis}{member.corporate_actions ? ` · 公司行动 ${member.corporate_actions}` : ''}</li>)}</ul>{snapshot.source.members.length > 3 && <p>其余 {snapshot.source.members.length - 3} 个成员也各自保留 provider、endpoint、价格口径与公司行动边界。</p>}</>}<p><b>固定篮子边界：</b>{snapshot.universe.scope_note}</p><p><b>缺失规则：</b>{snapshot.universe.missing_policy}</p><p><b>交易日：</b>{snapshot.universe.calendar}</p>{concept.definition && <p><b>定义：</b>{concept.definition}</p>}<p><b>所需字段：</b>{Object.keys(concept.inputs).join('、') || '无额外输入字段'}</p></div></details>
  </section>;
}

export function MarketBreadthVisual({ conceptId }: { conceptId: string }) {
  const [snapshot, setSnapshot] = useState<MarketBreadthSnapshot | null>(null);
  const [tick, setTick] = useState<MarketTickSnapshot | null>(null);
  const [error, setError] = useState('');
  const [retry, setRetry] = useState(0);
  const isTick = conceptId === 'tick';
  const endpoint = useMemo(() => appPath(isTick ? '/api/practice' : '/api/market-breadth/snapshot'), [isTick]);
  useEffect(() => {
    const controller = new AbortController();
    setSnapshot(null); setTick(null); setError('');
    const request = isTick ? {
      signal: controller.signal,
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ concept_id: 'tick', module: 'data', source: 'market_breadth', inputs: {} }),
    } : { signal: controller.signal };
    fetch(endpoint, request).then(async response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${(await response.text()).slice(0, 180)}`);
      return response.json() as Promise<MarketBreadthSnapshot | { market_tick: MarketTickSnapshot }>;
    }).then(result => {
      if (isTick) {
        if (!('market_tick' in result) || !result.market_tick) throw new Error('真实逐笔快照没有返回 TICK 数据');
        setTick(result.market_tick);
      } else setSnapshot(result as MarketBreadthSnapshot);
    }).catch(reason => { if ((reason as Error).name !== 'AbortError') setError(String(reason)); });
    return () => controller.abort();
  }, [endpoint, isTick, retry]);
  if (error) return <section className="ax-breadth-shell ax-breadth-error" role="alert"><b>真实市场宽度暂不可用</b><p>{error}</p><button type="button" onClick={() => setRetry(value => value + 1)}>重新加载</button></section>;
  if (tick) return <MarketTickSnapshotView snapshot={tick} />;
  if (!snapshot) return <section className="ax-breadth-shell ax-breadth-loading" role="status"><span />{isTick ? '正在读取交易所逐笔成交…' : '正在加载固定成分样本与逐日覆盖率…'}</section>;
  return <MarketBreadthSnapshotView conceptId={conceptId} snapshot={snapshot} />;
}
