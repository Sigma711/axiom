import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { StockAdjustmentVisual } from './StockAdjustmentVisual';
import type { PracticeResult } from './types';

const historicalResult = {
  adjustment_evidence: {
    provider: 'yahoo', endpoint: 'https://query1.finance.yahoo.com/v8/finance/chart/AAPL', fetched_at: '2026-09-30T00:00:00Z',
    scope: 'aapl_2020_4_for_1_split_historical_window', issuer_confirmation_url: 'https://www.apple.com/newsroom/2020/07/apple-reports-third-quarter-results/',
    event: { kind: 'split', effective_at: '2020-08-31T13:30:00Z', effective_trading_date: '2020-08-31', numerator: 4, denominator: 1, split_ratio: '4:1' },
    observations: [{ timestamp: '2020-08-28T00:00:00Z', open: 125, high: 126, low: 124, close: 125, volume: 1, adjusted_close: 122 }],
    quote_basis: 'provider_quote_semantics_unverified_for_split_adjustment', adjusted_close_basis: 'provider_adjusted_close_semantics_unverified_for_total_return',
    calculation: 'split_only_price_multiplier = denominator / numerator',
  },
} as PracticeResult;

describe('StockAdjustmentVisual', () => {
  it('shows the dated four-for-one event as a share transformation and keeps the price basis qualified', () => {
    const html = renderToStaticMarkup(<StockAdjustmentVisual result={historicalResult} />);
    expect(html).toContain('2020-08-31');
    expect((html.match(/class="ax-stock-share"/g) ?? []).length).toBe(5);
    expect(html).toContain('1/4');
    expect(html).toContain('供应商 OHLC 与调整收盘价的长期复权口径未经独立核验');
    expect(html).toContain('Apple 官方拆股公告');
    expect(html).toContain('href="https://query1.finance.yahoo.com/v8/finance/chart/AAPL"');
  });

  it('does not render when the result has no dated corporate-action evidence', () => {
    expect(renderToStaticMarkup(<StockAdjustmentVisual result={{} as PracticeResult} />)).toBe('');
  });
});
