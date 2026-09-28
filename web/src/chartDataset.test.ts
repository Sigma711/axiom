import { describe, expect, it } from 'vitest';
import { chartDataset } from './chartDataset';
import type { Bar } from './types';

const bars: Bar[] = [
  { timestamp: '2024-01-01T00:00:00Z', open: 10, high: 14, low: 8, close: 12, volume: 100 },
  { timestamp: '2024-01-02T00:00:00Z', open: 12, high: 16, low: 9, close: 15, volume: 120 },
];

describe('chartDataset', () => {
  it('derives Heikin Ashi from one raw snapshot while preserving its metadata and raw bars', () => {
    const provenance = { provider: 'tencent', endpoint: 'https://example.test/day', price_basis: 'unadjusted_requested', corporate_actions: 'not_simulated' };
    const raw = { bars, symbol: '600519', source: 'a_share', indicators: {}, market_provenance: provenance, bar_origin: 'server_fetched_completed_source_bars' };
    const result = chartDataset(raw, 'heikin_ashi');
    expect(result.bars).toBe(bars);
    expect(result.market_provenance).toBe(provenance);
    expect(result.bar_origin).toBe('server_fetched_completed_source_bars');
    expect(result.display_price_basis).toBe('heikin_ashi_synthetic');
    expect(result.display_bars).toEqual([
      { timestamp: '2024-01-01T00:00:00Z', open: 11, high: 14, low: 8, close: 11, volume: 100 },
      { timestamp: '2024-01-02T00:00:00Z', open: 11, high: 16, low: 9, close: 13, volume: 120 },
    ]);
  });

  it('uses observed bars directly for a standard candle chart', () => {
    const result = chartDataset({ bars }, 'candle');
    expect(result.display_bars).toBe(bars);
    expect(result.display_price_basis).toBe('observed_ohlc');
  });
});
