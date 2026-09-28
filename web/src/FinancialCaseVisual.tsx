import type { PracticeResult } from './types';

type Fact = { section: 'annual_income_statement' | 'balance_sheets' | 'annual_cash_flows'; key: string; label: string; unit?: string };
const income = (key: string, label: string, unit?: string): Fact => ({ section: 'annual_income_statement', key, label, unit });
const balance = (key: string, label: string): Fact => ({ section: 'balance_sheets', key, label });
const cash = (key: string, label: string): Fact => ({ section: 'annual_cash_flows', key, label });
const sales = income('sales', '营业收入'), profit = income('netincome', '净利润'), assets = balance('assets', '总资产'), liabilities = balance('liabilities', '总负债'), equity = balance('equity', '股东权益'), cfo = cash('cfo', '经营现金流'), capex = cash('ppe_capex_cash_outflow', '购置固定资产的现金支出');
const explanations: Record<string, { formula: string; facts: Fact[] }> = {
  accrual_ratio: { formula: '(净利润 − 经营现金流) ÷ 平均总资产', facts: [profit, cfo, assets] },
  book_yoy: { formula: '本案例观察营业收入同比：本年营收 ÷ 上年营收 − 1，不是每股收益增速', facts: [sales] },
  book_ccc: { formula: '应收天数（销售额代理赊销）+ 存货天数 − 应付天数；使用平均余额与财年实际 364 天。负值表示先收款、后付款的周转特征。', facts: [balance('accounts_receivable', '应收账款'), balance('inventory', '存货'), balance('accounts_payable', '应付账款'), sales, income('cogs', '销售成本')] },
  eps: { formula: '以公告披露 EPS 为准；利润与加权股数的复算受披露单位舍入影响', facts: [income('basic_eps', '披露基本每股收益', '美元/股'), income('diluted_eps', '披露稀释每股收益', '美元/股')] },
  book_diluted_shares: { formula: '采用整年加权平均稀释股数，不能用期末股数替代', facts: [income('weighted_basic_shares', '加权基本股数', '千股'), income('weighted_diluted_shares', '加权稀释股数', '千股')] },
  book_revenue: { formula: '营业收入是期间销售额，不是利润', facts: [sales] },
  book_gross_margin: { formula: '毛利润 ÷ 营业收入', facts: [income('grossprofit', '毛利润'), sales] },
  book_ebit_margin: { formula: '营业利润 ÷ 营业收入；本案例明确以营业利润作为 EBIT 口径', facts: [income('operatingincome', '营业利润'), sales] },
  book_net_margin: { formula: '净利润 ÷ 营业收入', facts: [profit, sales] },
  book_current_ratio: { formula: '流动资产 ÷ 流动负债', facts: [balance('current_assets', '流动资产'), balance('current_liabilities', '流动负债')] },
  book_quick_ratio: { formula: '(现金 + 流动有价证券 + 应收账款) ÷ 流动负债；不包括供应商非贸易应收款', facts: [balance('cash', '现金'), balance('current_securities', '流动有价证券'), balance('accounts_receivable', '应收账款'), balance('current_liabilities', '流动负债')] },
  book_cash_ratio: { formula: '现金 ÷ 流动负债；此处采用纯现金口径', facts: [balance('cash', '现金'), balance('current_liabilities', '流动负债')] },
  book_debt_ratio: { formula: '总负债 ÷ 总资产；不是仅计有息债务', facts: [liabilities, assets] },
  book_de_ratio: { formula: '总负债 ÷ 股东权益', facts: [liabilities, equity] },
  book_net_debt: { formula: '商业票据 + 流动借款 + 非流动借款 − 现金', facts: [balance('commercial_paper', '商业票据'), balance('current_term_debt', '流动借款'), balance('noncurrent_term_debt', '非流动借款'), balance('cash', '现金')] },
  book_cfo: { formula: '经营活动现金流量净额：实际现金收支，不是会计净利润', facts: [cfo, profit] },
  book_capex: { formula: '购置固定资产的现金支出，以正数展示支出额', facts: [capex] },
  book_fcf: { formula: '经营现金流 − 购置固定资产的现金支出', facts: [cfo, capex] },
  fcf: { formula: '经营现金流 − 购置固定资产的现金支出', facts: [cfo, capex] },
  book_cfo_income: { formula: '经营现金流 ÷ 净利润', facts: [cfo, profit] },
  book_asset_turnover: { formula: '营业收入 ÷ 平均总资产；平均数使用期初和期末', facts: [sales, assets] },
  book_roa: { formula: '净利润 ÷ 平均总资产；平均数使用期初和期末', facts: [profit, assets] },
  book_inventory_turnover: { formula: '销售成本 ÷ 平均存货；平均数使用期初和期末', facts: [income('cogs', '销售成本'), balance('inventory', '存货')] },
  book_receivable_turnover: { formula: '营业收入 ÷ 平均应收账款；未披露赊销额，因此只是代理口径', facts: [sales, balance('accounts_receivable', '应收账款')] },
  book_dpo: { formula: '平均应付账款 ÷ 销售成本 × 364 天；财年实际为 52 周', facts: [balance('accounts_payable', '应付账款'), income('cogs', '销售成本')] },
  roe: { formula: '净利润 ÷ 平均股东权益；平均数使用期初和期末', facts: [profit, equity] },
  dupont: { formula: '净利率 × 资产周转率 × 权益乘数 = 平均权益收益率', facts: [profit, sales, assets, equity] },
  book_roce: { formula: '营业利润 ÷ (期末总资产 − 期末流动负债)', facts: [income('operatingincome', '营业利润'), assets, balance('current_liabilities', '流动负债')] },
};
const labels: Record<string, string> = { accrual_ratio: '应计比率', revenue_year_over_year: '营业收入同比', cash_conversion_cycle: '现金转换周期', days_sales_outstanding_proxy: '应收天数（销售额代理）', days_inventory_outstanding: '存货天数', reported_basic_eps: '披露基本每股收益', reported_diluted_eps: '披露稀释每股收益', approx_basic_eps_cross_check: '基本 EPS 近似复算', approx_diluted_eps_cross_check: '稀释 EPS 近似复算', weighted_diluted_shares: '加权稀释股数', revenue: '营业收入', gross_margin: '毛利率', ebit_margin: '营业利润率', net_margin: '净利率', current_ratio: '流动比率', quick_ratio: '速动比率', cash_ratio: '现金比率', debt_ratio: '资产负债率', debt_to_equity: '负债权益比', net_debt: '净债务', operating_cash_flow: '经营现金流', capital_expenditure: '资本支出', free_cash_flow: '自由现金流', cfo_to_net_income: '现金利润比', asset_turnover: '资产周转率', return_on_average_assets: '平均资产收益率', inventory_turnover: '存货周转率', receivable_turnover_proxy: '应收账款周转率（代理）', days_payable_outstanding: '应付账款天数', return_on_average_equity: '平均权益收益率', equity_multiplier: '权益乘数', dupont_roe: '杜邦权益收益率', return_on_capital_employed: '资本投入回报率' };
const units: Record<string, string> = { 'USD millions': '百万美元', 'USD/share': '美元/股', 'thousand shares': '千股', multiple: '倍', days: '天', fraction: '%' };
const format = (value: number) => value.toLocaleString('zh-CN', { maximumFractionDigits: 4 });

export function FinancialCaseVisual({ result, name }: { result: PracticeResult; name: string }) {
  const source = result.filing_case, facts = result.facts;
  if (!source || !facts) return null;
  const explanation = explanations[result.concept_id];
  const observations = (explanation?.facts || []).map(fact => {
    const dates = fact.section === 'balance_sheets' ? [source.comparison_period.end, source.period.end] : [String(source.comparison_period.fiscal_year), String(source.period.fiscal_year)];
    return { ...fact, page: source.pages[fact.section === 'annual_income_statement' ? 'income_statement' : fact.section === 'balance_sheets' ? 'balance_sheet' : 'cash_flow'], values: dates.map(date => facts[fact.section][date]?.[fact.key]).map(value => typeof value === 'number' && Number.isFinite(value) ? value : null) };
  });
  const maximum = Math.max(1, ...observations.flatMap(fact => fact.values.filter((v): v is number => v != null).map(Math.abs)));
  return <section className="ax-filing-case" aria-label={`${name} 真实财务案例`}>
    <header><span className="ax-filing-badge">历史财务案例 · 原文已核验</span><h4>{source.issuer} · FY{source.period.fiscal_year}</h4><p>{source.period.start} 至 {source.period.end} · 公告发布 {source.published}</p><p>官方业绩公告，未经审计；不是年度报告，也不是当前行情或当前所选股票的财务数据。</p></header>
    <div className="ax-filing-results">{Object.entries(result.values).map(([key, value]) => <div key={key}><span>{labels[key] || key}</span><strong>{value == null ? '无法计算' : format(result.units?.[key] === 'fraction' ? value * 100 : value)} <small>{units[result.units?.[key] || ''] || result.units?.[key]}</small></strong></div>)}</div>
    <figure><figcaption><strong>{explanation?.formula || name}</strong><span>下面比较两年披露输入；资产负债表为各年期末值。需要平均数的公式使用两个期末值。</span></figcaption>
      <div className="ax-filing-legend"><span>FY{source.comparison_period.fiscal_year}</span><span>FY{source.period.fiscal_year}</span></div>
      {observations.map(fact => <div className="ax-filing-fact" data-filing-fact={fact.key} key={fact.key}><header><strong>{fact.label}</strong><a href={`${source.url}#page=${fact.page}`} target="_blank" rel="noopener noreferrer">第 {fact.page} 页 ↗</a></header>{fact.values.map((value, index) => <div className={`ax-filing-bar year-${index}`} key={index}><span aria-hidden="true" style={{ width: `${value == null ? 0 : Math.abs(value) / maximum * 100}%` }} /><p>{value == null ? '未披露' : `${format(value)} ${fact.unit || '百万美元'}`}</p></div>)}</div>)}
    </figure>
    {result.notes.map((note, index) => <p className="ax-practice-note" key={index}>{note}</p>)}
    <details><summary>核对原文与来源</summary><a href={source.url} target="_blank" rel="noopener noreferrer">Apple 官方财务公告 PDF ↗</a><p>完整文件 {format(source.bytes)} 字节 · SHA-256 <code>{source.sha256}</code></p><p>核验时间 {source.verification.verified_at} · 只在文件字节数与指纹一致时发布计算结果。</p></details>
  </section>;
}
