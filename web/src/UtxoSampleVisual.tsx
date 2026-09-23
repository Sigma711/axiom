import type { PracticeResult } from './types';

const fmt = (value: number, digits = 4) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: digits }).format(value);
const BTC = 100_000_000;

/** A pinned block-page observation, never a snapshot of the global UTXO set. */
export function UtxoSampleVisual({ result }: { result: PracticeResult }) {
  const sample = result.utxo_sample;
  if (!sample) return null;
  const rows = result.utxo_transactions || [];
  const counts = result.concept_id === 'book_utxo_counts';
  const valueStats = result.concept_id === 'book_utxo_value_stats';
  const pairs = rows.map(row => counts
    ? [row.spent_prevout_count, row.created_non_op_return_output_count]
    : [row.spent_prevout_value_sats / BTC, row.created_non_op_return_value_sats / BTC]);
  const max = Math.max(1, ...pairs.flat());
  const width = Math.max(740, 110 + rows.length * 38);
  const step = (width - 134) / Math.max(1, rows.length);
  const unit = counts ? '个' : 'BTC';
  const blockLink = `https://blockstream.info/block/${sample.block_hash}`;
  return <figure className="ax-utxo-sample" aria-label={counts ? 'Bitcoin 区块首页观察到的输入与输出数量图' : 'Bitcoin 区块首页观察到的输入与输出价值图'}>
    <p className="ax-utxo-source">Bitcoin 主网 · 区块 #{fmt(sample.block_height, 0)} · {sample.provider}</p>
    <p className="ax-utxo-scope">固定区块交易首页返回 {fmt(sample.returned_count, 0)} 笔，排除 {fmt(sample.excluded_coinbase_count, 0)} 笔 coinbase；观察 {fmt(sample.sampled_noncoinbase_transaction_count, 0)} 笔普通交易。每笔花费的是输入引用的既有输出；新建数量仅统计非 OP_RETURN 输出。<a href={blockLink} target="_blank" rel="noreferrer">核对原始区块 ↗</a></p>
    <div className="ax-utxo-legend"><span><i className="ax-utxo-spent-key" /> 花费的输入引用</span><span><i className="ax-utxo-created-key" /> 新建的非 OP_RETURN 输出</span></div>
    {rows.length ? <div className="ax-utxo-scroll"><svg role="img" aria-label={`${rows.length} 笔普通交易花费与新建输出的${counts ? '数量' : '价值'}，单位 ${unit}`} width={width} height="278" viewBox={`0 0 ${width} 278`}>
      <text className="ax-utxo-axis" x="80" y="22">{counts ? '输入 / 输出数量（个）' : '输入 / 输出价值（BTC）'}</text>
      {[0, max / 2, max].map((value, index) => <g key={index}><line className="ax-utxo-grid" x1="78" x2={width - 22} y1={218 - value / max * 144} y2={218 - value / max * 144} /><text className="ax-utxo-tick" x="69" y={222 - value / max * 144} textAnchor="end">{fmt(value, counts ? 0 : 2)}</text></g>)}
      {rows.map((row, index) => {
        const [spent, created] = pairs[index];
        const barWidth = Math.max(3, (step - 10) / 2);
        const x = 82 + index * step;
        const spentHeight = spent / max * 144;
        const createdHeight = created / max * 144;
        return <g key={row.txid} data-utxo-txid={row.txid} data-spent={spent} data-created={created}>
          <rect className="ax-utxo-spent" x={x} y={218 - Math.max(2, spentHeight)} width={barWidth} height={Math.max(2, spentHeight)}><title>{`第 ${index + 1} 笔花费 ${fmt(spent)} ${unit} · ${row.txid}`}</title></rect>
          <rect className="ax-utxo-created" x={x + barWidth + 2} y={218 - Math.max(2, createdHeight)} width={barWidth} height={Math.max(2, createdHeight)}><title>{`第 ${index + 1} 笔新建 ${fmt(created)} ${unit} · ${row.txid}`}</title></rect>
          {(index === 0 || index === rows.length - 1 || (index + 1) % 4 === 0) && <text className="ax-utxo-tick" x={x + barWidth} y="243" textAnchor="middle">{index + 1}</text>}
        </g>;
      })}
    </svg></div> : <p className="ax-utxo-empty">这个固定区块首页没有普通交易，无法计算样本分布。</p>}
    {rows.length > 0 && <p className="ax-utxo-scroll-hint">左右滑动查看全部交易 →</p>}
    {rows.length > 0 && <figcaption>{valueStats
      ? <>本页非 OP_RETURN 输出：均值 {fmt((result.values.created_mean_value_sats ?? 0) / BTC, 8)} BTC，中位数 {fmt((result.values.created_median_value_sats ?? 0) / BTC, 8)} BTC；花费输入：均值 {fmt((result.values.spent_mean_value_sats ?? 0) / BTC, 8)} BTC，中位数 {fmt((result.values.spent_median_value_sats ?? 0) / BTC, 8)} BTC。</>
      : counts
        ? <>本页新建非 OP_RETURN 输出 {fmt(result.values.created_non_op_return_output_count ?? 0, 0)} 个，花费输入引用 {fmt(result.values.spent_prevout_count ?? 0, 0)} 个。全网当前 UTXO 总数：无法从这页样本得出。</>
        : <>本页新建非 OP_RETURN 输出价值 {fmt((result.values.created_non_op_return_value_sats ?? 0) / BTC, 8)} BTC，花费输入引用价值 {fmt((result.values.spent_prevout_value_sats ?? 0) / BTC, 8)} BTC。全网当前 UTXO 总价值：无法从这页样本得出。</>}</figcaption>}
    <p className="ax-utxo-caveat">这是一个已确认区块的第一页交易，最多 24 笔普通交易，不是完整区块、随机样本或全网 UTXO 集。OP_RETURN 输出排除在新建数外；其他未知脚本类型不能保证可花费。本页的“新建”也不表示抓取时仍未花费。</p>
  </figure>;
}
