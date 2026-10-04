import type { PracticeResult } from './types';

const number = (value: number, digits = 0) => value.toLocaleString('zh-CN', { minimumFractionDigits: digits, maximumFractionDigits: digits });
const sourceLabels: Record<string, string> = {
  issuer_annual_report: '发行人年度报告',
  issuer_concert_party_announcement: '发行人一致行动关系公告',
  official_index_methodology: '中证指数编制方案',
  historical_price_api: '历史行情 API',
};

export function FloatCaseVisual({ result, name }: { result: PracticeResult; name: string }) {
  const item = result.float_case;
  if (!item) return null;
  const ratio = result.values.book_free_float;
  const marketCap = result.values.book_float_market_cap;
  return <section className="ax-filing-case ax-industry-case ax-float-case" aria-label={`${name} A股自由流通口径案例`}>
    <header>
      <span className="ax-filing-badge">固定发行人 · 多来源交叉核验</span>
      <h4>{item.issuer.name}（{item.issuer.ticker}）· 截至 {item.as_of}</h4>
      <p>股本登记日 {item.share_register_date} · 年报发布于 {item.published}</p>
      <p>无限售条件流通股份不等于自由流通股。后者按中证定义，剔除达到门槛的战略长期持股及其一致行动人。</p>
    </header>
    <div className="ax-filing-results">
      <div><span>无限售条件流通股份</span><strong>{number(result.values.unrestricted_shares || 0)} <small>股</small></strong></div>
      <div><span>透明复算的自由流通量</span><strong>{number(result.values.free_float_shares || 0)} <small>股</small></strong></div>
      {ratio != null && <div><span>自由流通比例</span><strong>{number(ratio * 100, 4)}<small>%</small></strong></div>}
      {marketCap != null && <div><span>同日流通市值</span><strong>{number(marketCap, 2)} <small>元</small></strong></div>}
    </div>
    <figure className="ax-industry-formula">
      <figcaption><strong>自由流通量 = 无限售条件流通股份 − 非自由流通持股</strong><span>按公开股东事实逐项应用中证口径；不冒充中证公司发布的证券级自由流通量或指数调整股本。</span></figcaption>
      <div className="ax-industry-formula-flow" aria-label="自由流通量计算关系">
        <span>{number(result.values.unrestricted_shares || 0)} 股无限售条件流通股份</span>
        {item.non_free_float_holders.map(holder => <span key={holder.name}>减 {holder.name} {number(holder.shares)} 股{holder.relationship === 'wholly_owned_subsidiary_and_concert_party' ? '（控股股东全资子公司及一致行动人）' : ''}</span>)}
        <strong>得到 {number(result.values.free_float_shares || 0)} 股自由流通量</strong>
      </div>
    </figure>
    <section className="ax-industry-unit-group" data-unit-group="price">
      <h5>同日价格与流通市值</h5>
      <div className="ax-filing-fact"><header><strong>{item.price.trading_date} 未复权日收盘价</strong><a href={item.price.endpoint} target="_blank" rel="noopener noreferrer">查看行情 ↗</a></header><p>{number(item.price.close, 2)} 元/股</p></div>
      <p className="ax-industry-provenance">股数日期与价格交易日一致：{item.share_register_date}。流通市值使用无限售条件流通股份，不把自由流通量换作分母。</p>
    </section>
    <details className="ax-industry-definitions"><summary>定义与边界</summary><p>{item.definition.provider}：{item.definition.rule}；门槛为持股达到 5%，并合并一致行动人。</p><p>本案例是依据公开规则与具名持股事实的透明复算，不冒充中证公司发布的证券级自由流通量。</p></details>
    <details className="ax-filing-sources"><summary>核对原始资料</summary><div className="ax-filing-source-grid">{item.sources.map((source, index) => <div className="ax-industry-provenance" key={`${source.kind}-${index}`}><a href={source.official_url} target="_blank" rel="noopener noreferrer">{sourceLabels[source.kind] || source.kind} ↗</a>{source.pdf_pages && <p>第 {source.pdf_pages.join('、')} 页</p>}{source.verification && <p>{source.verification.matched_sha256 === source.sha256 && source.verification.matched_bytes === source.bytes ? '原文已核对' : '原文核对失败'}</p>}{source.sha256 && <details><summary>文件指纹</summary><code>{source.sha256}</code></details>}</div>)}</div></details>
    {result.notes.map((note, index) => <p className="ax-practice-note" key={index}>{note}</p>)}
  </section>;
}
