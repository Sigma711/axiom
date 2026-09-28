import type { PracticeResult } from './types';

const fmt = (value: number) => new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 0 }).format(value);
const compact = (address: string) => address.length > 19 ? `${address.slice(0, 9)}…${address.slice(-7)}` : address;

export function AddressSetVisual({ result }: { result: PracticeResult }) {
  const sample = result.address_sample;
  if (!sample) return null;
  const rows = result.address_transactions || [];
  const sending = new Set(rows.flatMap(row => row.input_addresses));
  const receiving = new Set(rows.flatMap(row => row.output_addresses));
  const shared = [...sending].filter(address => receiving.has(address)).sort();
  const inputOnly = [...sending].filter(address => !receiving.has(address)).sort();
  const outputOnly = [...receiving].filter(address => !sending.has(address)).sort();
  const all = [...new Set([...sending, ...receiving])].sort();
  const missingInputs = rows.reduce((sum, row) => sum + row.missing_input_address_slots, 0);
  const missingOutputs = rows.reduce((sum, row) => sum + row.missing_output_address_slots, 0);
  const excludedOpReturn = rows.reduce((sum, row) => sum + row.excluded_op_return_output_count, 0);
  const blockLink = `${new URL(sample.endpoint).origin}/block/${sample.block_hash}`;
  const Group = ({ kind, title, addresses }: { kind: string; title: string; addresses: string[] }) => <section className={`ax-address-group ${kind}`} aria-label={title}><header><span>{title}</span><strong>{fmt(addresses.length)}</strong></header>{addresses.length ? <ul>{addresses.map(address => <li key={address} data-address={address} title={address}>{compact(address)}</li>)}</ul> : <p>没有可解出的地址</p>}</section>;
  return <figure className="ax-address-sample" aria-label="Bitcoin 交易输入输出脚本地址集合图">
    <p className="ax-address-source">Bitcoin 主网 · 区块 #{fmt(sample.block_height)} · {sample.provider}</p>
    <p className="ax-address-scope">固定区块交易首页返回 {fmt(sample.returned_count)} 笔，排除 {fmt(sample.excluded_coinbase_count)} 笔 coinbase；观察 {fmt(sample.sampled_noncoinbase_transaction_count)} 笔普通交易。<a href={blockLink} target="_blank" rel="noreferrer">核对原始区块 ↗</a></p>
    <div className="ax-address-sets"><Group kind="sending" title="仅输入" addresses={inputOnly} /><Group kind="shared" title="两边都出现" addresses={shared} /><Group kind="receiving" title="仅输出" addresses={outputOnly} /></div>
    <div className="ax-address-transactions" aria-label="本页普通交易"><span>覆盖 {fmt(rows.length)} 笔普通交易</span>{rows.map((row, index) => <span key={row.txid} data-address-txid={row.txid} title={row.txid}>交易 {index + 1} · 输入 {fmt(row.input_addresses.length)} · 输出 {fmt(row.output_addresses.length)}</span>)}</div>
    <div className="ax-address-stats" aria-label="地址集合统计"><span>输入脚本中解出的地址 <b>{fmt(sending.size)}</b></span><span>输出脚本中解出的地址 <b>{fmt(receiving.size)}</b></span><span>合并去重 <b>{fmt(all.length)}</b></span></div>
    <figcaption>读法：左侧是本页交易花费的旧输出脚本里可解出的地址，右侧是本页新输出脚本里可解出的地址；中间代表同一地址在两边都出现。输出包含找零，所以“收到”不等于付款给另一位人。</figcaption>
    <p className="ax-address-caveat">缺少 {fmt(missingInputs)} 个输入槽位、{fmt(missingOutputs)} 个输出槽位的可解出地址；OP_RETURN 输出已排除（{fmt(excludedOpReturn)} 个）。地址不是人或实体，不能只凭这些集合判断谁向谁付款、控制权或资金归属。</p>
  </figure>;
}
