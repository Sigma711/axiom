import type { PracticeResult } from './types';

const number = (value: number, digits = 6) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: digits }).format(value);

export function RecentTradesVisual({ result }: { result: PracticeResult }) {
  const window = result.recent_trades;
  if (!window) return null;
  const base = result.asset_units?.base_asset || result.symbol.replace(/USDT$/, '');
  const net = result.concept_id === 'book_net_volume';
  const levels = result.profile_levels || [];
  const chartWidth = Math.max(720, 90 + levels.length * 8);
  const barWidth = (chartWidth - 90) / Math.max(levels.length, 1);
  const maxVolume = Math.max(1, ...levels.map(level => level.volume));
  const volumes = [
    { label: '上涨 Tick', value: result.values.uptick_volume ?? 0, kind: 'up' },
    { label: '下跌 Tick', value: result.values.downtick_volume ?? 0, kind: 'down' },
    { label: '同价 Tick', value: result.values.neutral_volume ?? 0, kind: 'neutral' },
  ];
  const maxDirectionVolume = Math.max(1, ...volumes.map(item => item.value));
  return <section className="ax-recent-trades" aria-label={net ? '逐笔净成交量图' : '逐笔成交量分布图'}>
    <p className="ax-recent-trades-window">Binance {result.symbol} 现货 · 最近 {window.trade_count} 笔逐笔成交 · {window.first_time} 至 {window.last_time} · ID {window.first_trade_id}–{window.last_trade_id}</p>
    {net ? <figure className="ax-trade-directions" role="img" aria-label={`最近逐笔窗口上涨、下跌、同价成交量与净成交量，单位 ${base}`}>
      {volumes.map(item => <div className="ax-trade-direction" key={item.kind}>
        <span>{item.label}</span><div className="ax-trade-direction-track"><i className={`ax-trade-direction-bar ${item.kind}`} style={{ width: `${item.value / maxDirectionVolume * 100}%` }} /></div><b>{number(item.value)} {base}</b>
      </div>)}
      <figcaption>净成交量 <strong>{number(result.values.net_volume ?? 0)} {base}</strong> = 上涨 Tick 量 − 下跌 Tick 量。首笔只作前价锚点，不计入分类；同价成交单列，未归入涨跌。这里的 Tick 指逐笔成交价相对前一笔的变化，不是主动买卖方向。</figcaption>
    </figure> : <figure className="ax-trade-profile">
      <svg role="img" aria-label={`最近逐笔窗口全部 ${levels.length} 个实际成交价位的成交量分布`} width={chartWidth} height="300" viewBox={`0 0 ${chartWidth} 300`}>
        <text x="45" y="23">成交量（{base}）</text>
        {[0, maxVolume / 2, maxVolume].map(value => <g key={value}><line x1="45" x2={chartWidth - 35} y1={260 - value / maxVolume * 195} y2={260 - value / maxVolume * 195} /><text x="39" y={264 - value / maxVolume * 195} textAnchor="end">{number(value, 3)}</text></g>)}
        {levels.map((level, index) => <rect key={level.price} data-price-level={level.price} x={48 + index * barWidth} y={260 - level.volume / maxVolume * 195} width={Math.max(1, barWidth - 1)} height={level.volume / maxVolume * 195} className={level.is_poc ? 'poc' : level.in_value_area ? 'value-area' : 'outside'}><title>{`${number(level.price)} USDT · ${number(level.volume)} ${base}${level.is_poc ? ' · POC' : ''}`}</title></rect>)}
        {levels[0] && <text x="48" y="282">{number(levels[0].price)} USDT</text>}
        {levels.length > 1 && <text x={chartWidth - 35} y="282" textAnchor="end">{number(levels[levels.length - 1].price)} USDT</text>}
      </svg>
      <figcaption>每根柱对应真实成交的一个价格层；金色为最大成交量价位 POC，蓝色为围绕 POC 扩展出的约 70% 价值区。POC {result.values.poc == null ? '—' : number(result.values.poc)} USDT · 价值区 {result.values.value_area_low == null ? '—' : number(result.values.value_area_low)}–{result.values.value_area_high == null ? '—' : number(result.values.value_area_high)} USDT · 覆盖 {result.values.included_fraction == null ? '—' : `${number(result.values.included_fraction * 100, 2)}%`}。横向滚动可查看全部价位；这只是最近逐笔窗口，不是全天成交分布。</figcaption>
    </figure>}
  </section>;
}
