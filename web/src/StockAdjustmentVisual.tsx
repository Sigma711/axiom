import type { PracticeResult } from './types';
import './StockAdjustmentVisual.css';

export function StockAdjustmentVisual({ result }: { result: PracticeResult }) {
  const evidence = result.adjustment_evidence;
  if (!evidence) return null;
  const split = evidence.event;
  return <figure aria-label="苹果公司历史拆股示意" className="ax-stock-adjustment">
    <figcaption><strong>一股变四股</strong><span>Apple · {split.effective_trading_date} 生效的历史拆股</span></figcaption>
    <div className="ax-stock-adjustment-flow" role="img" aria-label={`拆股前 1 股，拆股后 ${split.numerator / split.denominator} 股，每股理论价格按 ${split.denominator}/${split.numerator} 缩放`}>
      <div><span className="ax-stock-share" /><p>拆股前 · 1 股</p></div>
      <span aria-hidden="true" className="ax-stock-adjustment-arrow">→</span>
      <div><span className="ax-stock-shares">{Array.from({ length: Math.min(8, Math.round(split.numerator / split.denominator)) }, (_, index) => <span className="ax-stock-share" key={index} />)}</span><p>拆股后 · {split.numerator / split.denominator} 股</p></div>
    </div>
    <p>同一笔持仓的股数按 {split.numerator}/{split.denominator} 倍变化；只考虑拆股、忽略市场波动时，每股价格对应乘 {split.denominator}/{split.numerator}。这不是收益，也没有把供应商的历史收盘价当成未经复权的原价。</p>
    <details><summary>核对事件与行情来源</summary><p>拆股日期 {split.effective_trading_date} · 供应商 {evidence.provider} · {evidence.observations.length} 根历史日线。供应商 OHLC 与调整收盘价的长期复权口径未经独立核验，因此不据此计算总回报。</p>{evidence.issuer_confirmation_url && <a href={evidence.issuer_confirmation_url} target="_blank" rel="noopener noreferrer">Apple 官方拆股公告 ↗</a>}<a href={evidence.endpoint} target="_blank" rel="noopener noreferrer">查看行情接口 ↗</a></details>
  </figure>;
}
