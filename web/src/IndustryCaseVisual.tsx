import type { PracticeResult } from './types';

const resultLabels: Record<string, string> = {
  book_share_counts: '限售股份余额',
  bank_nim: '银行净息差',
  book_bank_nim: '银行净息差',
  book_bank_cost_income: '成本收入比',
  book_bank_npl_ratio: '不良贷款率',
  book_insurance_solvency_ratio: '偿付能力充足率',
  book_saas_arr: '年化经常性收入运行率',
  book_saas_rule_of_40: 'Rule of 40',
  book_platform_gmv: '平台成交总额',
  book_platform_take_rate: '平台变现率',
  book_reit_occupancy: '出租率',
  short_interest: '回补天数（全证券汇总空仓）',
};

const unitLabels: Record<string, string> = {
  fraction: '%',
  'CNY millions': '百万人民币',
  'USD millions': '百万美元',
  'USD millions annualized': '百万美元（年化运行率）',
  'square feet': '平方英尺',
  count: '个',
  shares: '股',
  'million shares': '百万股',
  analysts: '位分析师',
  'USD/share': '美元/股',
  'USD/share split-adjusted': '美元/股（按拆股调整）',
  'EUR/share': '欧元/股',
  'USD thousands': '千美元',
  'EUR millions': '百万欧元',
  'EUR million': '百万欧元',
  'EUR billions': '十亿欧元',
  'billions of people': '十亿人',
  '8-inch-equivalent wafers': '片（8 英寸等效晶圆）',
  'US cents per ASM': '美分/可用座位英里',
  'millions of available seat miles': '百万可用座位英里',
  'millions of passenger miles': '百万旅客英里',
  'USD per boe': '美元/桶油当量',
  'USD per ounce': '美元/盎司',
  'USD per day': '美元/天',
  'USD per person per quarter': '美元/人/季度',
  'USD per connection per month': '美元/连接/月',
  'EUR per Premium user per month': '欧元/付费用户/月',
  'fraction per month': '%/月',
  'months lower bound': '个月（区间下限）',
  'months upper bound': '个月（区间上限）',
  'index 1982-84=100': '指数（1982–1984 年=100）',
  years: '年',
  months: '个月',
  multiple: '倍',
  ratio: '倍',
  days: '天',
  'USD per 8-inch-equivalent wafer': '美元/8 英寸等效晶圆',
  'multiple per percentage point': '倍/增长百分点',
  'fraction of pre-transaction holdings': '%（占交易前持股）',
  'fraction of issued H shares': '%（占已发行 H 股）',
  '2025 USD/share': '2025 年购买力美元/股',
  score: '分',
  '0-9 score': '分（0–9）',
  factor: '因子',
};

const secondaryLabels: Record<string, string> = {
  cac_payback_upper_bound: '获客回本期上限',
  aggregate_short_shares: '全证券汇总空仓股数',
  reported_days_to_cover: '原文回补天数（四舍五入）',
  analyst_count: '参与预测的分析师人数',
};

const auditBoundaries: Record<string, string> = {
  "Management discussion facts on PDF pages 57-58 are outside EY's audit opinion covering the financial statements": '相关事实位于管理层讨论 PDF 第 57–58 页，不在安永对财务报表出具的审计意见范围内。',
  'Issuer earnings release metric; no audit assurance is asserted for MRR, GMV or non-GAAP free cash flow': '指标来自发行人业绩公告；未声称 MRR、GMV 或非 GAAP 自由现金流获得审计保证。',
  "Note 49(7), PDF page 336, is inside the audited financial statements covered by EY's opinion": '附注 49(7)（PDF 第 336 页）属于安永审计意见覆盖的财务报表范围。',
  'The issuer labels the supplemental operating information unaudited': '发行人明确标注该补充经营信息未经审计。',
};

const factLabels: Record<string, string> = {
  opening_total_shares: '2025年期初股份总数', cancelled_shares: '年内回购注销股份（减少数量）',
  closing_total_shares: '2025年末股份总数', unrestricted_shares: '2025年末无限售条件流通股份',
  net_interest_income: '净利息收入', average_earning_assets: '平均生息资产',
  operating_expenses: '一般及行政费用（绝对金额）', operating_income: '营业收入',
  nonperforming_loans: '不良贷款', gross_loans: '客户贷款及垫款总额（不含利息）',
  available_capital: '实际资本', required_capital: '最低资本',
  monthly_recurring_revenue: '2024 年末月度经常性收入（MRR）',
  revenue_2024: '2024 年营业收入', revenue_2023: '2023 年营业收入',
  free_cash_flow: '2024 年非 GAAP 自由现金流', gross_merchandise_value: '2024 年平台成交总额（GMV）',
  platform_revenue: '2024 年平台净收入',
  leased_area: '已出租面积', lettable_area: '组合可出租总面积',
};

const symbolMeanings: Record<string, string> = {
  monthly_cash_burn: '经营现金消耗 ÷ 12（月均）',
  wafer_revenue: '总营收 × 晶圆营收占比',
  restricted_shares_residual: '限售股份余额 = 期末总股本 − 期末无限售条件流通股份',
  closing_total_shares: '期末总股本 = 期初总股本 − 注销股份',
  net_interest_income: '净利息收入', average_earning_assets: '平均生息资产',
  operating_expenses: '经营费用', operating_income: '营业收入',
  nonperforming_loans: '不良贷款', gross_loans: '贷款总额（不含利息）',
  available_capital: '实际资本', required_capital: '最低资本',
  monthly_recurring_revenue: '期末 MRR', revenue_growth_rate: '2024 营收 ÷ 2023 营收 − 1',
  fcf_margin: '自由现金流 ÷ 2024 营收', gross_merchandise_value: 'GMV',
  platform_revenue: '2024 年平台净收入', leased_area: '已出租面积', lettable_area: '可出租总面积',
};

const factName = (key: string, label?: string) => label && /[\u3400-\u9fff]/.test(label) ? label : factLabels[key] || label || key;

const verificationLabels: Record<string, string> = {
  verified_immutable_cache: '已核验固定缓存', verified_then_cached: '已核验官方原文并缓存',
  verified_archived_original: '已核验原文备份',
};

const entityLabels: Record<string, string> = {
  'Ping An Insurance (Group) Company of China, Ltd.': '中国平安保险（集团）股份有限公司',
  'Ping An Insurance (Group) Company of China, Ltd. and its subsidiaries': '中国平安保险集团及其子公司',
  'Ping An Bank': '平安银行',
  'Ping An Property & Casualty Insurance Company of China, Ltd.': '中国平安财产保险股份有限公司',
  'Shopify Inc.': 'Shopify 公司',
  'Shopify Inc. and its consolidated subsidiaries': 'Shopify 公司及其合并子公司',
  'Shopify platform': 'Shopify 平台',
  'Realty Income Corporation': 'Realty Income 公司',
  'Realty Income Corporation and consolidated subsidiaries': 'Realty Income 公司及其合并子公司',
  'Realty Income Corporation consolidated portfolio': 'Realty Income 合并物业组合',
  'eBay Inc.': 'eBay 公司',
};

const entityName = (name: string) => entityLabels[name] || name;
const publicationLabel = (value: string) => value.startsWith('undated PDF; collection window ended ')
  ? `发布日期未注明；预测收集截至 ${value.slice('undated PDF; collection window ended '.length)}`
  : `发布于 ${value}`;

const format = (value: number, unit?: string) => {
  const adjusted = unit === 'fraction' || unit?.startsWith('fraction ') ? value * 100 : value;
  return adjusted.toLocaleString('zh-CN', { maximumFractionDigits: 4 });
};

export function IndustryCaseVisual({ result, name }: { result: PracticeResult; name: string }) {
  const source = result.industry_case;
  const facts = result.industry_facts;
  if (!source || !facts) return null;
  const disclosureLabel = source.label || (result.concept_id === 'book_share_counts' ? '股本披露' : '历史行业披露');
  const explicitSourceCategory = ['公司股东披露', '预测汇总', '预测汇总与公司业绩原文', '分析师报告', '市场空仓报告'].includes(disclosureLabel);
  const sourceBadge = disclosureLabel === '财报与估值'
    ? `${disclosureLabel} · 固定历史来源`
    : explicitSourceCategory ? `固定${disclosureLabel}` : `${disclosureLabel} · 固定发行人原文`;
  const sourceLinkLabel = explicitSourceCategory ? `查看${disclosureLabel}` : `${source.issuer.name} 官方披露 ${source.format || 'PDF'}`;
  const usesAssumptions = facts.reported_facts.some(fact => fact.kind === 'assumption');

  const groups = facts.reported_facts.reduce<Array<{ unit: string; facts: typeof facts.reported_facts }>>((all, fact) => {
    const group = all.find(item => item.unit === fact.unit);
    if (group) group.facts.push(fact);
    else all.push({ unit: fact.unit, facts: [fact] });
    return all;
  }, []);
  const calculated = result.values[facts.calculation.result_key];
  const resultProvenance = facts.field_provenance[facts.calculation.result_key];
  const secondary = Object.entries(result.values).flatMap(([key, value]) => {
    if (key === facts.calculation.result_key || typeof value !== 'number' || !Number.isFinite(value)) return [];
    const metric = facts.derived_metrics?.find(item => item.key === key);
    return [{ key, value, label: metric?.label || secondaryLabels[key] || factName(key, facts.reported_facts.find(fact => fact.key === key)?.label), description: metric?.description }];
  });
  const auditBoundary = auditBoundaries[source.audit_boundary] || source.audit_boundary;
  const fingerprintMatches = source.sha256 === source.verification.matched_sha256 && source.bytes === source.verification.matched_bytes;

  return <section className="ax-filing-case ax-industry-case" aria-label={`${name} ${disclosureLabel}案例`}>
    <header>
      <span className="ax-filing-badge">{sourceBadge}</span>
      <h4>{entityName(source.issuer.name)}（{source.issuer.ticker}）· {source.period.label.replace(/^As of /, '截至 ')}</h4>
      <p>{source.period.start === source.period.end ? source.period.end : `${source.period.start} 至 ${source.period.end}`} · {publicationLabel(source.published)}</p>
      <p>报告主体：{entityName(source.issuer.reporting_entity)} · 指标主体：{entityName(source.issuer.metric_entity)}</p>
      <p>{auditBoundary} <strong>{source.audited ? '审计范围内' : result.concept_id === 'book_share_counts' ? '未声明审计保证' : '未经审计'}</strong>。这是固定历史案例，不是当前行情或当前所选股票的指标。</p>
    </header>

    <div className="ax-filing-results">
      <div><span>{resultLabels[result.concept_id] || name}</span><strong>{calculated == null ? '无法计算' : format(calculated, result.units?.[facts.calculation.result_key])} <small>{unitLabels[result.units?.[facts.calculation.result_key] || ''] || result.units?.[facts.calculation.result_key]}</small></strong>{resultProvenance && <p className="ax-industry-provenance">{usesAssumptions ? '模型估算 · 历史披露为起点，预测假设单独列示' : '披露值推导 · 仅由下列原文披露值按所示公式计算'}</p>}</div>
    </div>

    {secondary.length > 0 && <details className="ax-industry-derived"><summary>计算拆解（{secondary.length} 项）</summary><div className="ax-filing-results">{secondary.map(metric => <div key={metric.key} data-derived-metric={metric.key}><span>{metric.label}</span><strong>{result.units?.[metric.key] === 'binary' ? metric.value === 1 ? '满足（1 分）' : '未满足（0 分）' : <>{format(metric.value, result.units?.[metric.key])} <small>{unitLabels[result.units?.[metric.key] || ''] || result.units?.[metric.key]}</small></>}</strong>{metric.description && <p className="ax-industry-provenance">{metric.description}</p>}</div>)}</div></details>}

    <figure className="ax-industry-formula">
      <figcaption><strong>{result.concept_id === 'book_share_counts' ? facts.calculation.formula.split('；').map(line => <span key={line} style={{ display: 'block', marginBottom: '0.5rem' }}>{line}</span>) : facts.calculation.formula}</strong><span>{usesAssumptions ? '历史事实与模型假设分别标注；估算结果随假设变化。' : '计算只使用下列披露事实；'}不同单位分组展示，各组独立缩放。</span></figcaption>
      <div className="ax-industry-formula-flow" aria-label="公式关系">
        {facts.calculation.operands.map(key => <span key={key}>{factName(key, facts.reported_facts.find(fact => fact.key === key)?.label)}</span>)}
        <strong>{usesAssumptions ? '以上输入' : '以上披露值'}代入公式，得到 {resultLabels[result.concept_id] || name}</strong>
      </div>
      <p className="ax-industry-symbols">{Object.keys(facts.calculation.symbol_mapping).map(symbol => {
        const mappedFactKey = facts.calculation.symbol_mapping[symbol];
        const fact = facts.reported_facts.find(item => item.key === symbol)
          || facts.reported_facts.find(item => item.key === mappedFactKey);
        return fact ? factName(fact.key, fact.label) : symbolMeanings[symbol] || factLabels[symbol] || symbol;
      }).join('；')}</p>
    </figure>

    {groups.map(group => {
      const maximum = Math.max(1, ...group.facts.map(fact => Math.abs(fact.value)));
      return <section className="ax-industry-unit-group" data-unit-group={group.unit} key={group.unit}>
        <h5>{unitLabels[group.unit] || group.unit}</h5>
        {group.facts.map(fact => {
          const provenance = facts.field_provenance[fact.key];
          return <div className="ax-filing-fact" data-industry-fact={fact.key} key={fact.key}>
            <header><strong>{factName(fact.key, fact.label)}</strong>{fact.kind === 'assumption' ? <span>模型假设</span> : <a href={`${fact.source_url || source.url}${(fact.pdf_page ?? 0) > 0 ? `#page=${fact.pdf_page}` : ''}`} target="_blank" rel="noopener noreferrer">{(fact.pdf_page ?? 0) > 0 ? `PDF 第 ${fact.pdf_page} 页` : fact.source_section || `${fact.source_format || '原文'} 来源`} ↗</a>}</header>
            {(fact.published || fact.retrieved_on || fact.as_of) && <p className="ax-industry-provenance">{[
              fact.as_of && `数据时点 ${fact.as_of}`,
              fact.published && publicationLabel(fact.published),
              fact.retrieved_on && `资料获取于 ${fact.retrieved_on}`,
            ].filter(Boolean).join(' · ')}</p>}
            <div className="ax-filing-bar year-1"><span aria-hidden="true" style={{ width: `${Math.abs(fact.value) / maximum * 100}%` }} /><p>{format(fact.value, fact.unit)} {unitLabels[fact.unit] || fact.unit}</p></div>
            {provenance && fact.kind !== 'assumption' && <p className="ax-industry-provenance">{provenance.kind === 'reported' ? `原文披露${(fact.pdf_page ?? 0) > 0 ? ` · PDF 第 ${fact.pdf_page} 页` : ` · ${fact.source_section || fact.source_format || '原始记录'}`}` : '披露值推导 · 仅由已列出的原文数值计算'}</p>}
          </div>;
        })}
      </section>;
    })}

    <details className="ax-industry-definitions"><summary>口径定义</summary><p><strong>案例边界</strong>：固定发行人与报告期；不会根据调用方当前选择的标的或数据集外推。</p><p><strong>指标定义</strong>：{facts.definitions.metric}</p></details>
    {result.notes.map((note, index) => <p className="ax-practice-note" key={index}>{note}</p>)}
    <details><summary>核对原文与文件指纹</summary><a href={source.url} target="_blank" rel="noopener noreferrer">{sourceLinkLabel} ↗</a><p>{source.pdf_pages.length > 0 && `原文页：${source.pdf_pages.join('、')} · `}内嵌审阅文件 {format(source.bytes)} 字节</p><p>内嵌 SHA-256 <code>{source.sha256}</code></p><a href={source.verification.requested_url} target="_blank" rel="noopener noreferrer">实际核验地址 ↗</a><p>实际匹配文件 {format(source.verification.matched_bytes)} 字节 · SHA-256 <code>{source.verification.matched_sha256}</code></p><p>{verificationLabels[source.verification.status] || source.verification.status} · {fingerprintMatches ? '字节数与指纹完全匹配' : '字节数或指纹不匹配'} · 核验时间 {source.verification.verified_at}</p>{source.verification.retrieval_note && <p className="ax-industry-provenance">{source.verification.retrieval_note}</p>}</details>
    {source.sources?.map(document => <details key={document.id}><summary>来源：{document.title || document.id} · {document.format}</summary><a href={document.url} target="_blank" rel="noopener noreferrer">查看原文 ↗</a><p>{[document.published && publicationLabel(document.published), document.retrieved_on && `资料获取于 ${document.retrieved_on}`, `${document.verification_basis?.startsWith('canonical') ? '规范化记录 ' : ''}${format(document.bytes)} 字节`].filter(Boolean).join(' · ')}</p>{document.verification_basis?.startsWith('canonical') && <p>指定日期与标的的记录指纹；接口中其他日期和元数据不参与此核验。</p>}<p>SHA-256 <code>{document.sha256}</code></p><a href={document.verification.requested_url} target="_blank" rel="noopener noreferrer">实际核验地址 ↗</a><p>{verificationLabels[document.verification.status] || document.verification.status} · {document.sha256 === document.verification.matched_sha256 && document.bytes === document.verification.matched_bytes ? '字节数与指纹完全匹配' : '字节数或指纹不匹配'} · 核验时间 {document.verification.verified_at}</p>{document.verification.retrieval_note && <p className="ax-industry-provenance">{document.verification.retrieval_note}</p>}</details>)}
  </section>;
}
