import { useEffect, useMemo, useRef, useState, type FormEvent } from 'react';
import { alphaApi } from './alphaApi';
import type { AlphaCatalog, AlphaCollection, AlphaCollectionId, AlphaFactor, AlphaPoint, AlphaResearchResult, AlphaResearchSource } from './types';
import './AlphaResearchPage.css';

const COLLECTION_CONFIG: Record<AlphaCollectionId, { source: AlphaResearchSource; symbol: string; sourceLabel: string }> = {
  worldquant101: { source: 'us_stock', symbol: 'AAPL', sourceLabel: '美股日线' },
  qlib_alpha158: { source: 'a_share', symbol: '600519', sourceLabel: 'A 股日线' },
};
const PROVIDERS: Record<string, string> = { yahoo: 'Yahoo Finance', eastmoney: '东方财富', tencent: '腾讯行情' };
const isWebUrl = (value?: string | null) => Boolean(value && /^https?:\/\//i.test(value));

function factorFamily(id: string) {
  const normalized = id.toUpperCase();
  if (/^K(?:MID|LEN|UP|LOW|SFT)/.test(normalized)) return 'candle';
  if (/^(OPEN|HIGH|LOW|VWAP)0$/.test(normalized)) return 'price';
  return normalized.replace(/\d+$/, '');
}

function factorLesson(factor: AlphaFactor) {
  const family = factorFamily(factor.id);
  const window = factor.id.match(/(\d+)$/)?.[1];
  const suffix = window ? `近 ${window} 个交易日` : '当前交易日';
  if (factor.id.startsWith('alpha')) {
    if (factor.required_fields.includes('cross_section_universe')) return ['同日横向比较', '想象把同一天的股票排成一列：公式比较的是它们彼此的位置，单只股票没有这个名次。', '股票名单、行业归属和可用行情都要按当时的日期确定。'];
    if (factor.required_fields.includes('vwap')) return ['成交价的重心', '成交量大的交易对平均成交价影响更大；它和高低收三价的平均值不是一回事。', '没有真实成交额或逐笔成交，就不能算出原式所需的 VWAP。'];
    if (factor.required_fields.some(field => field.startsWith('adv'))) return ['先量一量成交金额', '把每天成交的金额先做滚动平均，再看今天的量价关系是否异常。', '股数和成交金额单位不同，不能互换。'];
  }
  const lessons: Record<string, [string, string, string]> = {
    candle: ['拆解一根 K 线', '把开、高、低、收的位置关系压缩成可比的数字。', '留意分母为 0 的交易日，以及复权口径是否一致。'],
    price: ['价格位置', '用当日某个价格与收盘价比较，便于跨标的对照。', '它描述价格位置，不是未来收益。'],
    ROC: ['看变化速度', `Qlib 这里用${suffix}前的收盘价除以当前收盘价；小于 1 表示当前价更高。`, '窗口开始前没有足够历史，结果应为空值。'],
    MA: ['看平均水平', `${suffix}的平均值用来平滑短期噪声。`, '移动平均是滞后描述，不是预言。'],
    STD: ['看波动大小', `用${suffix}的离散程度表示价格波动。`, '样本方差和总体方差的口径不能混用。'],
    RANK: ['看窗口内位置', `计算当前值在${suffix}历史中的分位位置。`, 'Qlib Alpha158 这里是单标的滚动时间序列排名，不是同日横截面排名。'],
    CORR: ['看两序列同步程度', `在${suffix}内配对两组观测，衡量线性共变。`, '序列没有变化或成对样本不足时，相关值不可定义。'],
    BETA: ['看线性趋势', `对${suffix}的序列做线性回归，斜率描述方向和速度。`, '回归窗口内的索引定义和缺失值处理会影响结果。'],
  };
  return lessons[family] || ['读懂这个因子', factor.summary, `先核对${suffix}、公式中的每个输入和缺失值口径，再解读数值。`];
}

function FactorDiagram({ factor }: { factor: AlphaFactor }) {
  const family = factorFamily(factor.id);
  if (factor.required_fields.includes('cross_section_universe')) return <svg className="ax-alpha-lesson-svg" viewBox="0 0 320 150" role="img" aria-label={`${factor.name} 同日股票比较示意图`}>
    <line x1="24" y1="116" x2="300" y2="116" /><rect x="48" y="73" width="35" height="43" /><rect className="signal" x="104" y="42" width="35" height="74" /><rect x="160" y="87" width="35" height="29" /><rect x="216" y="58" width="35" height="58" /><text x="42" y="137">甲</text><text x="104" y="137">乙</text><text x="160" y="137">丙</text><text x="216" y="137">丁</text><text x="32" y="24">同一天 · 多只股票</text>
  </svg>;
  if (factor.required_fields.includes('vwap')) return <svg className="ax-alpha-lesson-svg" viewBox="0 0 320 150" role="img" aria-label={`${factor.name} 成交加权价示意图`}>
    <line x1="20" y1="118" x2="302" y2="118" /><path className="signal-path" d="M24 94 L72 82 L120 100 L168 49 L216 66 L264 34 L300 48" /><path className="window" d="M24 101 L72 95 L120 87 L168 76 L216 66 L264 57 L300 52" /><text x="24" y="22">成交加权价需要逐笔或可靠成交额</text>
  </svg>;
  if (family === 'candle' || family === 'price') return <svg className="ax-alpha-lesson-svg" viewBox="0 0 320 150" role="img" aria-label={`${factor.name} 因子示意图`}>
    <line x1="90" y1="18" x2="90" y2="132" /><rect x="68" y="52" width="44" height="50" /><text x="122" y="28">high</text><text x="122" y="130">low</text><text x="18" y="60">open</text><text x="18" y="102">close</text>
    <line x1="170" y1="118" x2="294" y2="118" /><rect className="signal" x="184" y="72" width="24" height="46" /><rect className="signal" x="222" y="50" width="24" height="68" /><rect className="signal" x="260" y="30" width="24" height="88" />
  </svg>;
  return <svg className="ax-alpha-lesson-svg" viewBox="0 0 320 150" role="img" aria-label={`${factor.name} 因子示意图`}>
    <line x1="20" y1="118" x2="302" y2="118" /><line x1="20" y1="24" x2="20" y2="118" />
    <path className="signal-path" d="M 24 100 L 64 86 L 104 94 L 144 58 L 184 70 L 224 40 L 264 52 L 300 27" />
    <line className="window" x1="144" y1="18" x2="144" y2="126" /><text x="154" y="142">滚动窗口 → 当前日</text>
  </svg>;
}

function LineFigure({ points, label, empty }: { points: AlphaPoint[]; label: string; empty: string }) {
  const values = points.flatMap((point, index) => point.value == null || !Number.isFinite(point.value) ? [] : [{ index, value: point.value }]);
  if (!values.length) return <div className="ax-alpha-chart-empty">{empty}</div>;
  const min = Math.min(...values.map(point => point.value)); const max = Math.max(...values.map(point => point.value)); const span = max - min || 1;
  const x = (index: number) => 60 + index * 564 / Math.max(1, points.length - 1); const y = (value: number) => 20 + (max - value) * 172 / span;
  const segments = values.reduce<Array<typeof values>>((groups, point) => { const current = groups[groups.length - 1]; if (!current || point.index !== current[current.length - 1].index + 1) groups.push([point]); else current.push(point); return groups; }, []);
  return <svg viewBox="0 0 640 230" role="img" aria-label={label}>{min <= 0 && max >= 0 && <line className="ax-alpha-zero" x1="60" x2="624" y1={y(0)} y2={y(0)} />}<text x="4" y="24">{max.toPrecision(4)}</text><text x="4" y="194">{min.toPrecision(4)}</text><text x="60" y="220">{points[0]?.timestamp.slice(0, 10)}</text><text x="624" y="220" textAnchor="end">{points[points.length - 1]?.timestamp.slice(0, 10)}</text>{segments.map((segment, index) => <path key={index} className="ax-alpha-line" d={segment.map((point, i) => `${i ? 'L' : 'M'} ${x(point.index).toFixed(1)} ${y(point.value).toFixed(1)}`).join(' ')} />)}</svg>;
}

function ScatterFigure({ factor, returns }: { factor: AlphaPoint[]; returns: AlphaPoint[] }) {
  const returnByTime = new Map(returns.map(point => [point.timestamp, point.value]));
  const pairs = factor.flatMap(point => point.value == null || returnByTime.get(point.timestamp) == null ? [] : [{ x: point.value, y: returnByTime.get(point.timestamp)!, timestamp: point.timestamp }]);
  if (!pairs.length) return <div className="ax-alpha-chart-empty">没有可配对的因子与收益观测</div>;
  const xMin = Math.min(...pairs.map(point => point.x)); const xMax = Math.max(...pairs.map(point => point.x)); const yMin = Math.min(...pairs.map(point => point.y)); const yMax = Math.max(...pairs.map(point => point.y));
  const x = (value: number) => 60 + (value - xMin) * 564 / (xMax - xMin || 1); const y = (value: number) => 20 + (yMax - value) * 172 / (yMax - yMin || 1);
  const step = Math.max(1, Math.ceil(pairs.length / 80));
  return <svg viewBox="0 0 640 230" role="img" aria-label="因子与前瞻收益散点图">{yMin <= 0 && yMax >= 0 && <line className="ax-alpha-zero" x1="60" x2="624" y1={y(0)} y2={y(0)} />}{xMin <= 0 && xMax >= 0 && <line className="ax-alpha-zero" x1={x(0)} x2={x(0)} y1="20" y2="192" />}{pairs.filter((_, index) => index % step === 0 || index === pairs.length - 1).map(point => <circle key={point.timestamp} className="ax-alpha-point" cx={x(point.x)} cy={y(point.y)} r="4" />)}</svg>;
}

function Method({ factor }: { factor: AlphaFactor }) {
  const [title, intuition, pitfall] = factorLesson(factor);
  return <article className="ax-alpha-method" aria-label="当前因子方法">
    <div><span className="ax-alpha-kicker">{factor.status === 'available' ? 'verified exercise' : 'catalog lesson'}</span><h4>{factor.name} · {title}</h4><p>{factor.summary}</p>{factor.formula ? <code className="ax-alpha-formula">{factor.formula}</code> : <p className="ax-alpha-formula ax-alpha-formula-missing">目录未提供可核对的公式，这里不补写。</p>}<p>{intuition}</p></div>
    <FactorDiagram factor={factor} />
    <div className="ax-alpha-method-meta"><h4>数据与边界</h4>{factor.required_fields.length ? <ul className="ax-alpha-fields">{factor.required_fields.map(field => <li key={field}>{field}</li>)}</ul> : <p>目录还没有给出已核对的输入字段。</p>}<p>{pitfall}</p>{factor.status === 'unavailable' && <p className="ax-alpha-unavailable">当前只能学习，尚不可运行。原因：{factor.summary}</p>}<p className="ax-alpha-links">{isWebUrl(factor.source_url) && <a href={factor.source_url} target="_blank" rel="noopener noreferrer">阅读实际来源 ↗</a>}{isWebUrl(factor.code_url) && <a href={factor.code_url!} target="_blank" rel="noopener noreferrer">查看核验实现 ↗</a>}</p></div>
  </article>;
}

function LearningCenter({ collection, selectedId, onSelect }: { collection: AlphaCollection; selectedId: string; onSelect: (factor: AlphaFactor) => void }) {
  const [query, setQuery] = useState(''); const [status, setStatus] = useState<'all' | AlphaFactor['status']>('all');
  useEffect(() => { setQuery(''); setStatus('all'); }, [collection.id]);
  const factors = useMemo(() => { const term = query.trim().toLocaleLowerCase(); return collection.factors.filter(factor => (status === 'all' || factor.status === status) && (!term || [factor.id, factor.name, factor.summary, factor.formula || ''].some(value => value.toLocaleLowerCase().includes(term)))); }, [collection.factors, query, status]);
  const selected = collection.factors.find(factor => factor.id === selectedId) || collection.factors[0];
  return <section className="ax-alpha-learning" aria-label="因子学习中心"><header><div><span className="ax-alpha-kicker">factor learning center</span><h3>{collection.name} 逐项学</h3><p>点开任意条目，查看它自己的公式、直觉、数据前提和可运行状态。</p></div><strong>{factors.length} / {collection.total}</strong></header>
    <div className="ax-alpha-learning-layout"><div className="ax-alpha-catalog"><div className="ax-alpha-catalog-tools"><label>搜索<input aria-label="搜索因子" value={query} onChange={event => setQuery(event.target.value)} placeholder="例如 ROC5、rank、成交量" /></label><label>状态<select aria-label="因子状态" value={status} onChange={event => setStatus(event.target.value as typeof status)}><option value="all">全部</option><option value="available">可运行</option><option value="unavailable">待核验</option></select></label></div>
      <div className="ax-alpha-factor-list" aria-label="因子目录" tabIndex={0}>{factors.map(factor => <button type="button" data-factor-card key={factor.id} className={factor.id === selected?.id ? 'selected' : ''} onClick={() => onSelect(factor)}><span>{factor.name}</span><small>{factor.status === 'available' ? '可运行' : '待核验'}</small><p>{factor.summary}</p></button>)}{!factors.length && <p className="ax-alpha-no-match">没有符合条件的因子。</p>}</div></div>
      {selected && <Method factor={selected} />}
    </div>
  </section>;
}

function Results({ result }: { result: AlphaResearchResult }) {
  const provider = PROVIDERS[result.market_provenance.provider] || result.market_provenance.provider;
  return <section className="ax-alpha-results" aria-live="polite" aria-label="因子研究结果"><div className="ax-alpha-metrics"><article><span>PAIRED OBSERVATIONS</span><strong>{result.stats.paired_count}</strong></article><article><span>时间序列相关</span><strong>{result.stats.time_series_correlation == null ? '样本不足' : result.stats.time_series_correlation.toFixed(4)}</strong></article><article><span>MARKET SOURCE</span><strong>{provider}</strong></article></div>{result.reason && <p className="ax-alpha-status">{result.reason}</p>}<div className="ax-alpha-charts"><figure className="ax-alpha-chart-card"><h4>因子序列</h4><p>只展示服务端返回的实际计算值。</p><LineFigure points={result.series} label="因子时间序列" empty="没有可绘制的因子序列" /></figure><figure className="ax-alpha-chart-card"><h4>前瞻收益标签</h4><p>信号形成后才进入。</p><LineFigure points={result.forward_returns} label="前瞻收益时间序列" empty="没有可配对的前瞻收益" /></figure><figure className="ax-alpha-chart-card ax-alpha-scatter"><h4>因子 × 前瞻收益</h4><ScatterFigure factor={result.series} returns={result.forward_returns} /></figure></div><details className="ax-alpha-provenance"><summary>核对行情来源与价格口径</summary><p><strong>{provider}</strong> · {result.market_provenance.price_basis}</p><p>{result.market_provenance.corporate_actions}</p>{isWebUrl(result.market_provenance.endpoint) && <a href={result.market_provenance.endpoint} target="_blank" rel="noopener noreferrer">查看行情接口 ↗</a>}</details><aside className="ax-alpha-caveats"><h4>诊断边界</h4><ul><li>Pearson 值是单标的时间序列诊断，不是横截面 IC。</li><li>没有扣交易费、滑点和容量影响。</li>{result.notes.map(note => <li key={note}>{note}</li>)}</ul></aside></section>;
}

export function AlphaResearchPage() {
  const [catalog, setCatalog] = useState<AlphaCatalog>(); const [catalogError, setCatalogError] = useState(''); const [retry, setRetry] = useState(0);
  const initialCollection = new URLSearchParams(window.location.search).get('collection');
  const [collectionId, setCollectionId] = useState<AlphaCollectionId>(initialCollection === 'qlib_alpha158' ? 'qlib_alpha158' : 'worldquant101');
  const [factorId, setFactorId] = useState(''); const [lessonFactorId, setLessonFactorId] = useState(new URLSearchParams(window.location.search).get('factor') || '');
  const [source, setSource] = useState<AlphaResearchSource>(COLLECTION_CONFIG[collectionId].source); const [symbol, setSymbol] = useState(COLLECTION_CONFIG[collectionId].symbol); const [limit, setLimit] = useState(160);
  const [result, setResult] = useState<AlphaResearchResult>(); const [running, setRunning] = useState(false); const [runError, setRunError] = useState(''); const requestId = useRef(0);
  useEffect(() => { let current = true; setCatalog(undefined); setCatalogError(''); alphaApi.catalog().then(data => { if (current) setCatalog(data); }).catch(error => { if (current) setCatalogError((error as Error).message); }); return () => { current = false; }; }, [retry]);
  const collection = useMemo(() => catalog?.collections.find(item => item.id === collectionId), [catalog, collectionId]);
  const available = useMemo(() => collection?.factors.filter(factor => factor.status === 'available') || [], [collection]);
  useEffect(() => { if (!collection) return; const lesson = collection.factors.some(factor => factor.id === lessonFactorId) ? lessonFactorId : collection.factors[0]?.id || ''; setLessonFactorId(lesson); if (!available.some(factor => factor.id === factorId)) setFactorId(available[0]?.id || ''); }, [available, collection, factorId, lessonFactorId]);
  const changeCollection = (next: AlphaCollectionId) => { const config = COLLECTION_CONFIG[next]; requestId.current += 1; setCollectionId(next); setSource(config.source); setSymbol(config.symbol); setFactorId(''); setLessonFactorId(''); setResult(undefined); setRunError(''); setRunning(false); if (/^https?:$/.test(window.location.protocol)) { const url = new URL(window.location.href); url.searchParams.set('collection', next); url.searchParams.delete('factor'); window.history.replaceState(null, '', url); } };
  const selectLesson = (factor: AlphaFactor) => { requestId.current += 1; setRunning(false); setResult(undefined); setRunError(''); setLessonFactorId(factor.id); if (factor.status === 'available') setFactorId(factor.id); if (/^https?:$/.test(window.location.protocol)) { const url = new URL(window.location.href); url.searchParams.set('collection', collectionId); url.searchParams.set('factor', factor.id); window.history.replaceState(null, '', url); } };
  const submit = async (event: FormEvent) => { event.preventDefault(); if (!factorId) return; const current = ++requestId.current; setRunning(true); setRunError(''); setResult(undefined); try { const next = await alphaApi.research({ collection: collectionId, id: factorId, source, symbol: symbol.trim(), limit }); if (current === requestId.current) setResult(next); } catch (error) { if (current === requestId.current) setRunError((error as Error).message); } finally { if (current === requestId.current) setRunning(false); } };
  if (catalogError) return <div className="ax-alpha ax-alpha-error"><p>因子目录加载失败：{catalogError}</p><button className="ax-btn" data-alpha-retry onClick={() => setRetry(value => value + 1)}>重新加载</button></div>;
  if (!catalog || !collection) return <div className="ax-alpha ax-loading">正在加载因子目录…</div>;
  return <div className="ax-alpha"><div className="ax-alpha-hero"><article className="ax-alpha-intro"><span className="ax-alpha-kicker">alpha research studio</span><h3>把公式变成可证伪的研究</h3><p>先在逐项学习中心读懂公式与边界，再对已核验条目运行真实行情诊断。</p></article><aside className="ax-alpha-truth"><strong>目录 ≠ 可运行</strong><p>所有条目都能学；只有完成输入、窗口与实现核验的条目才能运行。</p></aside></div>
    <div className="ax-alpha-collection-switch"><label>因子库<select aria-label="选择因子库" value={collectionId} onChange={event => changeCollection(event.target.value as AlphaCollectionId)}>{catalog.collections.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label></div>
    <LearningCenter key={collection.id} collection={collection} selectedId={lessonFactorId} onSelect={selectLesson} />
    <section className="ax-alpha-lab"><h3>真实行情研究台</h3><p>这里只列出服务端已核验的可运行因子。</p><form className="ax-alpha-form" onSubmit={submit}><label>可运行因子<select aria-label="选择因子" value={factorId} onChange={event => { const factor = available.find(item => item.id === event.target.value); if (factor) selectLesson(factor); setResult(undefined); }}>{available.map(factor => <option key={factor.id} value={factor.id}>{factor.name}</option>)}</select></label><label>行情来源<select aria-label="行情来源" value={source} disabled><option value={source}>{COLLECTION_CONFIG[collectionId].sourceLabel}</option></select></label><label>研究标的<input aria-label="研究标的" value={symbol} onChange={event => setSymbol(event.target.value.toUpperCase())} /></label><label>日线根数<input aria-label="日线根数" type="number" min="30" max="500" value={limit} onChange={event => setLimit(Number(event.target.value))} /></label><button className="ax-btn" disabled={running || !factorId || !symbol.trim()}>{running ? '研究中…' : '运行实证'}</button></form>{!available.length && <p className="ax-alpha-status">该因子库暂无完成实现核验的条目。</p>}{runError && <p className="ax-alpha-error" role="alert">研究请求失败：{runError}</p>}{result && <Results result={result} />}</section>
  </div>;
}
