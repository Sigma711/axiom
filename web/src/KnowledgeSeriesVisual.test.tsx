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
  const html = renderToStaticMarkup(<KnowledgeSeriesVisual name="重画" result={result} />);
  expect(html).toContain('局部高点发生位置（仅回看）');
  expect(html).toContain('局部低点确认价（t+2）');
  expect(html).toContain('根 K 线');
  expect((html.match(/data-series-marker=/g) || [])).toHaveLength(6);
  expect(html).toContain('fill="var(--bg)"');
  expect((html.match(/transform="translate/g) || [])).toHaveLength(2);
});
