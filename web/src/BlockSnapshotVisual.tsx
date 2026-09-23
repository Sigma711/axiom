import type { PracticeResult } from './types';

const whole = (value: number) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 0 }).format(value);
const compactHash = (hash: string) => `${hash.slice(0, 8)}…${hash.slice(-6)}`;

export function BlockSnapshotVisual({ result }: { result: PracticeResult }) {
  const snapshot = result.block_snapshot;
  const blocks = result.blocks || [];
  if (!snapshot || !blocks.length) return null;
  const heights = result.concept_id === 'book_block_height';
  const maxBytes = Math.max(...blocks.map(block => block.size_bytes), 1);
  const meanBytes = result.values.mean_size_bytes ?? 0;
  const fetched = new Date(snapshot.fetched_at);
  const fetchedLabel = Number.isFinite(fetched.getTime()) ? `${fetched.toISOString().slice(0, 19).replace('T', ' ')} UTC` : snapshot.fetched_at;
  const width = 860;
  return <figure className="ax-block-snapshot" aria-label={heights ? '比特币主网区块高度关系图' : '比特币主网区块实际字节大小图'}>
    <p className="ax-block-source">Bitcoin 主网 · {snapshot.provider} 最近 {snapshot.observed_block_count} 个已观测区块 · 抓取于 {fetchedLabel}</p>
    {heights ? <div className="ax-block-scroll"><svg role="img" aria-label={`从高度 ${snapshot.first_height} 到 ${snapshot.last_height} 的哈希相连区块，共相隔 ${result.values.blocks_since_reference} 个区块`} viewBox={`0 0 ${width} 205`} width={width} height="205">
      <line className="ax-block-link" x1="55" x2="805" y1="89" y2="89" />
      {blocks.map((block, index) => {
        const x = 55 + index * (750 / Math.max(1, blocks.length - 1));
        return <g key={block.hash} data-height={block.height}>
          <circle className={index === 0 || index === blocks.length - 1 ? 'ax-block-endpoint' : 'ax-block-node'} cx={x} cy="89" r={index === 0 || index === blocks.length - 1 ? 21 : 16} />
          <text className="ax-block-height" x={x} y="52" textAnchor="middle">{whole(block.height)}</text>
          {(index === 0 || index === blocks.length - 1) && <text className="ax-block-hash" x={x} y="137" textAnchor={index === 0 ? 'start' : 'end'}>{compactHash(block.hash)}</text>}
        </g>;
      })}
      <text className="ax-block-axis" x="55" y="178">较早观测</text><text className="ax-block-axis" x="805" y="178" textAnchor="end">最新观测</text>
    </svg></div> : <div className="ax-block-scroll"><svg role="img" aria-label={`${blocks.length} 个区块的实际序列化字节大小，平均 ${whole(meanBytes)} 字节`} viewBox={`0 0 ${width} 280`} width={width} height="280">
      <text className="ax-block-axis" x="50" y="24">实际区块大小（MB，10⁶ 字节）</text>
      {[0, maxBytes / 2, maxBytes].map((value, index) => <g key={index}><line className="ax-block-grid" x1="58" x2="826" y1={230 - value / maxBytes * 165} y2={230 - value / maxBytes * 165} /><text className="ax-block-tick" x="48" y={234 - value / maxBytes * 165} textAnchor="end">{(value / 1_000_000).toFixed(2)}</text></g>)}
      <line className="ax-block-mean" x1="58" x2="826" y1={230 - meanBytes / maxBytes * 165} y2={230 - meanBytes / maxBytes * 165} />
      {blocks.map((block, index) => {
        const step = 760 / blocks.length;
        const height = block.size_bytes / maxBytes * 165;
        return <g key={block.hash} data-height={block.height} data-size-bytes={block.size_bytes}>
          <rect className="ax-block-size-bar" x={62 + index * step} y={230 - height} width={Math.max(4, step - 12)} height={height}><title>{`区块 ${block.height} · ${whole(block.size_bytes)} 字节`}</title></rect>
          <text className="ax-block-tick" x={62 + index * step + Math.max(4, step - 12) / 2} y="252" textAnchor="middle">{block.height}</text>
        </g>;
      })}
    </svg></div>}
    <figcaption>{heights
      ? `首块 ${whole(snapshot.first_height)} → 最新块 ${whole(snapshot.last_height)}，相差 ${whole(result.values.blocks_since_reference ?? 0)} 个高度；十个区块只有九段相邻关系。相邻区块的前块哈希与上一个哈希已核对。`
      : `这 ${blocks.length} 个区块的实际序列化大小合计 ${whole(result.values.total_size_bytes ?? 0)} 字节，平均 ${whole(meanBytes)} 字节；虚拟字节和区块权重是不同口径。虚线表示本次样本均值。`}</figcaption>
    <p className="ax-block-caveat">这是公开区块浏览器在抓取时看到的主链片段，可能随链重组改变；不是独立节点验证，也不是 Binance 的成交数据。</p>
  </figure>;
}
