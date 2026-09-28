// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from './api';
import { SymbolPicker } from './SymbolPicker';

vi.mock('./api', () => ({ api: { listSymbols: vi.fn() } }));

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const listSymbols = vi.mocked(api.listSymbols);
const flush = async () => { await act(async () => { await Promise.resolve(); await Promise.resolve(); }); };
const setInputValue = (input: HTMLInputElement, value: string) => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
};

describe('SymbolPicker directory refresh', () => {
  let host: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.useFakeTimers();
    host = document.createElement('div');
    document.body.append(host);
    root = createRoot(host);
  });
  afterEach(async () => {
    await act(async () => root.unmount());
    host.remove();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('refreshes a typed symbol search after an incomplete cold-start directory becomes complete', async () => {
    let resolveInitial: (value: unknown) => void = () => undefined;
    const initial = new Promise(resolve => { resolveInitial = resolve; });
    listSymbols
      .mockImplementationOnce(() => initial as ReturnType<typeof api.listSymbols>)
      .mockResolvedValueOnce({ items: [], total: 0, offset: 0, has_more: false, universe_count: 30, status: 'warming', complete: false, symbols: [], count: 0, source: 'binance' })
      .mockResolvedValueOnce({ items: [{ symbol: 'DOGEUSDT', name: 'Dogecoin', exchange: 'Binance' }], total: 1, offset: 0, has_more: false, universe_count: 496, status: 'ready', complete: true, symbols: ['DOGEUSDT'], count: 1, source: 'binance' });

    await act(async () => { root.render(<SymbolPicker source="binance" value="" onChange={() => undefined} />); });
    const trigger = host.querySelector<HTMLButtonElement>('[aria-label="交易对"]')!;
    await act(async () => trigger.click());
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(listSymbols).toHaveBeenCalledWith('binance', '', 0, 50);

    const input = host.querySelector<HTMLInputElement>('[aria-label="搜索交易对"]')!;
    await act(async () => setInputValue(input, 'DOGEUSDT'));
    await act(async () => { await vi.advanceTimersByTimeAsync(180); });
    await flush();
    expect(listSymbols).toHaveBeenLastCalledWith('binance', 'DOGEUSDT', 0, 50);
    expect(host.textContent).toContain('没有匹配标的');

    await act(async () => { await vi.advanceTimersByTimeAsync(750); });
    await act(async () => { await vi.advanceTimersByTimeAsync(180); });
    await flush();
    expect(listSymbols).toHaveBeenLastCalledWith('binance', 'DOGEUSDT', 0, 50);
    expect(host.textContent).toContain('DOGEUSDT');

    resolveInitial({ items: [], total: 0, offset: 0, has_more: false, universe_count: 30, status: 'warming', complete: false });
  });

  it('pages complete results and accepts exact, free-form, and escape keyboard actions', async () => {
    const onChange = vi.fn();
    listSymbols
      .mockResolvedValueOnce({ items: [{ symbol: 'AAPL', name: 'Apple', exchange: 'NASDAQ' }], total: 51, offset: 0, has_more: true, universe_count: 5000, status: 'ready', complete: true, symbols: ['AAPL'], count: 1, source: 'us_stock' })
      .mockResolvedValueOnce({ items: [{ symbol: 'MSFT', name: 'Microsoft', exchange: 'NASDAQ' }], total: 51, offset: 50, has_more: false, universe_count: 5000, status: 'ready', complete: true, symbols: ['MSFT'], count: 1, source: 'us_stock' });
    await act(async () => { root.render(<SymbolPicker source="us_stock" value="" onChange={onChange} />); });
    const trigger = host.querySelector<HTMLButtonElement>('[aria-label="交易对"]')!;
    await act(async () => trigger.click());
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await flush();
    expect(host.textContent).toContain('目录共 5,000 个');
    await act(async () => host.querySelectorAll<HTMLButtonElement>('.ax-symbol-pages button')[1].click());
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await flush();
    expect(host.textContent).toContain('2 / 2');
    await act(async () => host.querySelector<HTMLButtonElement>('.ax-symbol-pages button')!.click());

    let input = host.querySelector<HTMLInputElement>('[aria-label="搜索交易对"]')!;
    await act(async () => setInputValue(input, 'AAPL'));
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })));
    expect(onChange).toHaveBeenLastCalledWith('AAPL');

    await act(async () => trigger.click());
    input = host.querySelector<HTMLInputElement>('[aria-label="搜索交易对"]')!;
    await act(async () => setInputValue(input, 'TSLA'));
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })));
    expect(onChange).toHaveBeenLastCalledWith('TSLA');

    await act(async () => trigger.click());
    input = host.querySelector<HTMLInputElement>('[aria-label="搜索交易对"]')!;
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })));
    expect(host.querySelector('[role="listbox"]')).toBeNull();
  });

  it('does not cache a directory response that completes after the picker closes', async () => {
    let resolve: (value: unknown) => void = () => undefined;
    const pending = new Promise(value => { resolve = value; });
    listSymbols
      .mockImplementationOnce(() => pending as ReturnType<typeof api.listSymbols>)
      .mockResolvedValueOnce({ items: [{ symbol: 'CURRENT', name: 'Current result', exchange: 'SSE' }], total: 1, offset: 0, has_more: false, universe_count: 5000, status: 'ready', complete: true, symbols: ['CURRENT'], count: 1, source: 'a_share' });
    await act(async () => { root.render(<SymbolPicker source="a_share" value="" onChange={() => undefined} />); });
    const trigger = host.querySelector<HTMLButtonElement>('[aria-label="交易对"]')!;
    await act(async () => trigger.click());
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await act(async () => trigger.click());
    resolve({ items: [{ symbol: 'STALE', name: 'Stale result', exchange: 'SSE' }], total: 1, offset: 0, has_more: false, universe_count: 30, status: 'warming', complete: true, symbols: ['STALE'], count: 1, source: 'a_share' });
    await flush();

    await act(async () => trigger.click());
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await flush();
    expect(listSymbols).toHaveBeenCalledTimes(2);
    expect(host.textContent).toContain('CURRENT');
  });
});
