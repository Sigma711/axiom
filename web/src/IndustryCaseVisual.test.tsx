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
    expect(html).toContain('2024 年营业收入；2023 年营业收入；2024 年非 GAAP 自由现金流');
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
    expect(html).toContain('2024 年平台净收入');
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
