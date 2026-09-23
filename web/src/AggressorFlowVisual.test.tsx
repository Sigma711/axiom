import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it } from 'vitest';
import { AggressorFlowVisual } from './AggressorFlowVisual';
import type { PracticeResult } from './types';

const bars = [0, 1, 2].map(hour => ({ timestamp: `2026-09-23T0${hour}:00:00Z`, open: 100, high: 110, low: 90, close: 105, volume: 100 }));
const result = (concept_id: string, series: Array<{ name: string; values: number[] }>, values: Record<string, number>) => ({
  concept_id, series, values, bars, asset_units: { base_asset: 'BTC', quote_asset: 'USDT' },
}) as unknown as PracticeResult;

it('separates Binance taker buy and complementary sell volumes by asset unit and actual hour', () => {
  const html = renderToStaticMarkup(<AggressorFlowVisual result={result('inside_outside', [
    { name: 'taker_buy_base_volume', values: [60, 50, 30] },
    { name: 'taker_sell_base_volume', values: [40, 50, 70] },
    { name: 'taker_buy_quote_volume', values: [6000, 5000, 3000] },
    { name: 'taker_sell_quote_volume', values: [4000, 5000, 7000] },
    { name: 'taker_base_imbalance', values: [20, 0, -40] },
    { name: 'taker_quote_imbalance', values: [2000, 0, -4000] },
  ], { latest_taker_buy_base_volume: 30, latest_taker_sell_base_volume: 70, latest_taker_buy_quote_volume: 3000, latest_taker_sell_quote_volume: 7000 })} />);
  expect(html).toContain('主动成交数量（BTC / 1 小时）');
  expect(html).toContain('主动成交额（USDT / 1 小时）');
  expect(html).toContain('2026-09-23 02:00 UTC');
  expect((html.match(/data-flow-bar=/g) || [])).toHaveLength(12);
  expect(html).toContain('data-flow-line="taker_base_imbalance"');
  expect(html).toContain('不是 A 股内外盘的等价数据');
  expect(html).toContain('最新 1 小时主动买额（USDT）');
});

it('draws signed aggressive order-flow deltas against a visible zero line', () => {
  const html = renderToStaticMarkup(<AggressorFlowVisual result={result('book_order_flow', [
    { name: 'aggressive_buy_base_volume', values: [60, 40, 30] },
    { name: 'aggressive_sell_base_volume', values: [40, 60, 70] },
    { name: 'aggressive_buy_quote_volume', values: [6000, 4000, 3000] },
    { name: 'aggressive_sell_quote_volume', values: [4000, 6000, 7000] },
    { name: 'aggressive_base_delta', values: [20, -20, -40] },
    { name: 'aggressive_quote_delta', values: [2000, -2000, -4000] },
  ], { latest_aggressive_base_delta: -40, latest_aggressive_quote_delta: -4000, window_aggressive_base_delta: -40, window_aggressive_quote_delta: -4000 })} />);
  expect(html).toContain('data-flow-line="aggressive_quote_delta"');
  expect(html).toContain('class="ax-flow-zero"');
  expect(html).toContain('24 小时买卖成交额差（USDT）');
  expect(html).toContain('不代表资金净流入');
});

it('makes the CVD window origin explicit and keeps hourly differences separate from the cumulative lines', () => {
  const html = renderToStaticMarkup(<AggressorFlowVisual result={result('cvd', [
    { name: 'base_delta', values: [20, -20, -15] },
    { name: 'quote_delta', values: [2000, -2000, -1500] },
    { name: 'cvd_base', values: [20, 0, -15] },
    { name: 'cvd_quote', values: [2000, 0, -1500] },
  ], { latest_cvd_base: -15, latest_cvd_quote: -1500, window_base_delta: -15, window_quote_delta: -1500 })} />);
  expect((html.match(/data-flow-bar=/g) || [])).toHaveLength(6);
  expect(html).toContain('data-flow-line="cvd_base"');
  expect(html).toContain('data-flow-line="cvd_quote"');
  expect(html).toContain('最新窗口成交额 CVD（USDT）');
  expect(html).toContain('每小时主动成交额差');
  expect(html).toContain('从窗口起点累计的成交额差');
  expect(html).toContain('不能把这里的数值称为全市场历史 CVD');
  const zero = Number(html.match(/<line[^>]*y1="([^"]+)"[^>]*class="ax-flow-zero"/)?.[1]);
  const deltaBars = [...html.matchAll(/<rect[^>]*data-flow-bar="base_delta"[^>]*>/g)].map(match => match[0]);
  const attribute = (tag: string, name: string) => Number(tag.match(new RegExp(`${name}="([^"]+)"`))?.[1]);
  expect(deltaBars).toHaveLength(3);
  expect(attribute(deltaBars[0], 'y') + attribute(deltaBars[0], 'height')).toBeCloseTo(zero, 6);
  expect(attribute(deltaBars[1], 'y')).toBeCloseTo(zero, 6);
  expect(attribute(deltaBars[1], 'y') + attribute(deltaBars[1], 'height')).toBeGreaterThan(zero);
  expect(deltaBars[1]).toContain('fill="var(--red)"');
});

it('keeps a readable zero baseline when a completed window has no trades', () => {
  const html = renderToStaticMarkup(<AggressorFlowVisual result={result('cvd', [
    { name: 'base_delta', values: [0, 0, 0] }, { name: 'quote_delta', values: [0, 0, 0] },
    { name: 'cvd_base', values: [0, 0, 0] }, { name: 'cvd_quote', values: [0, 0, 0] },
  ], { latest_cvd_base: 0, latest_cvd_quote: 0, window_base_delta: 0, window_quote_delta: 0 })} />);
  expect(html).toContain('class="ax-flow-zero"');
  expect(html).not.toContain('NaN');
  expect(html).not.toContain('Infinity');
});
