import type { PracticeResult } from './types';

const display = (value: number, digits = 2) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: digits }).format(value);

export function BlockTimingVisual({ result }: { result: PracticeResult }) {
  const snapshot = result.block_snapshot;
  const blocks = result.blocks || [];
  const intervals = result.intervals || [];
  if (!snapshot || blocks.length !== 10 || intervals.length !== 9) return null;
  const timing = result.concept_id === 'book_block_interval';
  const values = timing ? intervals.map(item => item.seconds) : blocks.slice(1).map(block => block.tx_count ?? 0);
  const minimum = Math.min(0, ...values);
  const maximum = Math.max(0, ...values);
  const range = Math.max(1, maximum - minimum);
  const y = (value: number) => 236 - (value - minimum) / range * 162;
  const zeroY = y(0);
  const fetched = new Date(snapshot.fetched_at);
  const fetchedLabel = Number.isFinite(fetched.getTime()) ? `${fetched.toISOString().slice(0, 19).replace('T', ' ')} UTC` : snapshot.fetched_at;
  return <figure className="ax-block-timing" aria-label={timing ? 'Bitcoin 相邻区块头时间戳差图' : 'Bitcoin 九个区块确认交易数图'}>
    <p className="ax-block-timing-source">Bitcoin 主网 · {snapshot.provider} · 高度 {display(snapshot.first_height, 0)}—{display(snapshot.last_height, 0)} · 抓取于 {fetchedLabel}</p>
    <p className="ax-block-timing-scope">十个相连区块形成九段相邻关系；最早区块只作为时间锚点。<a href={`https://blockstream.info/block/${snapshot.last_hash}`} target="_blank" rel="noreferrer">查看最新区块 ↗</a></p>
    <div className="ax-block-timing-scroll"><svg role="img" aria-label={timing ? '九段区块头时间戳差，负值和零值保留' : '九个后续区块各自确认的交易数，含 coinbase'} viewBox="0 0 860 285" width="860" height="285">
      <text className="ax-block-timing-axis" x="58" y="25">{timing ? '相邻区块头时间戳差（秒）' : '每区块确认交易数（含 coinbase）'}</text>
      {[minimum, 0, maximum].filter((value, index, list) => list.indexOf(value) === index).map(value => <g key={value}><line className={value === 0 ? 'ax-block-timing-zero' : 'ax-block-timing-grid'} x1="59" x2="832" y1={y(value)} y2={y(value)} /><text className="ax-block-timing-tick" x="49" y={y(value) + 4} textAnchor="end">{display(value, 0)}</text></g>)}
      {values.map((value, index) => {
        const left = 64 + index * 85;
        const top = Math.min(y(value), zeroY);
        const height = Math.max(2, Math.abs(y(value) - zeroY));
        const block = blocks[index + 1];
        return <g key={block.hash} data-height={block.height} data-value={value}>
          <rect className={timing && value <= 0 ? 'ax-block-timing-bar ax-block-timing-nonpositive' : 'ax-block-timing-bar'} x={left} y={top} width="54" height={height}><title>{timing ? `高度 ${blocks[index].height} → ${block.height}：${display(value, 0)} 秒` : `高度 ${block.height}：${display(value, 0)} 笔交易（含 coinbase）`}</title></rect>
          <text className="ax-block-timing-tick" x={left + 27} y="259" textAnchor="middle">{block.height}</text>
        </g>;
      })}
    </svg></div>
    <p className="ax-block-timing-scroll-hint">左右滑动查看全部区块 →</p>
    {timing ? <figcaption>九段头时间戳差合计 {display(result.values.total_declared_span_seconds ?? 0)} 秒，均值 {display(result.values.mean_block_interval_seconds ?? 0)} 秒，中位数 {display(result.values.median_block_interval_seconds ?? 0)} 秒；其中 {display(result.values.nonpositive_interval_count ?? 0, 0)} 段不大于零。图中保留负值和零值，不把它们改成正数。</figcaption>
      : <figcaption>后九个区块共确认 {display(result.values.confirmed_transaction_count ?? 0, 0)} 笔交易，首块不计入分子；首末区块头时间戳相差 {display(result.values.elapsed_seconds ?? 0)} 秒。{result.values.transaction_rate == null ? '首末时间戳差不大于零，本次速率无法定义。' : `样本速率 ${display(result.values.transaction_rate, 4)} 笔/秒。`}交易数包含每块的 coinbase。</figcaption>}
    <p className="ax-block-timing-caveat">横轴按区块高度排序；纵轴时间来自区块头声明的时间戳，不是实测出块耗时。样本仅十块，可能受链重组影响；不代表全网长期速率或未来价格。</p>
  </figure>;
}
