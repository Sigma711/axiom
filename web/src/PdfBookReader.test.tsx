// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { activeOutlinePage, createPdfBookLoader, PdfBookReader, resolvePdfOutline, type PdfBookDocument, type PdfRenderTask } from './PdfBookReader';

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = async () => { await act(async () => { await Promise.resolve(); await Promise.resolve(); }); };

function documentFixture(overrides: Partial<PdfBookDocument> = {}): PdfBookDocument {
  return {
    numPages: 83,
    getOutline: vi.fn(async () => [
      { title: '使用说明', dest: [{ num: 8, gen: 0 }], items: [] },
      { title: '第一部分 行情、K线与微观结构', dest: 'chapter-one', items: [
        { title: 'K线基础', dest: [{ num: 12, gen: 0 }], items: [] },
      ] },
    ]),
    getDestination: vi.fn(async (name: string) => name === 'chapter-one' ? [{ num: 10, gen: 0 }] : null),
    getPageIndex: vi.fn(async (ref: { num: number }) => ref.num - 1),
    getPage: vi.fn(),
    destroy: vi.fn(async () => undefined),
    ...overrides,
  };
}

describe('PDF book public behavior', () => {
  let host: HTMLDivElement;
  beforeEach(() => {
    host = document.createElement('div');
    document.body.append(host);
    HTMLElement.prototype.scrollIntoView = vi.fn();
    HTMLElement.prototype.scrollTo = vi.fn();
  });
  afterEach(() => { host.remove(); vi.restoreAllMocks(); });

  it('maps the PDF own nested outline to real one-based pages', async () => {
    const outline = await resolvePdfOutline(documentFixture());
    expect(outline).toEqual([
      { title: '使用说明', page: 8, items: [] },
      { title: '第一部分 行情、K线与微观结构', page: 10, items: [
        { title: 'K线基础', page: 12, items: [] },
      ] },
    ]);
    expect(activeOutlinePage(outline, 11)).toBe(10);
    expect(activeOutlinePage(outline, 12)).toBe(12);
    expect(activeOutlinePage(outline, 7)).toBeNull();
  });

  it('configures the bundled worker and opens only the requested same-origin URL', async () => {
    const pdf = documentFixture();
    const worker = { workerSrc: '' };
    const destroy = vi.fn(async () => undefined);
    const getDocument = vi.fn(() => ({ promise: Promise.resolve(pdf), destroy }));
    const load = createPdfBookLoader(async () => ({ GlobalWorkerOptions: worker, getDocument }));
    const loaded = await load('/api/book/pdf');
    expect(loaded.numPages).toBe(pdf.numPages);
    expect(await loaded.getOutline()).toEqual(await pdf.getOutline());
    await loaded.destroy();
    expect(destroy).toHaveBeenCalledOnce();
    expect(pdf.destroy).not.toHaveBeenCalled();
    expect(getDocument).toHaveBeenCalledWith({ url: '/api/book/pdf' });
    expect(worker.workerSrc).toMatch(/pdf\.worker\.min\.mjs/);
  });

  it('loads the local book, exposes its real outline, and destroys it on unmount', async () => {
    const pdf = documentFixture();
    const load = vi.fn(async () => pdf);
    const root = createRoot(host);
    await act(async () => { root.render(<PdfBookReader loadDocument={load} />); });
    await flush();
    expect(load).toHaveBeenCalledWith('/api/book/pdf');
    expect(host.textContent).toContain('第一部分 行情、K线与微观结构');
    expect(host.textContent).toContain('第 10 页');
    const chapter = [...host.querySelectorAll('button')].find(button => button.textContent?.includes('第一部分 行情、K线与微观结构'))!;
    await act(async () => chapter.click());
    expect(HTMLElement.prototype.scrollTo).toHaveBeenCalled();
    expect(HTMLElement.prototype.scrollIntoView).not.toHaveBeenCalled();
    expect(host.querySelector<HTMLInputElement>('[aria-label="当前页码"]')?.value).toBe('10');
    await act(async () => root.unmount());
    expect(pdf.destroy).toHaveBeenCalledOnce();
  });

  it('shows a useful empty-outline state and lets the reader collapse it', async () => {
    const pdf = documentFixture({ getOutline: vi.fn(async () => null) });
    const root = createRoot(host);
    await act(async () => { root.render(<PdfBookReader loadDocument={async () => pdf} />); });
    await flush();
    expect(host.textContent).toContain('这份 PDF 没有内置书签');
    const collapse = host.querySelector<HTMLButtonElement>('[aria-label="收起原书目录"]')!;
    await act(async () => collapse.click());
    expect(host.querySelector('[aria-label="原书目录"]')).toBeNull();
    expect(host.querySelector('[aria-label="展开原书目录"]')).not.toBeNull();
    await act(async () => root.unmount());
  });

  it('destroys exactly once when unmounted while outline parsing is still pending', async () => {
    let finishOutline: (value: null) => void = () => undefined;
    const outline = new Promise<null>(resolve => { finishOutline = resolve; });
    const pdf = documentFixture({ getOutline: vi.fn(() => outline) });
    const root = createRoot(host);
    await act(async () => { root.render(<PdfBookReader loadDocument={async () => pdf} />); await Promise.resolve(); });
    await act(async () => root.unmount());
    finishOutline(null);
    await flush();
    expect(pdf.destroy).toHaveBeenCalledOnce();
  });

  it('recovers from a failed load when the user retries', async () => {
    const pdf = documentFixture();
    const load = vi.fn()
      .mockRejectedValueOnce(new Error('bad pdf'))
      .mockResolvedValueOnce(pdf);
    const root = createRoot(host);
    await act(async () => { root.render(<PdfBookReader loadDocument={load} />); });
    await flush();
    expect(host.firstElementChild?.getAttribute('data-ready')).toBe('false');
    expect(host.textContent).toContain('原书加载失败');
    const retry = [...host.querySelectorAll('button')].find(button => button.textContent === '重新加载')!;
    await act(async () => retry.click());
    await flush();
    expect(load).toHaveBeenCalledTimes(2);
    expect(host.textContent).toContain('共 83 页');
    await act(async () => root.unmount());
  });

  it('cancels an in-flight page render and shows a retry when rendering fails', async () => {
    class VisibleObserver {
      private callback: IntersectionObserverCallback;
      constructor(callback: IntersectionObserverCallback) { this.callback = callback; }
      observe(target: Element) { this.callback([{ target, isIntersecting: true, intersectionRatio: .8 } as IntersectionObserverEntry], this as unknown as IntersectionObserver); }
      disconnect() {}
      unobserve() {}
      takeRecords() { return []; }
      root = null; rootMargin = '0px'; thresholds = [0];
    }
    class SizedObserver {
      private callback: ResizeObserverCallback;
      constructor(callback: ResizeObserverCallback) { this.callback = callback; }
      observe(target: Element) { this.callback([{ target, contentRect: { width: 600 } } as unknown as ResizeObserverEntry], this as unknown as ResizeObserver); }
      disconnect() {}
      unobserve() {}
    }
    vi.stubGlobal('IntersectionObserver', VisibleObserver);
    vi.stubGlobal('ResizeObserver', SizedObserver);
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({} as CanvasRenderingContext2D);
    const cancel = vi.fn();
    let rejectRender: (error: Error) => void = () => undefined;
    const failedTask: PdfRenderTask = { promise: new Promise((_, reject) => { rejectRender = reject; }), cancel };
    const retryTask: PdfRenderTask = { promise: new Promise(() => undefined), cancel };
    const render = vi.fn().mockReturnValueOnce(failedTask).mockReturnValue(retryTask);
    const cleanup = vi.fn();
    const pdf = documentFixture({ numPages: 1, getPage: vi.fn(async () => ({
      getViewport: ({ scale }: { scale: number }) => ({ width: 600 * scale, height: 840 * scale }),
      render,
      cleanup,
    })) });
    const root = createRoot(host);
    await act(async () => { root.render(<PdfBookReader loadDocument={async () => pdf} />); });
    await flush();
    await act(async () => rejectRender(new Error('damaged page')));
    await flush();
    expect(host.textContent).toContain('第 1 页渲染失败');
    expect(host.textContent).toContain('重试这一页');
    const retry = [...host.querySelectorAll('button')].find(button => button.textContent === '重试这一页')!;
    await act(async () => retry.click());
    await flush();
    expect(render).toHaveBeenCalledTimes(2);
    expect(host.textContent).not.toContain('第 1 页渲染失败');
    await act(async () => root.unmount());
    expect(cancel).toHaveBeenCalled();
    expect(cleanup).toHaveBeenCalled();
    vi.unstubAllGlobals();
  });
});
