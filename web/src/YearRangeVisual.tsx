import type { PracticeResult } from './types';

const price = (value: number) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 4 }).format(value);

export function YearRangeVisual({ result }: { result: PracticeResult }) {
  const range = result.year_range;
  if (!range) return null;
  const low = result.values.low_52w;
  const high = result.values.high_52w;
  const close = result.values.latest_close;
  const position = result.values.position_in_range;
  if (low == null || high == null || close == null) return null;
  const flat = position == null;
  const marker = flat ? 50 : Math.max(2, Math.min(98, position * 100));
  return <figure className="ax-year-range" role="img" aria-label={`${result.symbol} 52周价格区间：低点${price(low)}，最新收盘${price(close)}，高点${price(high)}`}>
    <div className="ax-year-range-head"><strong>52 周价格位置</strong><span>{result.symbol} · 截至 {range.as_of.slice(0, 10)}</span></div>
    <div className="ax-year-range-track" aria-hidden="true"><span className="ax-year-range-marker" style={{ left: `${marker}%` }} /></div>
    <div className="ax-year-range-prices"><span>区间低点 <b>{price(low)}</b></span><span>最新收盘 <b>{price(close)}</b></span><span>区间高点 <b>{price(high)}</b></span></div>
    <figcaption>{flat ? '高低点相同，区间位置无定义。' : `区间位置 ${(position * 100).toFixed(1)}%；距高点 ${((result.values.distance_from_high ?? 0) * 100).toFixed(2)}%。`} {range.window_start.slice(0, 10)} 后至 {range.window_end.slice(0, 10)}，{range.bar_count} 根已收盘日线。提供者 OHLC 的复权口径未经统一核验，交易日完整性也未经核验。</figcaption>
  </figure>;
}
