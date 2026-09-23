import type { PracticeResult } from './types';

type Trace = { name: string; title: string; color: string };
type Panel = { title: string; bars: Trace[]; line: Trace };

const buy = 'var(--accent-2)';
const sell = 'var(--accent)';
const difference = 'var(--text)';
const compact = (value: number) => Math.abs(value) >= 1e8 ? `${+(value / 1e8).toPrecision(3)}亿` : Math.abs(value) >= 1e4 ? `${+(value / 1e4).toPrecision(4)}万` : `${+value.toPrecision(4)}`;
const exact = (value: number | null | undefined) => value == null ? '—' : Number(value).toLocaleString('zh-CN', { maximumFractionDigits: 6 });

/** One observed 24-hour window, with taker side and asset units kept explicit. */
export function AggressorFlowVisual({ result }: { result: PracticeResult }) {
  const base = result.asset_units?.base_asset || '基础资产';
  const quote = result.asset_units?.quote_asset || '计价资产';
  const cvd = result.concept_id === 'cvd';
  const orderFlow = result.concept_id === 'book_order_flow';
  const panels: Panel[] = cvd ? [
    { title: `窗口内累计主动成交数量差（${base}）`, bars: [{ name: 'base_delta', title: '每小时主动成交数量差', color: buy }], line: { name: 'cvd_base', title: '从窗口起点累计的数量差', color: sell } },
    { title: `窗口内累计主动成交额差（${quote}）`, bars: [{ name: 'quote_delta', title: '每小时主动成交额差', color: buy }], line: { name: 'cvd_quote', title: '从窗口起点累计的成交额差', color: sell } },
  ] : orderFlow ? [
    { title: `主动成交数量（${base} / 1 小时）`, bars: [{ name: 'aggressive_buy_base_volume', title: '主动买入数量', color: buy }, { name: 'aggressive_sell_base_volume', title: '主动卖出数量', color: sell }], line: { name: 'aggressive_base_delta', title: '买卖数量差', color: difference } },
    { title: `主动成交额（${quote} / 1 小时）`, bars: [{ name: 'aggressive_buy_quote_volume', title: '主动买入成交额', color: buy }, { name: 'aggressive_sell_quote_volume', title: '主动卖出成交额', color: sell }], line: { name: 'aggressive_quote_delta', title: '买卖成交额差', color: difference } },
  ] : [
    { title: `Binance 主动成交数量（${base} / 1 小时）`, bars: [{ name: 'taker_buy_base_volume', title: '主动买入数量', color: buy }, { name: 'taker_sell_base_volume', title: '主动卖出数量', color: sell }], line: { name: 'taker_base_imbalance', title: '买卖数量差', color: difference } },
    { title: `Binance 主动成交额（${quote} / 1 小时）`, bars: [{ name: 'taker_buy_quote_volume', title: '主动买入成交额', color: buy }, { name: 'taker_sell_quote_volume', title: '主动卖出成交额', color: sell }], line: { name: 'taker_quote_imbalance', title: '买卖成交额差', color: difference } },
  ];
  const byName = new Map(result.series.map(series => [series.name, series.values]));
  const count = result.bars?.length || 0;
  const start = result.bars?.[0]?.timestamp.slice(0, 16).replace('T', ' ') || '';
  const end = result.bars?.[count - 1]?.timestamp.slice(0, 16).replace('T', ' ') || '';
  const metrics = cvd ? [
    ['最新窗口 CVD', 'latest_cvd_base', base], ['最新窗口成交额 CVD', 'latest_cvd_quote', quote],
    ['窗口买卖数量差', 'window_base_delta', base], ['窗口买卖成交额差', 'window_quote_delta', quote],
  ] : orderFlow ? [
    ['最新 1 小时买卖数量差', 'latest_aggressive_base_delta', base], ['最新 1 小时买卖成交额差', 'latest_aggressive_quote_delta', quote],
    ['24 小时买卖数量差', 'window_aggressive_base_delta', base], ['24 小时买卖成交额差', 'window_aggressive_quote_delta', quote],
  ] : [
    ['最新 1 小时主动买量', 'latest_taker_buy_base_volume', base], ['最新 1 小时主动卖量', 'latest_taker_sell_base_volume', base],
    ['最新 1 小时主动买额', 'latest_taker_buy_quote_volume', quote], ['最新 1 小时主动卖额', 'latest_taker_sell_quote_volume', quote],
  ];
  return <div className="ax-flow-practice">
    <figure className="ax-knowledge-chart ax-wide-series-chart ax-flow-chart">
      <svg viewBox="0 0 760 410" role="img" aria-label="Binance 已收盘 K 线主动买卖实测图，按基础资产和计价资产分图">
        {panels.map((panel, panelIndex) => {
          const traces = [...panel.bars, panel.line];
          const values = traces.flatMap(trace => byName.get(trace.name) || []).filter((value): value is number => value != null && Number.isFinite(value));
          if (!values.length || !count) return null;
          const observedMin = Math.min(0, ...values);
          const observedMax = Math.max(0, ...values);
          const extent = Math.max(Math.abs(observedMin), Math.abs(observedMax), 1);
          const min = observedMin < 0 ? observedMin - extent * 0.08 : 0;
          const max = observedMax + extent * 0.08;
          const y = (value: number) => 161 - (value - min) / (max - min) * 119;
          const x = (index: number) => 96 + index / Math.max(count - 1, 1) * 637;
          const width = Math.min(10, 637 / count / Math.max(panel.bars.length, 1) * 0.7);
          const lineValues = byName.get(panel.line.name) || [];
          const path = lineValues.map((value, index) => value == null || !Number.isFinite(value) ? '' : `${index === 0 || lineValues[index - 1] == null ? 'M' : 'L'}${x(index)},${y(value)}`).join(' ');
          const ticks = min < 0 && max > 0 ? [min, 0, max] : [0, max / 2, max];
          return <g key={panel.title} transform={`translate(0,${panelIndex * 205})`}>
            <text x="96" y="22" className="ax-flow-title">{panel.title}</text>
            {ticks.map(tick => <g key={tick}><line x1="96" x2="735" y1={y(tick)} y2={y(tick)} className={tick === 0 ? 'ax-flow-zero' : undefined} /><text x="84" y={y(tick) + 4} textAnchor="end">{compact(tick)}</text></g>)}
            {panel.bars.flatMap((trace, traceIndex) => (byName.get(trace.name) || []).map((value, index) => value == null || !Number.isFinite(value) ? null : <rect key={`${trace.name}-${index}`} data-flow-bar={trace.name} x={x(index) + (traceIndex - (panel.bars.length - 1) / 2) * (width + 2) - width / 2} y={Math.min(y(value), y(0))} width={width} height={Math.abs(y(0) - y(value))} fill={value < 0 ? 'var(--red)' : trace.color}><title>{`${trace.title} · ${result.bars[index]?.timestamp.slice(0, 16).replace('T', ' ')} UTC · ${exact(value)}`}</title></rect>))}
            <path d={path} fill="none" stroke={panel.line.color} strokeWidth="2.5" strokeLinejoin="round" data-flow-line={panel.line.name} />
            {lineValues.map((value, index) => value == null || !Number.isFinite(value) ? null : <circle key={index} cx={x(index)} cy={y(value)} r="2.25" fill={panel.line.color}><title>{`${panel.line.title} · ${result.bars[index]?.timestamp.slice(0, 16).replace('T', ' ')} UTC · ${exact(value)}`}</title></circle>)}
            <text x="96" y="187">{start} UTC</text><text x="735" y="187" textAnchor="end">{end} UTC</text>
          </g>;
        })}
      </svg>
      <figcaption><div className="ax-series-legend">{panels.flatMap(panel => [...panel.bars, panel.line]).map(trace => <span key={trace.name}><i style={{ backgroundColor: trace.color, color: trace.color }} />{trace.title}</span>)}</div>横轴为最近 24 根连续已收盘的 Binance 现货 1 小时 K 线。红色柱表示负差值；窄屏可左右滑动查看完整时段，两种资产单位分图。</figcaption>
    </figure>
    <dl className="ax-flow-metrics">{metrics.map(([label, key, unit]) => <div key={key}><dt>{label}（{unit}）</dt><dd>{exact(result.values[key])}</dd></div>)}</dl>
    <p className="ax-practice-reading">{cvd ? 'CVD 从这 24 根 K 线的窗口起点按零开始累计主动买卖差；窗口外历史不在图中，不能把这里的数值称为全市场历史 CVD。价格横盘时 CVD 上升也可能是限价卖单吸收，须结合之后的价格观察。' : '主动买入取 Binance K 线中的 taker-buy 原始字段；主动卖出是总成交减主动买入。这不是 A 股内外盘的等价数据，也不代表资金净流入。'}</p>
  </div>;
}
