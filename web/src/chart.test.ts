import { describe, expect, it } from 'vitest';
import { indicatorPanel, validSeries } from './chart';

describe('indicatorPanel', () => {
  it('keeps price overlays separate from non-price indicator panels', () => {
    expect(indicatorPanel('bbands_20_upper')).toBe('price');
    expect(indicatorPanel('vwap')).toBe('price');
    expect(indicatorPanel('rsi_14')).toBe('oscillator');
    expect(indicatorPanel('macd_histogram')).toBe('momentum');
    expect(indicatorPanel('obv')).toBe('volume');
    expect(indicatorPanel('atr_14')).toBe('volatility');
  });
});

describe('validSeries', () => {
  it('preserves valid points and gaps while rejecting invalid values and timestamps', () => {
    expect(validSeries([
      { x: '2025-01-01T00:00:00Z', y: 42 }, null,
      { x: 'invalid', y: 43 }, { x: '2025-01-02T00:00:00Z', y: Number.NaN },
    ])).toEqual([{ x: '2025-01-01T00:00:00Z', y: 42 }, null, null, null]);
  });
});
