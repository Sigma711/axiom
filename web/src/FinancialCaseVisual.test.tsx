import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { FinancialCaseVisual } from './FinancialCaseVisual';
import type { PracticeResult } from './types';

const fixture = (): PracticeResult => ({
  concept_id: 'book_fcf', status: 'computed', reason: null, input_kind: 'filing_case', provenance: 'verified_issuer_filing_case', values: { free_cash_flow: 98767 }, units: { free_cash_flow: 'USD millions' }, series: [], notes: ['自由现金流不是现金余额。'], module: 'data', source: 'us_stock', symbol: 'AAPL', bars: [],
  filing_case: { case_id: 'apple_fy2025', issuer: 'Apple Inc.', ticker: 'AAPL', scope: 'annual', period: { start: '2024-09-29', end: '2025-09-27', fiscal_year: 2025 }, comparison_period: { start: '2023-10-01', end: '2024-09-28', fiscal_year: 2024 }, published: '2025-10-30', audited: false, status: 'Unaudited', source_kind: 'issuer_pdf', url: 'https://www.apple.com/source.pdf', sha256: 'a'.repeat(64), bytes: 4919649, pages: { income_statement: 1, balance_sheet: 2, cash_flow: 3 }, verification: { cache_status: 'verified_cache', verified_at: '2026-09-28T00:00:00Z', requested_url: 'https://www.apple.com/source.pdf', matched_sha256: 'a'.repeat(64), matched_bytes: 4919649 } },
  facts: { units: {}, annual_income_statement: { '2025': { sales: 416161, netincome: 112010, basic_eps: 7.49, diluted_eps: 7.46 }, '2024': { sales: 391035, netincome: 93736, basic_eps: 6.11, diluted_eps: 6.08 } }, balance_sheets: {}, annual_cash_flows: { '2025': { cfo: 111482, ppe_capex_cash_outflow: 12715 }, '2024': { cfo: 118254, ppe_capex_cash_outflow: 9447 } }, calculation_boundaries: {} },
});

describe('FinancialCaseVisual', () => {
  it('shows real cash-flow operands with compatible units, historical dates, and issuer evidence', () => {
    const html = renderToStaticMarkup(<FinancialCaseVisual result={fixture()} name="自由现金流" />);
    expect(html).toContain('98,767');
    expect(html).toContain('111,482');
    expect(html).toContain('12,715');
    expect(html).toContain('百万美元');
    expect(html).toContain('未经审计');
    expect(html).toContain('2024-09-29');
    expect(html).toContain('2025-09-27');
    expect(html).toContain('https://www.apple.com/source.pdf');
    expect(html).toContain('不是当前行情');
    expect(html.match(/data-filing-fact=/g)).toHaveLength(2);
    expect(html).toContain('第 3 页');
  });
  it('keeps disclosed EPS separate from rounded-input cross-checks', () => {
    const result = fixture(); result.concept_id = 'eps';
    result.values = { reported_basic_eps: 7.49, reported_diluted_eps: 7.46, approx_basic_eps_cross_check: 7.49306 }; result.units = { reported_basic_eps: 'USD/share', reported_diluted_eps: 'USD/share', approx_basic_eps_cross_check: 'USD/share' };
    const html = renderToStaticMarkup(<FinancialCaseVisual result={result} name="每股收益" />);
    expect(html).toContain('披露基本每股收益');
    expect(html).toContain('近似复算');
    expect(html).toContain('美元/股');
  });
  it('does not invent facts when source metadata or facts are unavailable', () => {
    const result = fixture(); delete result.facts;
    expect(renderToStaticMarkup(<FinancialCaseVisual result={result} name="FCF" />)).toBe('');
    delete result.filing_case;
    expect(renderToStaticMarkup(<FinancialCaseVisual result={result} name="FCF" />)).toBe('');
  });
  it('explains a negative cash conversion cycle and preserves its real day basis', () => {
    const result = fixture(); result.concept_id = 'book_ccc';
    result.values = { cash_conversion_cycle: -71.625008 }; result.units = { cash_conversion_cycle: 'days' };
    const html = renderToStaticMarkup(<FinancialCaseVisual result={result} name="现金转换周期" />);
    expect(html).toContain('-71.625');
    expect(html).toContain('364 天');
    expect(html).toContain('代理');
    expect(html).toContain('负值');
    expect(html).toContain('未披露');
  });
});
