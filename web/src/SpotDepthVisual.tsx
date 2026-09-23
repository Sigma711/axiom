import type { PracticeResult } from './types';

const number = (value: number | null | undefined, digits = 8) => value == null ? '—' : value.toLocaleString('zh-CN', { maximumFractionDigits: digits });

/** One exchange order-book snapshot. Mirrored bars compare quantities at equal rank, not equal price. */
export function SpotDepthVisual({ result }: { result: PracticeResult }) {
  const bids = result.levels?.bids || [];
  const asks = result.levels?.asks || [];
  const base = result.asset_units?.base_asset || result.symbol.replace(/USDT$/, '') || '基础资产';
  const count = Math.max(bids.length, asks.length);
  const maxQuantity = Math.max(1, ...bids.map(level => level.quantity), ...asks.map(level => level.quantity));
  const spread = result.concept_id === 'bid_ask_spread';
  const caution = result.concept_id === 'book_pitfall_order_imbalance';
  const imbalance = result.values.order_imbalance;
  return <div className="ax-depth-practice">
    <figure className="ax-knowledge-chart ax-depth-chart">
      <svg role="img" aria-label={`Binance ${result.symbol} 现货买卖盘快照：按档位比较数量`} viewBox="0 0 760 310">
        <text x="72" y="27" className="ax-depth-title">买盘 · Bid</text>
        <text x="688" y="27" className="ax-depth-title" textAnchor="end">卖盘 · Ask</text>
        <text x="72" y="51">报价（USDT）</text><text x="326" y="51" textAnchor="end">数量（{base}）</text>
        <text x="434" y="51">数量（{base}）</text><text x="688" y="51" textAnchor="end">报价（USDT）</text>
        <line x1="380" x2="380" y1="62" y2="285" className="ax-depth-axis" />
        {Array.from({ length: count }, (_, index) => {
          const bid = bids[index];
          const ask = asks[index];
          const y = 78 + index * 42;
          const bidWidth = bid ? bid.quantity / maxQuantity * 116 : 0;
          const askWidth = ask ? ask.quantity / maxQuantity * 116 : 0;
          return <g key={index}>
            <text x="380" y={y + 15} textAnchor="middle" className="ax-depth-rank">{index + 1}</text>
            {bid && <>
              <rect data-depth-bar="bid" x={334 - bidWidth} y={y} width={bidWidth} height="22" rx="3" className="ax-depth-bid-bar"><title>{`买 ${index + 1} 档 · ${number(bid.price)} USDT · ${number(bid.quantity)} ${base}`}</title></rect>
              <text x="72" y={y + 16}>{number(bid.price)}</text><text x="326" y={y + 16} textAnchor="end">{number(bid.quantity)}</text>
            </>}
            {ask && <>
              <rect data-depth-bar="ask" x="426" y={y} width={askWidth} height="22" rx="3" className="ax-depth-ask-bar"><title>{`卖 ${index + 1} 档 · ${number(ask.price)} USDT · ${number(ask.quantity)} ${base}`}</title></rect>
              <text x="434" y={y + 16}>{number(ask.quantity)}</text><text x="688" y={y + 16} textAnchor="end">{number(ask.price)}</text>
            </>}
          </g>;
        })}
      </svg>
      <figcaption>每行对应买一/卖一、买二/卖二等相同档位；柱长按同一数量刻度绘制，报价各自标注。快照只有更新编号，没有交易所历史时间戳。</figcaption>
    </figure>
    <dl className="ax-flow-metrics">
      <div><dt>买一价（USDT）</dt><dd>{number(result.values.best_bid)}</dd></div>
      <div><dt>卖一价（USDT）</dt><dd>{number(result.values.best_ask)}</dd></div>
      {spread ? <>
        <div><dt>买卖价差（USDT）</dt><dd>{number(result.values.absolute_spread)}</dd></div>
        <div><dt>相对价差（以中间价为分母）</dt><dd>{result.values.relative_spread == null ? '—' : `${number(result.values.relative_spread * 100, 8)}%`}</dd></div>
      </> : <>
        <div><dt>前 {result.values.top_n ?? count} 档买盘数量（{base}）</dt><dd>{number(result.values.top_n_bid_quantity)}</dd></div>
        <div><dt>前 {result.values.top_n ?? count} 档卖盘数量（{base}）</dt><dd>{number(result.values.top_n_ask_quantity)}</dd></div>
        <div><dt>委比 · (买量 − 卖量) / 总量</dt><dd>{imbalance == null ? '—' : `${number(imbalance * 100, 2)}%`}</dd></div>
      </>}
      <div><dt>交易所快照更新编号</dt><dd>{result.depth_snapshot?.update_id == null ? '—' : number(result.depth_snapshot.update_id, 0)}</dd></div>
    </dl>
    <p className="ax-practice-reading">{spread ? '价差是买一与卖一之间的报价距离，实际成交还会受可用数量、手续费和行情变化影响。' : caution ? '委比只描述这一帧可见的前几档挂单；挂单可以撤销或移动，委比高不代表价格随后必涨。' : '委比比较这一帧前几档的挂单数量，不是已经成交的买卖量，也不能单独当作交易信号。'}</p>
  </div>;
}
