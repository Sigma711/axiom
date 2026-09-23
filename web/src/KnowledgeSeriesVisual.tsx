import type { PracticeResult } from './types';

const colors = ['var(--accent-2)', 'var(--accent)', 'var(--green)', 'var(--red)', 'var(--text)'];
const labels: Record<string, string> = { upper: '上轨', middle: '中轨', lower: '下轨', macd: 'MACD 快慢线差', signal: '信号线', histogram: '柱状差值', close: '收盘价', price: '价格', drawdown: '回撤', equity: '净值', volume: '成交量', hourly_volume: '每小时成交量', plus_di: '正向指标', minus_di: '负向指标', pivot_high_occurrence: '局部高点发生位置（仅回看）', pivot_low_occurrence: '局部低点发生位置（仅回看）', confirmed_pivot_high: '局部高点确认价（t+2）', confirmed_pivot_low: '局部低点确认价（t+2）', confirmation_delay_bars: '确认延迟', close_price: '真实收盘价', short_horizon_start: '短期窗口起点（5 根）', long_horizon_start: '长期窗口起点（20 根）', macd_histogram_x1: 'MACD 柱体 ×1', macd_histogram_x2: 'MACD 柱体 ×2', histogram_scale_difference: '两种口径差值', last_completed_close_series: '上一根已收盘价', current_candle_open_series: '当前 K 线开盘价', provisional_close_series: '当前临时价（尚未收盘）' };
const units: Record<string, string> = { currency: '价格 / 元', units: '数量 / 标的基本单位', price: '价格', fraction: '比例（小数）', percent: '百分比', '%': '百分比', ratio: '比率', shares: '股', volume: '成交量', bars: '根 K 线', macd_price: 'MACD 柱体（价格单位）', '': '指标值' };
const number = (v: number) => Math.abs(v) >= 10000 ? `${+(v / 10000).toPrecision(4)}万` : `${+v.toPrecision(4)}`;
/** Unit-separated panels retain every trace and every missing observation. */
export function KnowledgeSeriesVisual({ name, result }: { name: string; result: PracticeResult }) {
  const groups = new Map<string, PracticeResult['series']>();
  result.series.forEach(series => {
    const unit = result.units?.[series.name] || '';
    groups.set(unit, [...(groups.get(unit) || []), series]);
  });
  const panels = [...groups.entries()];
  const repainting = result.series.some(series => series.name === 'pivot_high_occurrence');
  const openCandle = result.concept_id === 'book_pitfall_open_candle';
  const wide = repainting || openCandle || result.series.some(series => series.name === 'short_horizon_start' || series.name === 'macd_histogram_x1');
  const right = wide ? 736 : 396;
  return <figure className={`ax-knowledge-chart ax-series-illustration${wide ? ' ax-wide-series-chart' : ''}${repainting ? ' ax-repainting-chart' : ''}`}>
    <svg viewBox={`0 0 ${wide ? 760 : 420} ${panels.length * 185}`} role="img" aria-label={`${name} 全部序列，按单位分图`}>
      {panels.map(([unit, series], panel) => {
        const values = series.flatMap(s => s.values.filter((v): v is number => v != null && Number.isFinite(v)));
        if (!values.length) return null;
        const lo = Math.min(...values), hi = Math.max(...values);
        const padding = (hi - lo || Math.max(Math.abs(hi), 1)) * 0.08;
        const min = lo - padding, max = hi + padding;
        const count = Math.max(...series.map(s => s.values.length));
        const y = (v: number) => 147 - (v - min) / (max - min) * 114;
        const x = (index: number) => 64 + index / Math.max(count - 1, 1) * (right - 64);
        const sampleLabel = (index: number) => {
          if (openCandle) {
            const seconds = index === 0 ? result.values.last_completed_timestamp : result.values.current_candle_open_timestamp;
            if (seconds != null && Number.isFinite(seconds)) return `${new Date(seconds * 1000).toISOString().slice(0, 16).replace('T', ' ')} UTC${index === 1 ? ' · 未收盘' : ''}`;
          }
          return wide && result.bars?.length === count
            ? `${result.bars[index].timestamp.slice(0, 16).replace('T', ' ')} UTC`
            : `样本 ${index + 1}`;
        };
        return <g key={unit} transform={`translate(0,${panel * 185})`}>
          <text x="64" y="20">{units[unit] || unit}</text>
          {[min, (min + max) / 2, max].map(value => <g key={value}><line x1="64" x2={right} y1={y(value)} y2={y(value)} /><text x="56" y={y(value) + 4} textAnchor="end">{number(value)}</text></g>)}
          <text x="64" y="171">{sampleLabel(0)}</text><text x={right} y="171" textAnchor="end">{sampleLabel(count - 1)}</text>
          {series.map((s, trace) => {
            const finite = (value: number | null | undefined): value is number => value != null && Number.isFinite(value);
            const retrospective = s.name.endsWith('_occurrence');
            let connected = false;
            const path = s.values.map((value, i) => {
              if (value == null || !Number.isFinite(value)) { connected = false; return ''; }
              const command = connected ? 'L' : 'M'; connected = true;
              return `${command}${x(i)},${y(value)}`;
            }).join(' ');
            const color = colors[trace % colors.length];
            return <g key={s.name} aria-label={labels[s.name] || s.name}>
              <path d={path} style={{ fill: 'none', stroke: color, strokeWidth: 2 }} strokeDasharray={trace > 3 ? '4 3' : undefined} />
              {s.values.map((value, i) => !finite(value) || finite(s.values[i - 1]) || finite(s.values[i + 1]) ? null : <circle key={i} data-series-marker={s.name} aria-label={`${labels[s.name] || s.name} 标记`} cx={x(i)} cy={y(value)} r="3" fill={retrospective ? 'var(--bg)' : color} stroke={color} strokeWidth="2"><title>{`${labels[s.name] || s.name} · ${sampleLabel(i)} · ${number(value)}`}</title></circle>)}
            </g>;
          })}
        </g>;
      })}
    </svg>
    <figcaption><div className="ax-series-legend">{panels.flatMap(([unit, series]) => series.map((s, i) => <span key={`${unit}-${s.name}`}><i className={s.name.endsWith('_occurrence') ? 'ax-legend-retrospective' : s.name.startsWith('confirmed_pivot') ? 'ax-legend-confirmed' : ''} style={{ backgroundColor: colors[i % colors.length], color: colors[i % colors.length] }} />{labels[s.name] || s.name}</span>))}</div>{openCandle ? '横轴为 Binance 实际 K 线开盘时间；右侧仅为抓取时的临时快照，尚未收盘，数值可能继续变化。' : <>{wide && result.bars?.length ? '横轴按实际 K 线时间排序；' : '横轴为样本顺序；'}不同单位分图，空白保留预热期或缺失数据。</>}</figcaption>
  </figure>;
}
