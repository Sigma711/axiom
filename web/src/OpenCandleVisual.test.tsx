import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it } from 'vitest';
import { OpenCandleVisual } from './OpenCandleVisual';
import type { PracticeResult } from './types';

const completed = { timestamp: '2026-09-23T03:00:00Z', open: 100, high: 104, low: 99, close: 102, volume: 10 };
const provisional = { timestamp: '2026-09-23T04:00:00Z', open: 102, high: 107, low: 101, close: 105, volume: 3 };
const result = { bars: [completed], provisional_snapshot: {
  candle: provisional, is_closed: false, fetched_at: '2026-09-23T04:12:00Z',
  expected_close_at: '2026-09-23T05:00:00Z', completion_evidence: 'timestamp-derived',
} } as PracticeResult;

it('draws actual completed and provisional OHLC separately and labels the open candle', () => {
  const html = renderToStaticMarkup(<OpenCandleVisual result={result} />);
  expect(html).toContain('真实 OHLC 对比');
  expect(html).toContain('data-candle="completed"');
  expect(html).toContain('data-candle="provisional"');
  expect(html).toContain('2026-09-23 03:00 UTC');
  expect(html).toContain('2026-09-23 04:00 UTC');
  expect(html).toContain('当前快照 · 未收盘');
  expect(html).toContain('stroke-dasharray="5 3"');
  expect(html).toContain('临时价 105');
  expect((html.match(/<rect /g) || [])).toHaveLength(2);
});

it('never renders an unmarked current candle as if it were final', () => {
  expect(renderToStaticMarkup(<OpenCandleVisual result={{ ...result, provisional_snapshot: undefined }} />)).toBe('');
  expect(renderToStaticMarkup(<OpenCandleVisual result={{ ...result, bars: [] }} />)).toBe('');
});
