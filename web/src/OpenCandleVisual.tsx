import type { Bar, PracticeResult } from './types';

const price = (value: number) => Number(value.toPrecision(7)).toLocaleString('zh-CN');
const utc = (value: string) => `${value.slice(0, 16).replace('T', ' ')} UTC`;

/** Two actual exchange candles. The right-hand candle is explicitly provisional. */
export function OpenCandleVisual({ result }: { result: PracticeResult }) {
  const completed = result.bars[0];
  const snapshot = result.provisional_snapshot;
  if (!completed || !snapshot || snapshot.is_closed !== false) return null;
  const candles: Bar[] = [completed, snapshot.candle];
  const lows = candles.map(candle => candle.low);
  const highs = candles.map(candle => candle.high);
  const low = Math.min(...lows), high = Math.max(...highs);
  const padding = Math.max((high - low) * 0.12, high * 0.0001);
  const min = low - padding, max = high + padding;
  const y = (value: number) => 145 - (value - min) / (max - min) * 108;
  return <figure className="ax-knowledge-chart ax-series-illustration ax-wide-series-chart ax-open-candle-chart">
    <svg viewBox="0 0 760 205" role="img" aria-label="Binance 已收盘与未收盘 K 线，真实 OHLC 对比">
      <text x="100" y="20">Binance · 1 小时 K 线</text>
      {[min, (min + max) / 2, max].map(value => <g key={value}><line x1="100" x2="736" y1={y(value)} y2={y(value)} /><text x="92" y={y(value) + 4} textAnchor="end">{price(value)}</text></g>)}
      {candles.map((candle, index) => {
        const provisional = index === 1;
        const x = provisional ? 540 : 270;
        const rising = candle.close >= candle.open;
        const color = rising ? 'var(--green)' : 'var(--red)';
        const top = y(Math.max(candle.open, candle.close));
        const bottom = y(Math.min(candle.open, candle.close));
        return <g key={candle.timestamp} data-candle={provisional ? 'provisional' : 'completed'}>
          <line x1={x} x2={x} y1={y(candle.high)} y2={y(candle.low)} stroke={color} strokeWidth="3" strokeDasharray={provisional ? '5 3' : undefined} />
          <rect x={x - 30} y={top} width="60" height={Math.max(bottom - top, 3)} fill={provisional ? 'var(--bg-card)' : color} stroke={color} strokeWidth="3" strokeDasharray={provisional ? '5 3' : undefined} />
          <circle cx={x} cy={y(candle.close)} r="5" fill={provisional ? 'var(--bg-card)' : color} stroke={color} strokeWidth="2"><title>{`${provisional ? '临时价' : '最终收盘价'} ${price(candle.close)}`}</title></circle>
          <text x={x} y="169" textAnchor="middle">{provisional ? '当前快照 · 未收盘' : '上一根 · 已收盘'}</text>
          <text x={x} y="190" textAnchor="middle">{utc(candle.timestamp)}</text>
        </g>;
      })}
    </svg>
    <figcaption>实体和影线按实际开、高、低、收绘制。右侧虚线 K 线只代表抓取时的临时状态；收盘前价格、最高最低与成交量都可能继续变化。</figcaption>
  </figure>;
}
