import type { PracticeResult } from './types';

const resultLabels: Record<string, string> = {
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
};

const unitLabels: Record<string, string> = {
  fraction: '%',
  'CNY millions': '百万人民币',
  'USD millions': '百万美元',
  'USD millions annualized': '百万美元（年化运行率）',
  'square feet': '平方英尺',
  count: '个',
};

const auditBoundaries: Record<string, string> = {
  "Management discussion facts on PDF pages 57-58 are outside EY's audit opinion covering the financial statements": '相关事实位于管理层讨论 PDF 第 57–58 页，不在安永对财务报表出具的审计意见范围内。',
  'Issuer earnings release metric; no audit assurance is asserted for MRR, GMV or non-GAAP free cash flow': '指标来自发行人业绩公告；未声称 MRR、GMV 或非 GAAP 自由现金流获得审计保证。',
  "Note 49(7), PDF page 336, is inside the audited financial statements covered by EY's opinion": '附注 49(7)（PDF 第 336 页）属于安永审计意见覆盖的财务报表范围。',
  'The issuer labels the supplemental operating information unaudited': '发行人明确标注该补充经营信息未经审计。',
};

const factLabels: Record<string, string> = {
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
  net_interest_income: '净利息收入', average_earning_assets: '平均生息资产',
  operating_expenses: '经营费用', operating_income: '营业收入',
  nonperforming_loans: '不良贷款', gross_loans: '贷款总额（不含利息）',
  available_capital: '实际资本', required_capital: '最低资本',
  monthly_recurring_revenue: '期末 MRR', revenue_growth_rate: '2024 营收 ÷ 2023 营收 − 1',
  fcf_margin: '自由现金流 ÷ 2024 营收', gross_merchandise_value: 'GMV',
  platform_revenue: '2024 年平台净收入', leased_area: '已出租面积', lettable_area: '可出租总面积',
};

const verificationLabels: Record<string, string> = {
  verified_immutable_cache: '已核验固定缓存', verified_then_cached: '已核验官方原文并缓存',
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

const format = (value: number, unit?: string) => {
  const adjusted = unit === 'fraction' ? value * 100 : value;
  return adjusted.toLocaleString('zh-CN', { maximumFractionDigits: 4 });
};

export function IndustryCaseVisual({ result, name }: { result: PracticeResult; name: string }) {
  const source = result.industry_case;
  const facts = result.industry_facts;
  if (!source || !facts) return null;

  const groups = facts.reported_facts.reduce<Array<{ unit: string; facts: typeof facts.reported_facts }>>((all, fact) => {
    const group = all.find(item => item.unit === fact.unit);
    if (group) group.facts.push(fact);
    else all.push({ unit: fact.unit, facts: [fact] });
    return all;
  }, []);
  const calculated = result.values[facts.calculation.result_key];
  const resultProvenance = facts.field_provenance[facts.calculation.result_key];
  const auditBoundary = auditBoundaries[source.audit_boundary] || source.audit_boundary;
  const fingerprintMatches = source.sha256 === source.verification.matched_sha256 && source.bytes === source.verification.matched_bytes;

  return <section className="ax-filing-case ax-industry-case" aria-label={`${name} 历史行业披露案例`}>
    <header>
      <span className="ax-filing-badge">历史行业披露 · 固定发行人原文</span>
      <h4>{entityName(source.issuer.name)}（{source.issuer.ticker}）· {source.period.label.replace(/^As of /, '截至 ')}</h4>
      <p>{source.period.start === source.period.end ? source.period.end : `${source.period.start} 至 ${source.period.end}`} · 发布于 {source.published}</p>
      <p>报告主体：{entityName(source.issuer.reporting_entity)} · 指标主体：{entityName(source.issuer.metric_entity)}</p>
      <p>{auditBoundary} <strong>{source.audited ? '审计范围内' : '未经审计'}</strong>。这是固定历史案例，不是当前行情或当前所选股票的指标。</p>
    </header>

    <div className="ax-filing-results">
      <div><span>{resultLabels[result.concept_id] || name}</span><strong>{calculated == null ? '无法计算' : format(calculated, result.units?.[facts.calculation.result_key])} <small>{unitLabels[result.units?.[facts.calculation.result_key] || ''] || result.units?.[facts.calculation.result_key]}</small></strong>{resultProvenance && <p className="ax-industry-provenance">披露值推导 · 仅由下列原文披露值按所示公式计算</p>}</div>
    </div>

    <figure className="ax-industry-formula">
      <figcaption><strong>{facts.calculation.formula}</strong><span>计算只使用下列披露事实；不同单位分组展示，各组独立缩放。</span></figcaption>
      <div className="ax-industry-formula-flow" aria-label="公式关系">
        {facts.calculation.operands.map(key => <span key={key}>{factLabels[key] || key}</span>)}
        <strong>以上披露值代入公式，得到 {resultLabels[result.concept_id] || name}</strong>
      </div>
      <p className="ax-industry-symbols">{Object.keys(facts.calculation.symbol_mapping).map(symbol => symbolMeanings[symbol] || factLabels[symbol] || symbol).join('；')}</p>
    </figure>

    {groups.map(group => {
      const maximum = Math.max(1, ...group.facts.map(fact => Math.abs(fact.value)));
      return <section className="ax-industry-unit-group" data-unit-group={group.unit} key={group.unit}>
        <h5>{unitLabels[group.unit] || group.unit}</h5>
        {group.facts.map(fact => {
          const provenance = facts.field_provenance[fact.key];
          return <div className="ax-filing-fact" data-industry-fact={fact.key} key={fact.key}>
            <header><strong>{factLabels[fact.key] || fact.label}</strong><a href={`${source.url}#page=${fact.pdf_page}`} target="_blank" rel="noopener noreferrer">PDF 第 {fact.pdf_page} 页 ↗</a></header>
            <div className="ax-filing-bar year-1"><span aria-hidden="true" style={{ width: `${Math.abs(fact.value) / maximum * 100}%` }} /><p>{format(fact.value, fact.unit)} {unitLabels[fact.unit] || fact.unit}</p></div>
            {provenance && <p className="ax-industry-provenance">{provenance.kind === 'reported' ? `原文披露 · PDF 第 ${fact.pdf_page} 页` : '披露值推导 · 仅由已列出的原文数值计算'}</p>}
          </div>;
        })}
      </section>;
    })}

    <details className="ax-industry-definitions"><summary>口径定义</summary><p><strong>案例边界</strong>：固定发行人与报告期；不会根据调用方当前选择的标的或数据集外推。</p><p><strong>指标定义</strong>：{facts.definitions.metric}</p></details>
    {result.notes.map((note, index) => <p className="ax-practice-note" key={index}>{note}</p>)}
    <details><summary>核对原文与文件指纹</summary><a href={source.url} target="_blank" rel="noopener noreferrer">{source.issuer.name} 官方披露 PDF ↗</a><p>原文页：{source.pdf_pages.join('、')} · 内嵌审阅文件 {format(source.bytes)} 字节</p><p>内嵌 SHA-256 <code>{source.sha256}</code></p><a href={source.verification.requested_url} target="_blank" rel="noopener noreferrer">实际核验地址 ↗</a><p>实际匹配文件 {format(source.verification.matched_bytes)} 字节 · SHA-256 <code>{source.verification.matched_sha256}</code></p><p>{verificationLabels[source.verification.status] || source.verification.status} · {fingerprintMatches ? '字节数与指纹完全匹配' : '字节数或指纹不匹配'} · 核验时间 {source.verification.verified_at}</p></details>
  </section>;
}
