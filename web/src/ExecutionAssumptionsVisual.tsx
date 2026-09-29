import './ExecutionAssumptionsVisual.css';

export type ExecutionAssumption = {
  id: string;
  description_zh: string;
  simulated: boolean;
  limitation_zh?: string | null;
  source_url?: string | null;
};
export type MarketProvenance = {
  provider: string;
  endpoint: string;
  price_basis: string;
  corporate_actions: string;
};

const providers: Record<string, string> = {
  configured_crypto_endpoint: '配置的兼容行情接口',
  eastmoney: '东方财富', tencent: '腾讯行情', yahoo: 'Yahoo Finance', nasdaq: 'Nasdaq',
  binance_spot: 'Binance 现货', local_csv_cache: '历史 CSV 缓存', caller_provided: '调用方提供', caller_provided_unverified: '调用方提供',
};
const bases: Record<string, string> = {
  unadjusted_requested: '不复权请求；不以备用源前复权数据替换',
  provider_adjustment_unverified: '供应商 OHLC，复权口径未核验',
  provider_quote_and_adjusted_close_semantics_unverified: '供应商报价与调整收盘价，口径未独立核验',
  cache_price_basis_unverified: '旧缓存的来源与复权口径未核验',
  caller_provided_unverified: '调用方数据，来源与复权口径未核验',
  spot_trade_prices: '现货成交价',
  configured_feed_unverified: '配置的行情接口，原始提供者与价格口径未核验',
};

export function ExecutionAssumptionsVisual({ assumptions = [], provenance }: {
  assumptions?: ExecutionAssumption[];
  provenance?: MarketProvenance;
}) {
  if (!assumptions.length && !provenance) return null;
  return <details className="ax-execution" aria-label="成交与价格口径">
    <summary>成交与价格口径 <span>查看模拟规则和数据来源</span></summary>
    {provenance && <div className="ax-execution-source">
      <p><strong>{providers[provenance.provider] || provenance.provider}</strong> · {bases[provenance.price_basis] || provenance.price_basis}</p>
      <p>{provenance.corporate_actions === 'not_simulated' ? '分红、拆股等公司行动未另行记账；价格曲线不代表含分红再投资的总回报。' : provenance.corporate_actions === 'dated_split_event_observed' ? '已观察到有日期的拆股事件；持仓与现金尚未按公司行动记账，报价和调整收盘价也不代表已核验总回报。' : provenance.corporate_actions}</p>
      {/^https:\/\//.test(provenance.endpoint) && <a href={provenance.endpoint} target="_blank" rel="noopener noreferrer">行情接口来源 ↗</a>}
    </div>}
    {!!assumptions.length && <ul>{assumptions.map(item => <li key={item.id}>
      <span className={`ax-execution-status ${item.simulated ? 'simulated' : 'omitted'}`}>{item.simulated ? '已模拟' : '未模拟'}</span>
      <div><p>{item.description_zh}</p>{item.limitation_zh && <p className="ax-execution-limit">{item.limitation_zh}</p>}
        {item.source_url && <a href={item.source_url} target="_blank" rel="noopener noreferrer">规则原文 ↗</a>}
      </div>
    </li>)}</ul>}
  </details>;
}
