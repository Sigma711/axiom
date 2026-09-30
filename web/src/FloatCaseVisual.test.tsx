import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { FloatCaseVisual } from './FloatCaseVisual';
import type { PracticeResult } from './types';

const fixture = (): PracticeResult => ({
  concept_id: 'book_free_float', status: 'computed', reason: null, input_kind: 'industry_case',
  provenance: 'verified_independent_a_share_float_case',
  values: { book_free_float: 0.4337223591954553, unrestricted_shares: 1252270215, non_free_float_shares: 709132623, free_float_shares: 543137592, close_price: 1377.18 },
  units: { book_free_float: 'fraction', unrestricted_shares: 'shares', non_free_float_shares: 'shares', free_float_shares: 'shares', close_price: 'CNY/share' },
  series: [], notes: ['无限售条件流通股份不等于自由流通股。'], module: 'data', source: 'issuer_disclosure', symbol: '600519', bars: [],
  float_case: {
    case_id: 'moutai_2025_12_31_free_float_and_circulating_cap', issuer: { name: '贵州茅台酒股份有限公司', ticker: '600519' },
    as_of: '2025-12-31', share_register_date: '2025-12-31', published: '2026-04-17',
    definition: { provider: 'China Securities Index Co., Ltd.', rule: 'total shares minus non-free holdings', threshold: '5% including concert parties', calculation_scope: 'transparent reproduction, not a CSI-published security-level estimate' },
    non_free_float_holders: [
      { name: '中国贵州茅台酒厂（集团）有限责任公司', shares: 681282935, classification: 'state_owned_controlling_holder', relationship: 'controller' },
      { name: '贵州茅台酒厂（集团）技术开发有限公司', shares: 27849688, classification: 'concert_party_of_controlling_holder', relationship: 'wholly_owned_subsidiary_and_concert_party' },
    ],
    price: { provider: 'Tencent public historical kline API', endpoint: 'https://price.example', trading_date: '2025-12-31', close: 1377.18, currency: 'CNY', basis: 'unadjusted_daily_close', request_adjustment: 'none' },
    sources: [
      { kind: 'issuer_annual_report', official_url: 'https://annual.example', pdf_pages: [47, 48, 49], sha256: 'a'.repeat(64), bytes: 1082847, verification: { status: 'verified_immutable_cache', requested_url: 'https://annual.example', matched_sha256: 'a'.repeat(64), matched_bytes: 1082847 } },
      { kind: 'issuer_concert_party_announcement', official_url: 'https://concert.example', pdf_pages: [1, 2, 3], sha256: 'b'.repeat(64), bytes: 92857, verification: { status: 'verified_then_cached', requested_url: 'https://concert.example', matched_sha256: 'b'.repeat(64), matched_bytes: 92857 } },
      { kind: 'official_index_methodology', official_url: 'https://method.example', pdf_pages: [5, 6], sha256: 'c'.repeat(64), bytes: 1184619, verification: { status: 'verified_immutable_cache', requested_url: 'https://method.example', matched_sha256: 'c'.repeat(64), matched_bytes: 1184619 } },
      { kind: 'historical_price_api', official_url: 'https://price.example', trading_date: '2025-12-31', basis: 'unadjusted_daily_close' },
    ],
  },
});

describe('FloatCaseVisual', () => {
  it('shows distinct unrestricted and free-float counts with aligned date and sources', () => {
    const html = renderToStaticMarkup(<FloatCaseVisual result={fixture()} name="自由流通比例" />);
    expect(html).toContain('贵州茅台酒股份有限公司（600519）');
    expect(html).toContain('2025-12-31');
    expect(html).toContain('1,252,270,215');
    expect(html).toContain('543,137,592');
    expect(html).toContain('43.3722<small>%</small>');
    expect(html).toContain('681,282,935');
    expect(html).toContain('27,849,688');
    expect(html).toContain('一致行动人');
    expect(html).toContain('不冒充中证公司发布的证券级自由流通量');
    expect(html).toContain('https://annual.example');
    expect(html).toContain('https://concert.example');
    expect(html).toContain('https://method.example');
    expect(html).toContain('aaaaaaaaaaaaaaaa');
    expect(html).toContain('<details class="ax-filing-sources">');
    expect(html).toContain('原文已核对');
    expect(html).not.toContain('verified_immutable_cache');
  });

  it('shows the same-date unadjusted close and circulating market cap', () => {
    const result = fixture();
    result.concept_id = 'book_float_market_cap';
    result.values.book_float_market_cap = 1724601494693.7;
    const html = renderToStaticMarkup(<FloatCaseVisual result={result} name="流通市值" />);
    expect(html).toContain('1,377.18 元/股');
    expect(html).toContain('1,724,601,494,693.70 <small>元</small>');
    expect(html).toContain('未复权日收盘价');
    expect(html).toContain('股数日期与价格交易日一致');
  });

  it('fails closed without the evidence payload', () => {
    const result = fixture();
    delete result.float_case;
    expect(renderToStaticMarkup(<FloatCaseVisual result={result} name="自由流通比例" />)).toBe('');
  });
});
