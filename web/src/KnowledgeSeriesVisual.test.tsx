import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it } from 'vitest';
import { KnowledgeSeriesVisual } from './KnowledgeSeriesVisual';
import type { PracticeResult } from './types';
it('displays all three band traces without forcing a distant zero baseline', () => {
  const result = {series: [{name:'upper',values:[110,111]}, {name:'middle',values:[105,106]}, {name:'lower',values:[100,101]}], units:{upper:'currency',middle:'currency',lower:'currency'}} as unknown as PracticeResult;
  const html = renderToStaticMarkup(<KnowledgeSeriesVisual name="布林带" result={result} />);
  expect(html).toContain('上轨'); expect(html).toContain('中轨'); expect(html).toContain('下轨');
  expect(html).toContain('99.12'); expect(html).toContain('111.9');
});
it('keeps unit panels distinct and leaves an internal null gap disconnected', () => {
  const result = {series:[{name:'price',values:[10,null,12]}, {name:'volume',values:[1000,1100,1200]}],units:{price:'currency',volume:'shares'}} as unknown as PracticeResult;
  const html = renderToStaticMarkup(<KnowledgeSeriesVisual name="价格与量" result={result} />);
  expect(html).toContain('价格 / 元'); expect(html).toContain('股');
  const paths = [...html.matchAll(/<path d="([^"]*)"/g)].map(m => m[1]);
  expect(paths[0].match(/M/g)).toHaveLength(2); expect(paths[0]).not.toContain('L');
});
it('groups repainting prices, labels retrospective markers, and renders every isolated event', () => {
  const result = {series:[
    {name:'pivot_high_occurrence',values:[null,10,null,null,12,null]},
    {name:'pivot_low_occurrence',values:[null,null,8,null,null,7]},
    {name:'confirmed_pivot_high',values:[null,null,null,10,null,null]},
    {name:'confirmed_pivot_low',values:[null,null,null,null,8,null]},
    {name:'confirmation_delay_bars',values:[null,null,null,2,2,null]},
  ], units:{pivot_high_occurrence:'price',pivot_low_occurrence:'price',confirmed_pivot_high:'price',confirmed_pivot_low:'price',confirmation_delay_bars:'bars'}} as unknown as PracticeResult;
  result.bars = Array.from({ length: 6 }, (_, index) => ({ timestamp: `2026-09-01T0${index}:00:00Z`, open: 9, high: 12, low: 7, close: 10, volume: 1 }));
  const html = renderToStaticMarkup(<KnowledgeSeriesVisual name="重画" result={result} />);
  expect(html).toContain('局部高点发生位置（仅回看）');
  expect(html).toContain('局部低点确认价（t+2）');
  expect(html).toContain('根 K 线');
  expect(html).toContain('2026-09-01 00:00 UTC');
  expect(html).toContain('<title>局部高点发生位置（仅回看） · 2026-09-01 01:00 UTC · 10</title>');
  expect((html.match(/data-series-marker=/g) || [])).toHaveLength(6);
  expect(html).toContain('fill="var(--bg)"');
  expect((html.match(/transform="translate/g) || [])).toHaveLength(2);
});

it('labels both real-market timeframe windows and keeps sparse starts visible', () => {
  const bars = Array.from({ length: 21 }, (_, index) => ({ timestamp: `2026-09-${String(index + 1).padStart(2, '0')}T00:00:00Z`, open: 100 + index, high: 101 + index, low: 99 + index, close: 100 + index, volume: 1 }));
  const result = { bars, series: [
    { name: 'close_price', values: bars.map(bar => bar.close) },
    { name: 'short_horizon_start', values: bars.map((bar, index) => index === 15 ? bar.close : null) },
    { name: 'long_horizon_start', values: bars.map((bar, index) => index === 0 ? bar.close : null) },
  ], units: { close_price: 'price', short_horizon_start: 'price', long_horizon_start: 'price' } } as unknown as PracticeResult;
  const html = renderToStaticMarkup(<KnowledgeSeriesVisual name="时间尺度" result={result} />);
  expect(html).toContain('ax-wide-series-chart');
  expect(html).toContain('真实收盘价');
  expect(html).toContain('短期窗口起点（5 根）');
  expect(html).toContain('长期窗口起点（20 根）');
  expect(html).toContain('2026-09-21 00:00 UTC');
  expect((html.match(/data-series-marker=/g) || [])).toHaveLength(2);
});

it('explains the real MACD histogram scaling conventions on one price-unit axis', () => {
  const result = { series: [
    { name: 'macd_histogram_x1', values: [null, 1, 2] },
    { name: 'macd_histogram_x2', values: [null, 2, 4] },
  ], units: { macd_histogram_x1: 'macd_price', macd_histogram_x2: 'macd_price' } } as unknown as PracticeResult;
  const html = renderToStaticMarkup(<KnowledgeSeriesVisual name="公式口径" result={result} />);
  expect(html).toContain('ax-wide-series-chart');
  expect(html).toContain('MACD 柱体（价格单位）');
  expect(html).toContain('MACD 柱体 ×1');
  expect(html).toContain('MACD 柱体 ×2');
  expect(html).toContain('不同单位分图');
  expect(html).not.toContain('两种口径差值');
  expect((html.match(/transform="translate/g) || [])).toHaveLength(1);
});
