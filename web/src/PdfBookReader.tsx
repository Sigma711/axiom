import { useCallback, useEffect, useRef, useState, type CSSProperties, type FormEvent } from 'react';
import { appPath } from './api';

type PdfDestination = string | unknown[] | null;
export type PdfOutlineSource = { title: string; dest: PdfDestination; items: PdfOutlineSource[] };
export type PdfOutlineEntry = { title: string; page: number | null; items: PdfOutlineEntry[] };
export type PdfRenderTask = { promise: Promise<unknown>; cancel(): void };
export type PdfPageProxy = {
  getViewport(options: { scale: number }): { width: number; height: number };
  render(options: { canvasContext: CanvasRenderingContext2D; viewport: unknown; transform?: number[] }): PdfRenderTask;
  cleanup(): void;
};
export type PdfBookDocument = {
  numPages: number;
  getOutline(): Promise<PdfOutlineSource[] | null>;
  getDestination(name: string): Promise<unknown[] | null>;
  getPageIndex(reference: unknown): Promise<number>;
  getPage(pageNumber: number): Promise<PdfPageProxy>;
  destroy(): Promise<void>;
};
export type PdfBookLoader = (url: string) => Promise<PdfBookDocument>;

type PdfJsModule = {
  GlobalWorkerOptions: { workerSrc: string };
  getDocument(options: { url: string }): { promise: Promise<unknown>; destroy(): Promise<void> };
};

export function createPdfBookLoader(importPdf: () => Promise<PdfJsModule> = () => import('pdfjs-dist')): PdfBookLoader {
  return async url => {
    const pdfjs = await importPdf();
    pdfjs.GlobalWorkerOptions.workerSrc = new URL('pdfjs-dist/build/pdf.worker.min.mjs', import.meta.url).toString();
    const task = pdfjs.getDocument({ url });
    const pdf = await task.promise as Omit<PdfBookDocument, 'destroy'>;
    return {
      numPages: pdf.numPages,
      getOutline: () => pdf.getOutline(),
      getDestination: name => pdf.getDestination(name),
      getPageIndex: reference => pdf.getPageIndex(reference),
      getPage: pageNumber => pdf.getPage(pageNumber),
      destroy: () => task.destroy(),
    };
  };
}

const defaultLoadDocument = createPdfBookLoader();

async function destinationPage(document: PdfBookDocument, destination: PdfDestination) {
  const explicit = typeof destination === 'string' ? await document.getDestination(destination) : destination;
  if (!explicit?.length) return null;
  const reference = explicit[0];
  if (Number.isInteger(reference)) return Number(reference) + 1;
  try { return (await document.getPageIndex(reference)) + 1; } catch { return null; }
}

export async function resolvePdfOutline(document: PdfBookDocument): Promise<PdfOutlineEntry[]> {
  const outline = await document.getOutline();
  const resolve = async (items: PdfOutlineSource[]): Promise<PdfOutlineEntry[]> => Promise.all(items.map(async item => ({
    title: item.title.trim() || '未命名书签',
    page: await destinationPage(document, item.dest),
    items: await resolve(item.items || []),
  })));
  return resolve(outline || []);
}

export function activeOutlinePage(entries: PdfOutlineEntry[], currentPage: number): number | null {
  let active: number | null = null;
  const visit = (items: PdfOutlineEntry[]) => items.forEach(item => {
    if (item.page != null && item.page <= currentPage && (active == null || item.page > active)) active = item.page;
    visit(item.items);
  });
  visit(entries);
  return active;
}

function PdfPage({ document, pageNumber, register, onVisible }: {
  document: PdfBookDocument;
  pageNumber: number;
  register: (page: number, node: HTMLElement | null) => void;
  onVisible: (page: number) => void;
}) {
  const hostRef = useRef<HTMLElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [visible, setVisible] = useState(false);
  const [width, setWidth] = useState(0);
  const [ratio, setRatio] = useState(1.414);
  const [renderError, setRenderError] = useState(false);
  const [renderAttempt, setRenderAttempt] = useState(0);

  const setHost = useCallback((node: HTMLElement | null) => {
    hostRef.current = node;
    register(pageNumber, node);
  }, [pageNumber, register]);

  useEffect(() => {
    const node = hostRef.current;
    if (!node || typeof IntersectionObserver === 'undefined') return;
    const preloadObserver = new IntersectionObserver(entries => {
      const entry = entries[0];
      setVisible(entry.isIntersecting);
    }, { root: node.closest('.ax-book-scroll'), rootMargin: '900px 0px' });
    const positionObserver = new IntersectionObserver(entries => {
      const entry = entries[0];
      if (entry.isIntersecting && entry.intersectionRatio >= .45) onVisible(pageNumber);
    }, { root: node.closest('.ax-book-scroll'), threshold: [.45, .7] });
    preloadObserver.observe(node);
    positionObserver.observe(node);
    return () => { preloadObserver.disconnect(); positionObserver.disconnect(); };
  }, [onVisible, pageNumber]);

  useEffect(() => {
    const node = hostRef.current;
    if (!node || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(entries => setWidth(Math.max(1, entries[0].contentRect.width)));
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!visible || width < 1 || !canvasRef.current) return;
    let disposed = false;
    let page: PdfPageProxy | undefined;
    let task: PdfRenderTask | undefined;
    setRenderError(false);
    void document.getPage(pageNumber).then(loadedPage => {
      if (disposed || !canvasRef.current) { loadedPage.cleanup(); return; }
      page = loadedPage;
      const base = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: width / base.width });
      const pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
      const canvas = canvasRef.current;
      canvas.width = Math.ceil(viewport.width * pixelRatio);
      canvas.height = Math.ceil(viewport.height * pixelRatio);
      canvas.style.width = `${viewport.width}px`;
      canvas.style.height = `${viewport.height}px`;
      setRatio(viewport.height / viewport.width);
      const context = canvas.getContext('2d', { alpha: false });
      if (!context) throw new Error('Canvas 2D context unavailable');
      task = page.render({ canvasContext: context, viewport, transform: pixelRatio === 1 ? undefined : [pixelRatio, 0, 0, pixelRatio, 0, 0] });
      return task.promise;
    }).catch(error => {
      if (!disposed && (error as Error).name !== 'RenderingCancelledException') setRenderError(true);
    });
    return () => { disposed = true; task?.cancel(); page?.cleanup(); };
  }, [document, pageNumber, renderAttempt, visible, width]);

  return <article ref={setHost} id={`pdf-page-${pageNumber}`} className="ax-pdf-sheet" aria-label={`第 ${pageNumber} 页`} style={{ aspectRatio: `1 / ${ratio}` }}>
    {visible && <canvas ref={canvasRef} />}
    {renderError && <div className="ax-pdf-page-error" role="alert"><strong>第 {pageNumber} 页渲染失败</strong><button type="button" onClick={() => setRenderAttempt(value => value + 1)}>重试这一页</button></div>}
    <span>{pageNumber}</span>
  </article>;
}

function OutlineList({ entries, activePage, depth = 0, goToPage }: { entries: PdfOutlineEntry[]; activePage: number | null; depth?: number; goToPage: (page: number) => void }) {
  return <ol>{entries.map((entry, index) => <li key={`${entry.title}-${entry.page ?? 'unknown'}-${index}`}>
    <button type="button" disabled={entry.page == null} className={entry.page === activePage ? 'active' : ''} style={{ '--outline-depth': depth } as CSSProperties} onClick={() => entry.page && goToPage(entry.page)}>
      <span>{entry.title}</span><small>{entry.page ? `第 ${entry.page} 页` : '无页码'}</small>
    </button>
    {entry.items.length > 0 && <OutlineList entries={entry.items} activePage={activePage} depth={depth + 1} goToPage={goToPage} />}
  </li>)}</ol>;
}

export function PdfBookReader({ loadDocument = defaultLoadDocument }: { loadDocument?: PdfBookLoader }) {
  const [tocOpen, setTocOpen] = useState(true);
  const [document, setDocument] = useState<PdfBookDocument | null>(null);
  const [outline, setOutline] = useState<PdfOutlineEntry[]>([]);
  const [currentPage, setCurrentPage] = useState(1);
  const [pageInput, setPageInput] = useState('1');
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const [retry, setRetry] = useState(0);
  const pageNodes = useRef(new Map<number, HTMLElement>());

  useEffect(() => {
    let cancelled = false;
    let loaded: PdfBookDocument | null = null;
    let destroyed = false;
    const destroy = async () => {
      if (loaded && !destroyed) { destroyed = true; await loaded.destroy(); }
    };
    setStatus('loading');
    setDocument(null);
    setOutline([]);
    void loadDocument(appPath('/api/book/pdf')).then(async pdf => {
      loaded = pdf;
      const entries = await resolvePdfOutline(pdf);
      if (cancelled) { await destroy(); return; }
      setDocument(pdf);
      setOutline(entries);
      setStatus('ready');
    }).catch(() => { if (!cancelled) setStatus('error'); });
    return () => { cancelled = true; void destroy(); };
  }, [loadDocument, retry]);

  const register = useCallback((page: number, node: HTMLElement | null) => {
    if (node) pageNodes.current.set(page, node); else pageNodes.current.delete(page);
  }, []);
  const markVisible = useCallback((page: number) => { setCurrentPage(page); setPageInput(String(page)); }, []);
  const goToPage = useCallback((page: number) => {
    if (!document) return;
    const target = Math.max(1, Math.min(document.numPages, Math.round(page)));
    setCurrentPage(target);
    setPageInput(String(target));
    const node = pageNodes.current.get(target);
    const scroll = node?.closest<HTMLElement>('.ax-book-scroll');
    if (node && scroll) scroll.scrollTo({ top: scroll.scrollTop + node.getBoundingClientRect().top - scroll.getBoundingClientRect().top - scroll.clientTop, behavior: 'smooth' });
  }, [document]);
  const submitPage = (event: FormEvent) => { event.preventDefault(); goToPage(Number(pageInput) || currentPage); };

  return <div className={`ax-book-reader${tocOpen ? '' : ' toc-collapsed'}`} data-ready={status === 'ready'}>
    <aside className={`ax-book-toc${tocOpen ? '' : ' collapsed'}`}>
      <button className="ax-book-toc-toggle" type="button" aria-expanded={tocOpen} aria-label={tocOpen ? '收起原书目录' : '展开原书目录'} title={tocOpen ? '收起目录' : '展开目录'} onClick={() => setTocOpen(open => !open)}><span aria-hidden="true">{tocOpen ? '‹' : '›'}</span></button>
      {tocOpen && <nav aria-label="原书目录"><div className="ax-book-toc-heading"><span>原书目录</span>{document && <small>{document.numPages} 页</small>}</div>
        {status === 'loading' && <p className="ax-book-status">正在读取原书与书签…</p>}
        {status === 'error' && <div className="ax-book-status error"><p>原书加载失败，请检查连接后重试。</p><button type="button" onClick={() => setRetry(value => value + 1)}>重新加载</button></div>}
        {status === 'ready' && (outline.length ? <OutlineList entries={outline} activePage={activeOutlinePage(outline, currentPage)} goToPage={goToPage} /> : <p className="ax-book-status">这份 PDF 没有内置书签，仍可连续翻阅全部页面。</p>)}
      </nav>}
    </aside>
    <section className="ax-book-page" aria-label="股票交易软件专业指标全解阅读器">
      <header className="ax-book-toolbar">
        <div><strong>股票交易软件专业指标全解</strong><small>{document ? `共 ${document.numPages} 页 · 连续阅读` : '正在准备阅读器'}</small></div>
        <form onSubmit={submitPage} aria-label="页码导航"><button type="button" disabled={!document || currentPage <= 1} onClick={() => goToPage(currentPage - 1)} aria-label="上一页">‹</button><label><span>页码</span><input aria-label="当前页码" inputMode="numeric" value={pageInput} onChange={event => setPageInput(event.target.value)} /></label><span>/ {document?.numPages ?? '—'}</span><button type="button" disabled={!document || currentPage >= document.numPages} onClick={() => goToPage(currentPage + 1)} aria-label="下一页">›</button></form>
      </header>
      <div className="ax-book-scroll" aria-busy={status === 'loading'}>
        {status === 'loading' && <div className="ax-book-loading"><i /><span>正在加载原书…</span></div>}
        {status === 'error' && <div className="ax-book-error"><strong>暂时无法显示原书</strong><span>使用左侧的“重新加载”再次尝试。</span></div>}
        {document && Array.from({ length: document.numPages }, (_, index) => <PdfPage key={index + 1} document={document} pageNumber={index + 1} register={register} onVisible={markVisible} />)}
      </div>
    </section>
  </div>;
}
