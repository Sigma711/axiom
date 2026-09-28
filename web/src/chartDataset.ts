import type { Bar, ChartType } from './types';

export type DisplayPriceBasis = 'observed_ohlc' | 'heikin_ashi_synthetic';

export function chartDataset<T extends { bars: Bar[] }>(raw: T, chartType: ChartType): T & {
  display_bars: Bar[];
  display_price_basis: DisplayPriceBasis;
} {
  if (chartType === 'candle') {
    return { ...raw, display_bars: raw.bars, display_price_basis: 'observed_ohlc' };
  }
  const displayBars: Bar[] = [];
  for (const bar of raw.bars) {
    const close = (bar.open + bar.high + bar.low + bar.close) / 4;
    const previous = displayBars[displayBars.length - 1];
    const open = previous ? (previous.open + previous.close) / 2 : (bar.open + bar.close) / 2;
    displayBars.push({
      ...bar,
      open,
      high: Math.max(bar.high, open, close),
      low: Math.min(bar.low, open, close),
      close,
    });
  }
  return { ...raw, display_bars: displayBars, display_price_basis: 'heikin_ashi_synthetic' };
}
