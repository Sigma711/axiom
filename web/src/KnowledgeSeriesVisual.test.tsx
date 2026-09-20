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
