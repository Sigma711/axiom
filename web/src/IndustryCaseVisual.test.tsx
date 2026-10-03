import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { IndustryCaseVisual } from './IndustryCaseVisual';
import type { PracticeResult } from './types';

const fixture = (): PracticeResult => ({
  concept_id: 'book_saas_rule_of_40',
  status: 'computed',
  reason: null,
  input_kind: 'industry_case',
  provenance: 'verified_original_issuer_disclosure',
  values: { book_saas_rule_of_40: 0.437632710614297 },
  units: { book_saas_rule_of_40: 'fraction' },
  series: [],
  notes: ['精确值来自披露金额复算；公告中的四舍五入口径为 44%。'],
  module: 'data',
  source: 'issuer_disclosure',
  symbol: 'SHOP',
  bars: [],
  context: 'historical_industry_disclosure',
  bar_origin: 'server_verified_issuer_filing_pdf',
  industry_case: {
    case_id: 'shopify_fy2024',
    issuer: { name: 'Shopify Inc.', ticker: 'SHOP', alternate_tickers: [], reporting_entity: 'Shopify Inc.', metric_entity: 'Shopify platform' },
    period: { label: 'FY2024', start: '2024-01-01', end: '2024-12-31' },
    published: '2025-02-11',
    audited: false,
    audit_boundary: '业绩公告及其中非 GAAP 指标未经审计。',
    status: 'published',
    source_kind: 'issuer_pdf',
    url: 'https://www.shopify.com/fy2024.pdf',
    pdf_pages: [1, 7, 8],
    sha256: 'b'.repeat(64),
    bytes: 86468,
    verification: { status: 'verified_immutable_cache', verified_at: '2026-09-28T00:00:00Z', requested_url: 'https://www.shopify.com/fy2024.pdf', matched_sha256: 'b'.repeat(64), matched_bytes: 86468 },
  },
  industry_facts: {
    currency: 'USD',
    scale: 'millions',
    reported_facts: [
      { key: 'revenue_2024', label: '2024 营业收入', value: 8880, unit: 'USD millions', pdf_page: 1 },
      { key: 'revenue_2023', label: '2023 营业收入', value: 7060, unit: 'USD millions', pdf_page: 1 },
      { key: 'free_cash_flow', label: '自由现金流', value: 1597, unit: 'USD millions', pdf_page: 7 },
    ],
    calculation: { formula: '(8,880 ÷ 7,060 − 1) + (1,597 ÷ 8,880)', result_key: 'book_saas_rule_of_40', operands: ['revenue_2024', 'revenue_2023', 'free_cash_flow'], symbol_mapping: { revenue_2024: '2024 营业收入', revenue_2023: '2023 营业收入', free_cash_flow: '自由现金流' } },
    field_provenance: {
      revenue_2024: { kind: 'reported', pdf_page: 1, note: 'Selected Business Performance Information' },
      revenue_2023: { kind: 'reported', pdf_page: 1, note: 'Selected Business Performance Information' },
      free_cash_flow: { kind: 'reported', pdf_page: 7, note: '经营现金流减资本开支' },
      book_saas_rule_of_40: { kind: 'derived', note: '只由列示的三项披露值计算' },
    },
    definitions: { case_boundary: 'Shopify FY2024 固定历史公告。', metric: '收入增长率与自由现金流利润率之和；自由现金流为经营活动现金流减资本开支。' },
  },
});

describe('IndustryCaseVisual', () => {
  it('explains the computed factors without recomputing them or hiding failed binary signals', () => {
    const result = fixture();
    result.values = { score: 7, dsri: 1.14, no_issuance: 0, missing: null };
    result.units = { score: '0-9 score', dsri: 'factor', no_issuance: 'binary' };
    result.industry_facts!.calculation.result_key = 'score';
    result.industry_facts!.derived_metrics = [
      { key: 'dsri', label: '应收指数（DSRI）', description: '本期应收占收入的比重，除以上期同一比重。' },
      { key: 'no_issuance', label: '没有发行普通股', description: '回购造成的股数下降不能抵消当年实际发行。' },
      { key: 'missing', label: '无数据', description: '不得画成零。' },
    ];
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="财务评分" />);
    expect(html).toContain('计算拆解（2 项）');
    expect(html).toContain('应收指数（DSRI）');
    expect(html).toContain('1.14');
    expect(html).toContain('未满足（0 分）');
    expect(html).toContain('回购造成的股数下降不能抵消当年实际发行');
    expect(html).not.toContain('无数据');
  });
  it('keeps the issuer-specific EBIT label instead of borrowing a bank revenue label for the same key', () => {
    const result = fixture();
    result.concept_id = 'book_ev_ebit';
    result.industry_facts!.reported_facts = [{ key: 'operating_income', label: '2025 财年营业利润（EBIT）', value: 133050, unit: 'USD millions', pdf_page: 32 }];
    result.industry_facts!.calculation.operands = ['operating_income'];
    result.industry_facts!.calculation.symbol_mapping = { operating_income: '2025 财年营业利润（EBIT）' };
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="EV/EBIT" />);
    expect(html).toContain('2025 财年营业利润（EBIT）');
    expect(html).not.toContain('营业收入');
  });
  it('links each fact to its own source and separates disclosed facts from valuation assumptions', () => {
    const result = fixture();
    result.industry_facts!.reported_facts = [
      { key: 'cashflow', label: '历史自由现金流', value: 1597, unit: 'USD millions', pdf_page: 7, source_url: 'https://issuer.example/annual.pdf', published: '2025-02-11' },
      { key: 'growth', label: '预测增长率', value: 0.04, unit: 'fraction', pdf_page: 0, kind: 'assumption' },
      { key: 'price', label: '估值日收盘价', value: 100, unit: 'USD/share', pdf_page: 0, source_url: 'https://exchange.example/prices.csv', source_format: 'CSV', as_of: '2025-02-12' },
    ];
    result.industry_facts!.calculation.operands = ['cashflow', 'growth', 'price'];
    result.industry_facts!.field_provenance.price = { kind: 'reported', note: '历史收盘价' };
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="现金流估值" />);
    expect(html).toContain('https://issuer.example/annual.pdf#page=7');
    expect(html).toContain('https://exchange.example/prices.csv');
    expect(html).not.toContain('prices.csv#page=0');
    expect(html).not.toContain('PDF 第 0 页');
    expect(html).toContain('模型假设');
    expect(html).toContain('历史自由现金流');
    expect(html).toContain('估值日收盘价');
    expect(html).toContain('2025-02-12');
    expect(html).toContain('2025-02-11');
    expect(html).toContain('美元/股');
    expect(html).not.toContain('PDF 第 0 页');
  });
  it('shows every verified original document separately rather than implying one fingerprint covers them all', () => {
    const result = fixture();
    result.industry_case!.sources = [{
      id: 'market-price', url: 'https://exchange.example/prices.csv', format: 'CSV', published: '2025-02-12', bytes: 80, sha256: 'c'.repeat(64),
      verification: { status: 'verified_then_cached', verified_at: '2026-10-01T00:00:00Z', requested_url: 'https://exchange.example/prices.csv', matched_bytes: 80, matched_sha256: 'c'.repeat(64) },
    }];
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="估值" />);
    expect(html).toContain('https://exchange.example/prices.csv');
    expect(html).toContain('cccccccccccccccc');
    expect(html).toContain('CSV');
    expect(html).toContain('2025-02-12');
  });
  it('identifies a canonical observation hash without presenting it as an original-file fingerprint', () => {
    const result = fixture();
    result.industry_case!.sources = [{ id: 'historical-price', title: '历史收盘记录', url: 'https://exchange.example/prices', format: 'JSON', published: '2025-02-12', bytes: 80, sha256: 'c'.repeat(64), verification_basis: 'canonical_observations', verification: { status: 'verified_then_cached', verified_at: '2026-10-03T00:00:00Z', requested_url: 'https://exchange.example/prices', matched_bytes: 80, matched_sha256: 'c'.repeat(64) } }];
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="估值" />);
    expect(html).toContain('指定日期与标的的记录指纹');
    expect(html).toContain('规范化记录 80 字节');
  });
  it('describes a shareholder HTML source without inventing PDF pages or an industry classification', () => {
    const result = fixture();
    result.industry_case!.label = '股东披露';
    result.industry_case!.format = 'HTML';
    result.industry_case!.pdf_pages = [];
    result.industry_facts!.reported_facts = [{ key: 'ownership', label: '持股数', value: 100, unit: 'shares', pdf_page: 0, source_section: '主要股东表' }];
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="持股比例" />);
    expect(html).toContain('股东披露');
    expect(html).toContain('官方披露 HTML');
    expect(html).toContain('主要股东表');
    expect(html).not.toContain('官方披露 PDF');
    expect(html).not.toContain('原文页：');
    expect(html).not.toContain('历史行业披露');
  });
  it.each([
    ['公司股东披露', '固定公司股东披露', '查看公司股东披露'],
    ['预测汇总', '固定预测汇总', '查看预测汇总'],
    ['分析师报告', '固定分析师报告', '查看分析师报告'],
    ['市场空仓报告', '固定市场空仓报告', '查看市场空仓报告'],
  ])('renders the real source category %s without claiming every document is an issuer original', (label, badge, link) => {
    const result = fixture();
    result.industry_case!.label = label;
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="来源边界" />);
    expect(html).toContain(badge);
    expect(html).toContain(link);
    expect(html).not.toContain('固定发行人原文');
    expect(html).not.toContain('Shopify Inc. 官方披露');
  });

  it('labels mixed filing, market, macro, and assumption inputs as fixed historical sources', () => {
    const result = fixture();
    result.industry_case!.label = '财报与估值';
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="现金流估值" />);
    expect(html).toContain('财报与估值 · 固定历史来源');
    expect(html).not.toContain('财报与估值 · 固定发行人原文');
  });

  it('discloses when an exact fingerprint-matched archived original replaced an unavailable source response', () => {
    const result = fixture();
    result.industry_case!.verification.status = 'verified_archived_original';
    result.industry_case!.verification.retrieval_note = 'Nuveen 原站请求未返回已核验 PDF；本次使用字节数与 SHA-256 完全相同的已核验原文备份。';
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="NAV 折价" />);
    expect(html).toContain('已核验原文备份');
    expect(html).toContain('Nuveen 原站请求未返回已核验 PDF');
  });

  it('distinguishes a market observation date and retrieval date from publication', () => {
    const result = fixture();
    result.industry_facts!.reported_facts = [{ key: 'price', label: '历史收盘价', value: 269.05, unit: 'USD/share', pdf_page: null, source_url: 'https://api.nasdaq.com/history', source_format: 'JSON', as_of: '2025-11-03', published: null, retrieved_on: '2026-10-03' }];
    result.industry_facts!.calculation.operands = ['price'];
    result.industry_case!.sources = [{ id: 'apple_price', title: 'Nasdaq AAPL 历史收盘价', url: 'https://api.nasdaq.com/history', sha256: 'b'.repeat(64), bytes: 80, format: 'JSON', published: null, retrieved_on: '2026-10-03', verification: result.industry_case!.verification }];
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="市盈率" />);
    expect(html).toContain('数据时点 2025-11-03 · 资料获取于 2026-10-03');
    expect(html).not.toContain('发布于 2026-10-03');
  });

  it('resolves a formula symbol through its mapped reported-fact key', () => {
    const result = fixture();
    result.industry_facts!.reported_facts = [{ key: 'online_monthly_churn', label: 'Online客户月均流失率', value: 0.032, unit: 'fraction per month', pdf_page: 5 }];
    result.industry_facts!.calculation.operands = ['online_monthly_churn'];
    result.industry_facts!.calculation.symbol_mapping = { reported_online_monthly_churn: 'online_monthly_churn' };
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="客户流失率" />);
    expect(html).toContain('<p class="ax-industry-symbols">Online客户月均流失率</p>');
    expect(html).not.toContain('reported_online_monthly_churn');
  });

  it('explains derived monthly cash burn and wafer revenue without internal field names', () => {
    const result = fixture();
    result.industry_facts!.calculation.symbol_mapping = {
      monthly_cash_burn: 'annual_operating_cash_burn / 12',
      wafer_revenue: 'revenue * wafer_revenue_share',
    };
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="经营指标" />);
    expect(html).toContain('经营现金消耗 ÷ 12（月均）');
    expect(html).toContain('总营收 × 晶圆营收占比');
    expect(html).not.toContain('monthly_cash_burn');
    expect(html).not.toContain('wafer_revenue');
  });

  it('distinguishes an undated forecast from its collection deadline in plain Chinese', () => {
    const result = fixture();
    result.industry_case!.published = 'undated PDF; collection window ended 2026-02-12';
    result.industry_facts!.reported_facts[0].published = result.industry_case!.published;
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="一致预期" />);
    expect(html).toContain('发布日期未注明；预测收集截至 2026-02-12');
    expect(html).not.toContain('发布于 undated');
  });

  it('shows a fixed historical issuer case with auditable formula and source pages', () => {
    const html = renderToStaticMarkup(<IndustryCaseVisual result={fixture()} name="Rule of 40" />);
    expect(html).toContain('Shopify Inc.');
    expect(html).toContain('<h4>Shopify 公司（SHOP）');
    expect(html).not.toContain('Shopify 公司（Shopify Inc.）');
    expect(html).toContain('FY2024');
    expect(html).toContain('43.7633');
    expect(html).toContain('<small>%</small>');
    expect(html).toContain('业绩公告及其中非 GAAP 指标未经审计');
    expect(html).toContain('不是当前行情或当前所选股票');
    expect(html).toContain('https://www.shopify.com/fy2024.pdf#page=7');
    expect(html).toContain('(8,880 ÷ 7,060 − 1) + (1,597 ÷ 8,880)');
    expect(html).toContain('以上披露值代入公式，得到 Rule of 40');
    expect(html).toContain('8,880 百万美元');
    expect(html).toContain('原文披露 · PDF 第 1 页');
    expect(html).toContain('2024 营业收入；2023 营业收入；自由现金流');
    expect(html).toContain('披露值推导 · 仅由下列原文披露值按所示公式计算');
    expect(html).toContain('经营活动现金流减资本开支');
    expect(html).toContain('bbbbbbbbbbbbbbbb');
    expect(html).toContain('实际核验地址');
    expect(html).toContain('已核验固定缓存');
    expect(html).toContain('字节数与指纹完全匹配');
    expect(html.match(/data-industry-fact=/g)).toHaveLength(3);
    expect(html.match(/data-unit-group=/g)).toHaveLength(1);
  });

  it('does not render an industry case when its required evidence payload is absent', () => {
    const result = fixture();
    delete result.industry_facts;
    expect(renderToStaticMarkup(<IndustryCaseVisual result={result} name="Rule of 40" />)).toBe('');
    result.industry_facts = fixture().industry_facts;
    delete result.industry_case;
    expect(renderToStaticMarkup(<IndustryCaseVisual result={result} name="Rule of 40" />)).toBe('');
  });

  it('uses eBay platform net revenue rather than a teaching proxy for take rate', () => {
    const result = fixture();
    result.concept_id = 'book_platform_take_rate';
    result.industry_case!.issuer.name = 'eBay Inc.';
    result.industry_case!.issuer.ticker = 'EBAY';
    result.industry_case!.issuer.reporting_entity = 'eBay Inc.';
    result.industry_case!.issuer.metric_entity = 'eBay Marketplace';
    result.industry_case!.url = 'https://investors.ebayinc.com/files/doc_financials/2024/q4/eBay-10-K-2024.pdf';
    result.industry_case!.published = '2025-02-27';
    result.values = { platform_take_rate: 0.13771816143275478 };
    result.units = { platform_take_rate: 'fraction' };
    result.notes = ['平台净收入包含市场服务和广告收入；GMV 包含运费与税款。'];
    result.industry_facts!.reported_facts = [
      { key: 'platform_revenue', label: '平台净收入', value: 10283, unit: 'USD millions', pdf_page: 44 },
      { key: 'gross_merchandise_value', label: 'GMV', value: 74667, unit: 'USD millions', pdf_page: 44 },
    ];
    result.industry_facts!.calculation.formula = '平台净收入 ÷ GMV';
    result.industry_facts!.calculation.result_key = 'platform_take_rate';
    result.industry_facts!.calculation.operands = ['platform_revenue', 'gross_merchandise_value'];
    result.industry_facts!.calculation.symbol_mapping = { platform_revenue: '平台净收入', gross_merchandise_value: 'GMV' };
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="平台变现率" />);
    expect(html).toContain('平台净收入');
    expect(html).not.toContain('教学代理');
    expect(html).toContain('13.7718');
  });

  it('labels audited scope precisely and handles raw count and area units independently', () => {
    const result = fixture();
    result.industry_case!.audited = true;
    result.industry_case!.audit_boundary = '该资本充足率附注位于经审计财务报表范围内。';
    result.industry_facts!.reported_facts = [
      { key: 'available_capital', label: '可用资本', value: 138649, unit: 'CNY millions', pdf_page: 336 },
      { key: 'leased_area', label: '已出租面积', value: 335777818, unit: 'square feet', pdf_page: 25 },
      { key: 'properties', label: '物业数', value: 1548, unit: 'count', pdf_page: 25 },
    ];
    const html = renderToStaticMarkup(<IndustryCaseVisual result={result} name="偿付能力充足率" />);
    expect(html).toContain('审计范围内');
    expect(html).toContain('138,649 百万人民币');
    expect(html).toContain('335,777,818 平方英尺');
    expect(html).toContain('1,548 个');
    expect(html.match(/data-unit-group=/g)).toHaveLength(3);
  });
});
