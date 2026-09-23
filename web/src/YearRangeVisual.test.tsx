import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { YearRangeVisual } from './YearRangeVisual';
import type { PracticeResult } from './types';

const sample = (position: number | null): PracticeResult => ({
  concept_id: 'book_52w_range', status: 'computed', reason: null, input_kind: 'market_bars', provenance: 'provided_market_bars',
  values: { low_52w: 50, high_52w: position == null ? 50 : 100, latest_close: position == null ? 50 : 80, distance_from_high: position == null ? 0 : -0.2, position_in_range: position },
  series: [], notes: [], module: 'data', source: 'a_share', symbol: '600519', bars: [],
  year_range: { window_start: '2025-09-23T00:00:00Z', window_end: '2026-09-22T00:00:00Z', as_of: '2026-09-22T00:00:00Z', bar_count: 240, price_basis: 'provider_ohlc_adjustment_unverified', source: 'a_share', pre_window_observation: '2025-09-22T00:00:00Z', window_start_inclusive: false },
});

describe('YearRangeVisual', () => {
  it('places the close using the backend range position and discloses the observed window', () => {
    const html = renderToStaticMarkup(<YearRangeVisual result={sample(0.6)} />);
    expect(html).toContain('left:60%');
    expect(html).toContain('区间位置 60.0%');
    expect(html).toContain('240 根已收盘日线');
    expect(html).toContain('复权口径未经统一核验');
  });

  it('does not invent a position when all observed prices are equal', () => {
    const html = renderToStaticMarkup(<YearRangeVisual result={sample(null)} />);
    expect(html).toContain('left:50%');
    expect(html).toContain('区间位置无定义');
  });

  it('omits a chart when the response lacks an observed range', () => {
    const withoutRange = sample(0.6);
    delete withoutRange.year_range;
    expect(renderToStaticMarkup(<YearRangeVisual result={withoutRange} />)).toBe('');
    const withoutHigh = sample(0.6);
    withoutHigh.values.high_52w = null;
    expect(renderToStaticMarkup(<YearRangeVisual result={withoutHigh} />)).toBe('');
  });
});
