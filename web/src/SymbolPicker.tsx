import { useEffect, useRef, useState } from 'react';
import { api } from './api';
import type { SourceType } from './types';

type SymbolItem = { symbol: string; name: string; exchange: string };
type Result = {
  items: SymbolItem[]; total: number; offset: number; has_more: boolean;
  universe_count: number; status: string; complete: boolean;
};
const catalogPageCache = new Map<string, Result>();
const cacheKey = (source: SourceType, query: string, page: number) => source + '|' + query.trim().toUpperCase() + '|' + page;

export function SymbolPicker({ source, value, onChange, label = '交易对' }: {
  source: SourceType; value: string; onChange: (symbol: string) => void; label?: string;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [page, setPage] = useState(0);
  const [result, setResult] = useState<Result | null>(null);
  const [error, setError] = useState('');
  const [refreshTick, setRefreshTick] = useState(0);
  const request = useRef(0);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => { setQuery(''); setPage(0); setResult(null); setError(''); }, [source]);

  useEffect(() => {
    if (!open) return;
    const id = ++request.current;
    const key = cacheKey(source, query, page);
    const cached = catalogPageCache.get(key);
    if (cached) {
      setResult(cached); setError('');
      if (cached.complete) return;
    }
    // Opening a list and changing a page are intent actions: do not add an
    // artificial debounce. Only a typed query is delayed to avoid bursts.
    const timer = window.setTimeout(() => {
      api.listSymbols(source, query, page * 50, 50).then(data => {
        if (id !== request.current) return;
        catalogPageCache.set(key, data);
        setResult(data); setError('');
        if (!data.complete && !query.trim()) window.setTimeout(() => setRefreshTick(tick => tick + 1), 750);
      }).catch(error => {
        if (id !== request.current) return;
        setError(String(error)); setResult(null);
      });
    }, query.trim() ? 180 : 0);
    return () => window.clearTimeout(timer);
  }, [open, source, query, page, refreshTick]);

  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => {
      if (root.current && !root.current.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', close);
    return () => document.removeEventListener('mousedown', close);
  }, [open]);

  const choose = (symbol: string) => {
    onChange(symbol); setQuery(''); setOpen(false);
  };
  const submit = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Escape') { setOpen(false); return; }
    if (event.key === 'Enter') {
      const exact = result?.items.find(item => item.symbol.toUpperCase() === query.trim().toUpperCase());
      if (exact) choose(exact.symbol);
      else if (query.trim()) choose(query.trim().toUpperCase());
    }
  };
  const state = result && !result.complete
    ? '完整目录正在后台更新；现在可搜索常用标的或直接输入代码。'
    : result ? '已检索 ' + result.total.toLocaleString() + ' 个匹配标的，目录共 ' + result.universe_count.toLocaleString() + ' 个。' : '';

  return (
    <div className={'ax-dropdown ax-symbol-picker' + (open ? ' open' : '')} ref={root}>
      <button type="button" className="ax-dd-trigger" aria-label={label} aria-haspopup="listbox" aria-expanded={open}
        onClick={() => setOpen(value => !value)}>
        <span>{value || '选择交易对…'}</span>
      </button>
      {open && (
        <div className="ax-dd-menu ax-symbol-menu" role="listbox" aria-label={`${label}搜索结果`}>
          <input className="ax-dd-search" aria-label={`搜索${label}`} value={query} placeholder="搜索代码或名称"
            autoFocus onChange={event => { setQuery(event.target.value); setPage(0); }}
            onKeyDown={submit} />
          <p className="ax-symbol-status">{error || state || '正在读取证券目录…'}</p>
          {result?.items.map(item => (
            <button type="button" role="option" aria-selected={item.symbol === value}
              className={'ax-symbol-option' + (item.symbol === value ? ' selected' : '')}
              key={item.symbol} onClick={() => choose(item.symbol)}>
              <strong>{item.symbol}</strong><span>{item.name}</span><em>{item.exchange}</em>
            </button>
          ))}
          {result && result.items.length === 0 && <p className="ax-symbol-empty">没有匹配标的；可按 Enter 使用输入的代码。</p>}
          {result && (result.offset > 0 || result.has_more) && (
            <div className="ax-symbol-pages">
              <button type="button" disabled={result.offset === 0} onClick={() => setPage(p => Math.max(0, p - 1))}>上一页</button>
              <span>{Math.floor(result.offset / 50) + 1} / {Math.max(1, Math.ceil(result.total / 50))}</span>
              <button type="button" disabled={!result.has_more} onClick={() => setPage(p => p + 1)}>下一页</button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
