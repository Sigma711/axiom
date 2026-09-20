import type { PracticeResult } from './types';

const colors = ['var(--accent-2)', 'var(--accent)', 'var(--green)', 'var(--red)', 'var(--text)'];
const labels: Record<string, string> = { upper: '上轨', middle: '中轨', lower: '下轨', macd: 'MACD 快慢线差', signal: '信号线', histogram: '柱状差值', close: '收盘价', price: '价格', drawdown: '回撤', equity: '净值', volume: '成交量', plus_di: '正向指标', minus_di: '负向指标' };
const units: Record<string, string> = { currency: '价格 / 元', price: '价格', fraction: '比例（小数）', percent: '百分比', '%': '百分比', ratio: '比率', shares: '股', volume: '成交量', '': '指标值' };
const number = (v: number) => Math.abs(v) >= 10000 ? `${+(v / 10000).toPrecision(4)}万` : `${+v.toPrecision(4)}`;
/** Unit-separated panels retain every trace and every missing observation. */
export function KnowledgeSeriesVisual({ name, result }: { name: string; result: PracticeResult }) {
  const groups = new Map<string, PracticeResult['series']>();
  result.series.forEach(series => {
    const unit = result.units?.[series.name] || '';
    groups.set(unit, [...(groups.get(unit) || []), series]);
  });
  const panels = [...groups.entries()];
  return <figure className="ax-knowledge-chart ax-series-illustration">
    <svg viewBox={`0 0 420 ${panels.length * 185}`} role="img" aria-label={`${name} 全部序列，按单位分图`}>
      {panels.map(([unit, series], panel) => {
        const values = series.flatMap(s => s.values.filter((v): v is number => v != null && Number.isFinite(v)));
        if (!values.length) return null;
        const lo = Math.min(...values), hi = Math.max(...values);
        const padding = (hi - lo || Math.max(Math.abs(hi), 1)) * 0.08;
        const min = lo - padding, max = hi + padding;
        const count = Math.max(...series.map(s => s.values.length));
        const y = (v: number) => 147 - (v - min) / (max - min) * 114;
        const x = (index: number) => 64 + index / Math.max(count - 1, 1) * 332;
        return <g key={unit} transform={`translate(0,${panel * 185})`}>
          <text x="64" y="20">{units[unit] || unit}</text>
          {[min, (min + max) / 2, max].map(value => <g key={value}><line x1="64" x2="396" y1={y(value)} y2={y(value)} /><text x="56" y={y(value) + 4} textAnchor="end">{number(value)}</text></g>)}
          <text x="64" y="171">样本 1</text><text x="396" y="171" textAnchor="end">样本 {count}</text>
          {series.map((s, trace) => {
            let connected = false;
            const path = s.values.map((value, i) => {
              if (value == null || !Number.isFinite(value)) { connected = false; return ''; }
              const command = connected ? 'L' : 'M'; connected = true;
              return `${command}${x(i)},${y(value)}`;
            }).join(' ');
            const color = colors[trace % colors.length];
            return <g key={s.name} aria-label={labels[s.name] || s.name}>
              <path d={path} style={{ fill: 'none', stroke: color, strokeWidth: 2 }} strokeDasharray={trace > 3 ? '4 3' : undefined} />
              {s.values.filter(value => value != null).length === 1 && s.values.map((value, i) => value == null ? null : <circle key={i} cx={x(i)} cy={y(value)} r="3" fill={color} />)}
            </g>;
          })}
        </g>;
      })}
    </svg>
    <figcaption><div className="ax-series-legend">{panels.flatMap(([unit, series]) => series.map((s, i) => <span key={`${unit}-${s.name}`}><i style={{ backgroundColor: colors[i % colors.length] }} />{labels[s.name] || s.name}</span>))}</div>横轴为样本顺序；不同单位分图，空白保留预热期或缺失数据。</figcaption>
  </figure>;
}
