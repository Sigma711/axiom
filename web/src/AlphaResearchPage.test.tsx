// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { alphaApi } from './alphaApi';
import { AlphaResearchPage } from './AlphaResearchPage';
import type { AlphaCatalog, AlphaFactor, AlphaResearchResult } from './types';

vi.mock('./alphaApi', () => ({ alphaApi: { catalog: vi.fn(), research: vi.fn() } }));
(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const qlibIds = ['kmid', 'open0', 'roc5', ...Array.from({ length: 155 }, (_, index) => `factor${index + 4}`)];
const qlibFactors: AlphaFactor[] = qlibIds.map((id, index) => ({
  id, name: id.toUpperCase(), status: id === 'open0' || index > 10 ? 'unavailable' : 'available',
  required_fields: id === 'kmid' || id === 'open0' ? ['open', 'close'] : ['close'],
  summary: id === 'kmid' ? '实体收涨幅相对收盘价的比例。' : id === 'roc5' ? '近 5 个交易日的价格变化。' : '因子目录条目。',
  formula: id === 'kmid' ? '(close - open) / open' : id === 'roc5' ? 'Ref(close, 5) / close' : null,
  source_url: id === 'open0' ? '' : 'https://qlib.readthedocs.io/en/latest/component/data.html',
  code_url: id === 'kmid' ? 'https://github.test/qlib-kmid' : null,
}));

const catalog: AlphaCatalog = { collections: [
  { id: 'worldquant101', name: 'WorldQuant Alpha101', source_url: 'https://arxiv.org/pdf/1601.00991', total: 101, factors: [
    { id: 'alpha101', name: 'Alpha 101', status: 'available', required_fields: ['open', 'high', 'low', 'close'], summary: '比较收盘与开盘。', formula: '(close - open) / (high - low + 0.001)', source_url: 'https://arxiv.org/pdf/1601.00991', code_url: 'https://github.test/alpha101' },
    { id: 'alpha1', name: 'Alpha 1', status: 'unavailable', required_fields: ['close', 'cross_section_universe'], summary: '在同一天比较股票名次。', formula: 'rank(close)', source_url: 'https://arxiv.org/pdf/1601.00991', code_url: null },
    { id: 'alpha41', name: 'Alpha 41', status: 'unavailable', required_fields: ['high', 'low', 'vwap'], summary: '比较价格与真实 VWAP。', formula: 'sqrt(high * low) - vwap', source_url: 'https://arxiv.org/pdf/1601.00991', code_url: null },
    { id: 'alpha7', name: 'Alpha 7', status: 'unavailable', required_fields: ['close', 'volume', 'adv20'], summary: '比较当日和平均成交金额。', formula: 'adv20 < volume', source_url: 'https://arxiv.org/pdf/1601.00991', code_url: null },
  ] },
  { id: 'qlib_alpha158', name: 'Microsoft Qlib Alpha158', source_url: 'https://qlib.readthedocs.io/en/latest/component/data.html', total: 158, factors: qlibFactors },
] };

const resultFor = (factor: AlphaFactor, correlation: number): AlphaResearchResult => ({
  factor, market_provenance: { provider: 'yahoo', endpoint: 'https://query.example/AAPL', price_basis: 'provider_adjustment_unverified', corporate_actions: 'not_simulated' },
  bars: [{ timestamp: '2026-01-01', open: 100, high: 105, low: 99, close: 103, volume: 1200 }],
  series: [{ timestamp: '2026-01-01', value: 0.75 }, { timestamp: '2026-01-02', value: null }, { timestamp: '2026-01-03', value: -0.25 }],
  forward_returns: [{ timestamp: '2026-01-01', value: 0.02 }, { timestamp: '2026-01-03', value: -0.01 }],
  stats: { paired_count: 2, time_series_correlation: correlation }, status: 'computed', reason: null, notes: ['样本很短。'],
});

const flush = async () => act(async () => { await Promise.resolve(); await Promise.resolve(); });
const change = async (element: HTMLInputElement | HTMLSelectElement, value: string) => act(async () => {
  const prototype = element instanceof HTMLInputElement ? HTMLInputElement.prototype : HTMLSelectElement.prototype;
  Object.getOwnPropertyDescriptor(prototype, 'value')!.set!.call(element, value);
  element.dispatchEvent(new Event(element instanceof HTMLInputElement ? 'input' : 'change', { bubbles: true }));
});

describe('Alpha research learning flow', () => {
  let host: HTMLDivElement; let root: Root;
  beforeEach(() => {
    window.history.replaceState(null, '', '/learn/alphas');
    host = document.createElement('div'); document.body.append(host); root = createRoot(host);
    vi.mocked(alphaApi.catalog).mockResolvedValue(catalog);
    vi.mocked(alphaApi.research).mockResolvedValue(resultFor(catalog.collections[0].factors[0], -.42));
  });
  afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.resetAllMocks(); });

  it('lists and searches all 158 Qlib factors in a separate learning center', async () => {
    await act(async () => root.render(<AlphaResearchPage />)); await flush();
    await change(host.querySelector('[aria-label="选择因子库"]')!, 'qlib_alpha158');
    expect(host.textContent).toContain('Microsoft Qlib Alpha158'); expect(host.textContent).toContain('158 / 158');
    expect(host.querySelectorAll('[data-factor-card]')).toHaveLength(158);
    await change(host.querySelector('[aria-label="搜索因子"]')!, 'roc5');
    expect(host.querySelectorAll('[data-factor-card]')).toHaveLength(1);
    await act(async () => (host.querySelector('[data-factor-card]') as HTMLButtonElement).click());
    expect(host.textContent).toContain('近 5 个交易日的价格变化');
    expect(host.querySelector('[aria-label="ROC5 因子示意图"]')).not.toBeNull();
    await change(host.querySelector('[aria-label="搜索因子"]')!, 'open0');
    await act(async () => (host.querySelector('[data-factor-card]') as HTMLButtonElement).click());
    expect(host.textContent).toContain('当前只能学习，尚不可运行');
    expect(host.querySelector('[aria-label="当前因子方法"] a')).toBeNull();
  });

  it('filters runnable factors and runs a verified Qlib factor', async () => {
    await act(async () => root.render(<AlphaResearchPage />)); await flush();
    await change(host.querySelector('[aria-label="选择因子库"]')!, 'qlib_alpha158');
    await change(host.querySelector('[aria-label="因子状态"]')!, 'available');
    expect(host.querySelectorAll('[data-factor-card]')).toHaveLength(10);
    expect((host.querySelector('[aria-label="选择因子"]') as HTMLSelectElement).value).toBe('kmid');
    await act(async () => host.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))); await flush();
    expect(alphaApi.research).toHaveBeenCalledWith({ collection: 'qlib_alpha158', id: 'kmid', source: 'a_share', symbol: '600519', limit: 160 });
    expect(host.textContent).toContain('-0.4200'); expect(host.textContent).toContain('查看核验实现');
  });

  it('explains cross-section, VWAP, and dollar-volume requirements with relevant visuals', async () => {
    await act(async () => root.render(<AlphaResearchPage />)); await flush();
    expect(host.querySelectorAll('[data-factor-card]')).toHaveLength(4);
    const open = async (id: string) => act(async () => (Array.from(host.querySelectorAll<HTMLButtonElement>('[data-factor-card]')).find(card => card.querySelector('span')?.textContent === `Alpha ${id.slice(5)}`)!).click());
    await open('alpha1');
    expect(host.querySelector('[aria-label="Alpha 1 同日股票比较示意图"]')).not.toBeNull();
    expect(host.textContent).toContain('单只股票没有这个名次');
    await open('alpha41');
    expect(host.querySelector('[aria-label="Alpha 41 成交加权价示意图"]')).not.toBeNull();
    expect(host.textContent).toContain('不能算出原式所需的 VWAP');
    await open('alpha7');
    expect(host.textContent).toContain('股数和成交金额单位不同');
  });

  it('gives every Qlib factor family a concrete reading prompt and distinguishes worked examples', async () => {
    const families = ['kmid', 'klen', 'roc5', 'ma5', 'std5', 'rank5', 'corr5', 'beta5', 'rsqr5', 'resi5', 'max5', 'min5', 'qtlu5', 'qtld5', 'rsv5', 'imax5', 'imin5', 'imxd5', 'cord5', 'cntp5', 'cntn5', 'cntd5', 'sump5', 'sumn5', 'sumd5', 'vma5', 'vstd5', 'wvma5', 'vsump5', 'vsumn5', 'vsumd5'];
    const factors = families.map(id => ({ ...qlibFactors[0], id, name: id.toUpperCase(), formula: id === 'corr5' ? 'Corr(Ref($close, 1), Log($volume+1), 5)' : id === 'vma5' ? 'Mean($volume, 5)/$volume' : 'Mean($close, 5)/$close' }));
    vi.mocked(alphaApi.catalog).mockResolvedValue({ collections: [catalog.collections[0], { ...catalog.collections[1], total: factors.length, factors }] });
    await act(async () => root.render(<AlphaResearchPage />)); await flush();
    await change(host.querySelector('[aria-label="选择因子库"]')!, 'qlib_alpha158');
    for (const factor of factors) {
      await act(async () => (Array.from(host.querySelectorAll<HTMLButtonElement>('[data-factor-card]')).find(card => card.querySelector('span')?.textContent === factor.name)!).click());
      expect(host.querySelector('.ax-alpha-method h4')?.textContent).toContain(factor.name);
      expect(host.textContent).toContain('它在问什么？');
      expect(host.textContent).toContain('动手想一想');
    }
    expect(host.textContent).toContain('量增与量减的净差');
    await act(async () => (Array.from(host.querySelectorAll<HTMLButtonElement>('[data-factor-card]')).find(card => card.querySelector('span')?.textContent === 'CORR5')!).click());
    expect(host.textContent).toContain('先把两条序列按日期对齐');
    expect(host.textContent).toContain('Ref / delay');
  });

  it('ignores a stale research response after a newer request wins', async () => {
    let resolveFirst!: (value: AlphaResearchResult) => void;
    vi.mocked(alphaApi.research).mockImplementationOnce(() => new Promise(resolve => { resolveFirst = resolve; })).mockResolvedValueOnce(resultFor(catalog.collections[0].factors[0], .73));
    await act(async () => root.render(<AlphaResearchPage />)); await flush();
    await act(async () => host.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
    await act(async () => host.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
    await flush(); expect(host.textContent).toContain('0.7300');
    await act(async () => resolveFirst(resultFor(catalog.collections[0].factors[0], -.99))); await flush();
    expect(host.textContent).toContain('0.7300'); expect(host.textContent).not.toContain('-0.9900');
  });

  it('can retry a failed catalog request', async () => {
    vi.mocked(alphaApi.catalog).mockRejectedValueOnce(new Error('catalog unavailable')).mockResolvedValueOnce(catalog);
    await act(async () => root.render(<AlphaResearchPage />)); await flush();
    expect(host.textContent).toContain('catalog unavailable');
    await act(async () => host.querySelector<HTMLButtonElement>('[data-alpha-retry]')!.click()); await flush();
    expect(host.textContent).toContain('WorldQuant Alpha101');
  });
});
