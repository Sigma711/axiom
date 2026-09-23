import type { PracticeResult } from './types';

const number = (value: number, digits = 2) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: digits }).format(value);

export function TransactionSampleVisual({ result }: { result: PracticeResult }) {
  const sample = result.transaction_sample;
  if (!sample) return null;
  const transactions = result.transactions || [];
  const fees = result.concept_id === 'book_transaction_fees';
  const values = transactions.map(tx => fees ? tx.fee_sats : tx.size_bytes);
  const max = Math.max(1, ...values);
  const mean = fees ? result.values.mean_fee_sats ?? 0 : result.values.mean_size_bytes ?? 0;
  const median = fees ? result.values.median_fee_sats ?? 0 : result.values.median_size_bytes ?? 0;
  const total = fees ? result.values.total_fee_sats ?? 0 : result.values.total_size_bytes ?? 0;
  const unit = fees ? 'sat' : '字节';
  const width = Math.max(740, 70 + transactions.length * 31);
  const barStep = (width - 84) / Math.max(1, transactions.length);
  const fetched = new Date(sample.fetched_at);
  const fetchedLabel = Number.isFinite(fetched.getTime()) ? `${fetched.toISOString().slice(0, 19).replace('T', ' ')} UTC` : sample.fetched_at;
  return <figure className="ax-transaction-sample" aria-label={fees ? 'Bitcoin 普通交易手续费样本图' : 'Bitcoin 普通交易实际字节样本图'}>
    <p className="ax-transaction-source">Bitcoin 主网 · 区块 #{number(sample.block_height, 0)} · {sample.provider} · 抓取于 {fetchedLabel}</p>
    <p className="ax-transaction-scope">这个区块之后，在抓取窗口中又观察到 {sample.observed_newer_blocks} 个区块。区块首页返回 {sample.returned_count} 笔，排除 {sample.excluded_coinbase_count} 笔 coinbase，本图分析 {sample.analyzed_count} 笔普通交易。<a href={`https://blockstream.info/block/${sample.block_hash}`} target="_blank" rel="noreferrer">查看区块 ↗</a></p>
    {transactions.length ? <div className="ax-transaction-scroll"><svg role="img" aria-label={`${transactions.length} 笔普通交易的${fees ? '手续费金额' : '实际序列化字节数'}，样本均值 ${number(mean)} ${unit}`} viewBox={`0 0 ${width} 285`} width={width} height="285">
      <text className="ax-transaction-axis" x="58" y="23">{fees ? '手续费金额（sat）' : '实际交易大小（字节）'}</text>
      {[0, max / 2, max].map((value, index) => <g key={index}><line className="ax-transaction-grid" x1="58" x2={width - 24} y1={234 - value / max * 164} y2={234 - value / max * 164} /><text className="ax-transaction-tick" x="49" y={238 - value / max * 164} textAnchor="end">{number(value, 0)}</text></g>)}
      <line className="ax-transaction-mean" x1="58" x2={width - 24} y1={234 - mean / max * 164} y2={234 - mean / max * 164} />
      {transactions.map((tx, index) => {
        const barHeight = values[index] / max * 164;
        return <g key={tx.txid} data-txid={tx.txid} data-value={values[index]}>
          <rect className={values[index] === 0 ? 'ax-transaction-bar ax-transaction-zero' : 'ax-transaction-bar'} x={62 + index * barStep} y={234 - Math.max(2, barHeight)} width={Math.max(3, barStep - 5)} height={Math.max(2, barHeight)}><title>{`第 ${index + 1} 笔 · ${number(values[index])} ${unit} · ${tx.txid}`}</title></rect>
          {(index === 0 || index === transactions.length - 1 || (index + 1) % 4 === 0) && <text className="ax-transaction-tick" x={62 + index * barStep + Math.max(3, barStep - 5) / 2} y="257" textAnchor="middle">{index + 1}</text>}
        </g>;
      })}
    </svg></div> : <p className="ax-transaction-empty">这个区块首页没有可分析的普通交易。</p>}
    {transactions.length > 0 && <p className="ax-transaction-scroll-hint">左右滑动查看全部交易 →</p>}
    {transactions.length > 0 && <figcaption>本页普通交易合计 {number(total)} {unit}，均值 {number(mean)} {unit}，中位数 {number(median)} {unit}。虚线表示本次样本均值。{fees && values.includes(0) ? `短横条代表 ${values.filter(value => value === 0).length} 笔零手续费交易。` : ''}</figcaption>}
    <p className="ax-transaction-caveat">这是固定区块首页的非随机样本，最多 24 笔普通交易；矿工的交易排序会影响结果。它不是完整区块、全网统计，也不声称达到共识终局性。{fees ? '手续费金额不同于每虚拟字节费率。' : '实际字节不同于虚拟字节和区块权重。'}</p>
  </figure>;
}
