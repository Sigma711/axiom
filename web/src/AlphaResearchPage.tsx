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
    RSQR: ['趋势有多整齐', `把${suffix}收盘价连成线，检查直线能解释其中多少起伏。接近 1 说明更像直线，不代表会继续上涨。`, '横盘时可能没有可解释的变化，不能把空值当成零。'],
    RESI: ['看偏离趋势多少', `用今天收盘价减去${suffix}拟合直线的末端值，再除以今天收盘价。`, '偏离趋势并不自动意味着价格会回归。'],
    MAX: ['离近期高点多远', `${suffix}最高价除以今天收盘价；例如 110÷100=1.10，表示最高价比今天收盘高 10%。`, '使用的是窗口内最高价，不是今天最高价。'],
    MIN: ['离近期低点多远', `${suffix}最低价除以今天收盘价；例如 90÷100=0.90。`, '使用的是窗口内最低价，不是今天最低价。'],
    QTLU: ['与偏高的历史价格比', `${suffix}收盘价的 80% 分位数除以今天收盘价。`, '分位数不是窗口最高价；样本太短时要看清计算窗口。'],
    QTLD: ['与偏低的历史价格比', `${suffix}收盘价的 20% 分位数除以今天收盘价。`, '分位数不是窗口最低价。'],
    RSV: ['收盘落在区间哪里', `用今天收盘价减去${suffix}最低价，再除以同期最高价与最低价之差。`, '0 接近区间底部，1 接近顶部；这不是涨跌概率。'],
    IMAX: ['高点出现得早还是晚', `找出${suffix}最高价在窗口中的位置，再除以窗口长度。`, '这里的位置按 Qlib 的窗口索引计算，不是距今的日历天数。'],
    IMIN: ['低点出现得早还是晚', `找出${suffix}最低价在窗口中的位置，再除以窗口长度。`, '并列极值如何取位置会影响结果。'],
    IMXD: ['比较高低点先后', `用${suffix}最高价位置减去最低价位置，再除以窗口长度。`, '正负号说明先后顺序，不说明未来方向。'],
    CORD: ['量价变化是否同步', `比较${suffix}价格比率与成交量比率的相关性。`, '两边都先对齐日期；价格和成交量本身的相关性属于 CORR。'],
    CNTP: ['上涨日占多少', `数出${suffix}收盘价高于前一天的日子，再除以窗口中的有效日数。`, '上涨频率不等于累计涨幅。'],
    CNTN: ['下跌日占多少', `数出${suffix}收盘价低于前一天的日子，再除以窗口中的有效日数。`, '平盘日既不计上涨也不计下跌。'],
    CNTD: ['上涨日比下跌日多多少', `用${suffix}上涨日比例减去下跌日比例。`, '只数日子，不考虑每一天涨跌的幅度。'],
    SUMP: ['上涨贡献了多少波动', `把${suffix}所有正的收盘变动加起来，除以全部变动绝对值之和。`, '价格连续不变时分母靠公式中的微小常数保护。'],
    SUMN: ['下跌贡献了多少波动', `把${suffix}所有负变动的绝对值加起来，除以全部变动绝对值之和。`, '不是收益率，不能直接当亏损百分比。'],
    SUMD: ['上涨幅度减下跌幅度', `用${suffix}正变动总和减去负变动绝对值总和，再除以总波动。`, '与只数涨跌天数的 CNTD 不同。'],
    VMA: ['今天的量比平时大吗', `${suffix}平均成交量除以今天成交量；例如平均 200 万股、今天 100 万股，结果约为 2。`, '该公式是“平均÷今天”，别把方向读反；停牌零成交要单独看。'],
    VSTD: ['成交量是否稳定', `${suffix}成交量标准差除以今天成交量。`, '分母接近零会放大数值，不代表交易机会。'],
    WVMA: ['放量波动是否稳定', `先计算每天“绝对收益变化×成交量”，再比较${suffix}的标准差与均值。`, '这是波动与量的组合，不是成交量加权均价 VWAP。'],
    VSUMP: ['增量日贡献多少', `把${suffix}所有正的成交量变化相加，再除以成交量变化的绝对值总和。`, '量增不等于价格上涨。'],
    VSUMN: ['减量日贡献多少', `把${suffix}所有负的成交量变化取绝对值后相加，再除以总变化。`, '分母为零时按原公式处理，不擅自填成有意义的信号。'],
    VSUMD: ['量增与量减的净差', `用${suffix}量增总和减去量减总和，再除以总变化。`, '只描述成交活跃度的方向，不直接预测股价。'],
  };
  return lessons[family] || ['读懂这个因子', factor.summary, `先核对${suffix}、公式中的每个输入和缺失值口径，再解读数值。`];
}

const FORMULA_WORDS: [RegExp, string][] = [
  [/\b(?:Ref|delay)\s*\(/i, 'Ref / delay：取之前某一天的值'],
  [/\b(?:Mean|Sum|Std|Max|Min|Quantile)\s*\(/i, '窗口函数：只使用截至当天已知道的历史'],
  [/\b(?:Rank|ts_rank)\s*\(/i, 'Rank：先确认是单只股票的历史排名，还是同日多股票排名'],
  [/\b(?:Corr|Cov|correlation|covariance)\s*\(/i, '相关 / 协方差：先把两条序列按日期对齐'],
  [/\b(?:Slope|Rsquare|Resi)\s*\(/i, '回归：用窗口内观测拟合趋势，不等于预测'],
  [/\b(?:Greater|Less|Abs|Sign)\s*\(/i, '比较 / 绝对值 / 符号：改变数值的取法'],
  [/\b(?:adv\d*|volume|\$volume)\b/i, '成交量：股数、金额和滚动均量的单位不能混用'],
  [/\b(?:vwap|\$vwap)\b/i, 'VWAP：实际成交额除以实际成交量'],
];
const WQ_FORMULA_WORDS: [RegExp, string][] = [
  [/\brank\s*\(/i, 'rank：同一天把股票排队；必须知道当天完整标的池'],
  [/\b(?:ts_rank|Ts_ArgMax|ts_argmax|ts_argmin)\s*\(/i, 'ts_ 开头：只在同一只股票的过去若干天里比较'],
  [/\b(?:delay|delta)\s*\(/i, 'delay 取过去的值；delta 算与过去相比变了多少'],
  [/\b(?:correlation|covariance)\s*\(/i, '相关 / 协方差：比较两条按日期配对的历史序列'],
  [/\b(?:adv\d*|volume)\b/i, 'volume 是成交股数；adv 是滚动平均成交金额，单位不同'],
  [/\bvwap\b/i, 'VWAP 是实际成交额÷实际成交量，不是开高低收平均价'],
  [/\bIndNeutralize\s*\(/i, '行业中性化：扣除同日同业股票的共同部分，行业分类须按当时确定'],
  [/\bscale\s*\(/i, 'scale：按当天股票池的绝对值总和缩放'],
  [/\?[^:]+:/, '问号与冒号：先判断条件，再只选其中一条计算路径'],
  [/\^|\bSignedPower\s*\(/i, '幂运算：先确认底数的正负及指数口径'],
];

function formulaReading(factor: AlphaFactor) {
  const formula = factor.formula || '';
  const words = (/^alpha/i.test(factor.id) ? WQ_FORMULA_WORDS : FORMULA_WORDS)
    .filter(([pattern]) => pattern.test(formula)).map(([, explanation]) => explanation);
  return words.slice(0, 6);
}

function teachingExample(factor: AlphaFactor) {
  const id = factor.id.toLowerCase();
  if (id === 'kmid') return '假设开盘 100 元、收盘 105 元：KMID = (105−100)÷100 = 0.05。它说的是这根 K 线涨了 5%，不是明天会涨 5%。';
  if (id === 'klen') return '假设开盘 100 元、最高 108 元、最低 98 元：KLEN = (108−98)÷100 = 0.10。这根 K 线的振幅相当于开盘价的 10%。';
  if (id.startsWith('roc')) return '假设窗口起点收盘 100 元、今天收盘 125 元：Qlib 的 ROC = 100÷125 = 0.8。这里小于 1 表示今天更高，方向容易读反。';
  if (id.startsWith('vma')) return '假设窗口平均每天成交 200 万股、今天成交 100 万股：VMA = 200÷100 = 2。它用平均量除以今天量。';
  if (id.startsWith('cntp')) return '假设窗口有 5 个有效交易日，其中 3 天收盘高于前一天：CNTP = 3÷5 = 0.6；它不管每次涨了多少。';
  if (id === 'alpha101') return '读数时先问：公式所用的是截至当天已收盘的信息吗？再把因子值与未来收益按日期配对。单只股票的相关性只是初步诊断。';
  if (/^alpha/.test(id) && factor.required_fields.includes('vwap')) return '假设 100 股以 10 元成交、300 股以 12 元成交：VWAP = (100×10+300×12)÷400 = 11.5 元。用这类真实成交信息才能进入原式。';
  if (/^alpha/.test(id) && factor.required_fields.includes('cross_section_universe')) return '想象同一天有甲乙丙丁四只股票：先逐只算原式所需数值，再在这四只之间排名。只输入甲，就没有同日名次。';
  return '先在公式中找到输入价格或成交量、回看窗口和除数；只用当天已知的数据手算一个小样本，再看图上的值是否方向一致。';
}

function fieldMeaning(field: string) {
  const names: Record<string, string> = {
    open: '开盘价', high: '最高价', low: '最低价', close: '收盘价', volume: '成交量',
    returns: '历史收益', vwap: '真实成交量加权价', market_cap: '当时可知的市值',
    cross_section_universe: '当时的多股票标的池', industry: '当时的行业分类', sector: '当时的板块分类',
  };
  return names[field] || (/^adv\d+$/i.test(field) ? `${field.slice(3)} 日平均成交额` : field);
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
  const words = formulaReading(factor);
  return <article className="ax-alpha-method" aria-label="当前因子方法">
    <div><span className="ax-alpha-kicker">{factor.status === 'available' ? '可用真实行情练习' : '先学公式与数据条件'}</span><h4>{factor.name} · {title}</h4><p>{factor.summary}</p><h5>它在问什么？</h5><p>{intuition}</p>{factor.formula ? <><h5>原式怎么读？</h5><code className="ax-alpha-formula">{factor.formula}</code>{words.length > 0 && <ul className="ax-alpha-reading">{words.map(word => <li key={word}>{word}</li>)}</ul>}</> : <p className="ax-alpha-formula ax-alpha-formula-missing">目录未提供可核对的公式，这里不补写。</p>}<div className="ax-alpha-example"><strong>动手想一想</strong><p>{teachingExample(factor)}</p></div></div>
    <figure className="ax-alpha-lesson-figure"><FactorDiagram factor={factor} /><figcaption>读公式的结构示意 · 不是该因子的实测曲线</figcaption></figure>
    <div className="ax-alpha-method-meta"><h4>需要哪些数据？</h4>{factor.required_fields.length ? <ul className="ax-alpha-fields">{factor.required_fields.map(field => <li key={field} title={field}>{fieldMeaning(field)}</li>)}</ul> : <p>目录还没有给出已核对的输入字段。</p>}<h4>哪里容易读错？</h4><p>{pitfall}</p>{factor.status === 'unavailable' && <p className="ax-alpha-unavailable">当前只能学习，尚不可运行。原因：{factor.summary}</p>}<p className="ax-alpha-links">{isWebUrl(factor.source_url) && <a href={factor.source_url} target="_blank" rel="noopener noreferrer">阅读实际来源 ↗</a>}{isWebUrl(factor.code_url) && <a href={factor.code_url!} target="_blank" rel="noopener noreferrer">查看核验实现 ↗</a>}</p></div>
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
  return <section className="ax-alpha-results" aria-live="polite" aria-label="因子研究结果"><div className="ax-alpha-metrics"><article><span>可比较的日期</span><strong>{result.stats.paired_count}</strong></article><article><span>与后续收益的相关性</span><strong>{result.stats.time_series_correlation == null ? '样本不足' : result.stats.time_series_correlation.toFixed(4)}</strong></article><article><span>行情来源</span><strong>{provider}</strong></article></div><p className="ax-alpha-result-guide">相关性接近 1 或 −1 只说明这段样本的线性关系较强；需要更多日期、不同标的和严格的样本外检验。</p>{result.reason && <p className="ax-alpha-status">{result.reason}</p>}<div className="ax-alpha-charts"><figure className="ax-alpha-chart-card"><h4>因子序列</h4><p>只展示服务端返回的实际计算值。</p><LineFigure points={result.series} label="因子时间序列" empty="没有可绘制的因子序列" /></figure><figure className="ax-alpha-chart-card"><h4>后续收益</h4><p>信号形成后的收益，只用于检验，不是当时已知信息。</p><LineFigure points={result.forward_returns} label="前瞻收益时间序列" empty="没有可配对的前瞻收益" /></figure><figure className="ax-alpha-chart-card ax-alpha-scatter"><h4>因子 × 后续收益</h4><ScatterFigure factor={result.series} returns={result.forward_returns} /></figure></div><details className="ax-alpha-provenance"><summary>核对行情来源与价格口径</summary><p><strong>{provider}</strong> · {result.market_provenance.price_basis}</p><p>{result.market_provenance.corporate_actions}</p>{isWebUrl(result.market_provenance.endpoint) && <a href={result.market_provenance.endpoint} target="_blank" rel="noopener noreferrer">查看行情接口 ↗</a>}</details><aside className="ax-alpha-caveats"><h4>诊断边界</h4><ul><li>Pearson 值是单标的时间序列诊断，不是横截面 IC。</li><li>没有扣交易费、滑点和容量影响。</li>{result.notes.map(note => <li key={note}>{note}</li>)}</ul></aside></section>;
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
  return <div className="ax-alpha"><div className="ax-alpha-hero"><article className="ax-alpha-intro"><span className="ax-alpha-kicker">因子研究</span><h3>从一个数字，问一个市场问题</h3><p>先看公式在量什么，再用真实日线核对它与后续价格的关系。一次结果只能形成新问题，不能直接证明策略有效。</p></article><aside className="ax-alpha-truth"><strong>先弄清数据，再动手</strong><p>每个因子都能逐项学习；完成数据口径与公式核验的因子才开放实证。</p></aside></div>
    <div className="ax-alpha-collection-switch"><label>因子库<select aria-label="选择因子库" value={collectionId} onChange={event => changeCollection(event.target.value as AlphaCollectionId)}>{catalog.collections.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label></div>
    <LearningCenter key={collection.id} collection={collection} selectedId={lessonFactorId} onSelect={selectLesson} />
    <section className="ax-alpha-lab"><div className="ax-alpha-lab-heading"><span className="ax-alpha-lab-mark" aria-hidden="true">◆</span><div><span className="ax-alpha-kicker">真实数据 · 可复查计算</span><h3>真实行情研究台</h3><p>选已核验因子和研究标的，先看因子与后续收益有没有关系，再检查来源与样本。</p></div></div><form className="ax-alpha-form" onSubmit={submit}><label>可运行因子<select aria-label="选择因子" value={factorId} onChange={event => { const factor = available.find(item => item.id === event.target.value); if (factor) selectLesson(factor); setResult(undefined); }}>{available.map(factor => <option key={factor.id} value={factor.id}>{factor.name}</option>)}</select></label><div className="ax-alpha-source"><span>行情来源</span><strong>{COLLECTION_CONFIG[collectionId].sourceLabel}</strong></div><label>研究标的<input aria-label="研究标的" value={symbol} onChange={event => setSymbol(event.target.value.toUpperCase())} /></label><label>日线根数<input aria-label="日线根数" type="number" min="30" max="500" value={limit} onChange={event => setLimit(Number(event.target.value))} /></label><button className="ax-btn ax-alpha-run" disabled={running || !factorId || !symbol.trim()}>{running ? '研究中…' : '运行实证 →'}</button></form>{!available.length && <p className="ax-alpha-status">该因子库暂无完成实现核验的条目。</p>}{runError && <p className="ax-alpha-error" role="alert">研究请求失败：{runError}</p>}{result && <Results result={result} />}</section>
  </div>;
}
