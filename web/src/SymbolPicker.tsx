import { useEffect, useRef, useState } from 'react';
import { api } from './api';
import type { SourceType } from './types';

type SymbolItem = { symbol: string; name: string; exchange: string };
type Result = {
  items: SymbolItem[]; total: number; offset: number; has_more: boolean;
  universe_count: number; status: string; complete: boolean;
};

export function SymbolPicker({ source, value, onChange }: {
  source: SourceType; value: string; onChange: (symbol: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [page, setPage] = useState(0);
  const [result, setResult] = useState<Result | null>(null);
  const [error, setError] = useState('');
  const request = useRef(0);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => { setQuery(''); setPage(0); }, [source]);

  useEffect(() => {
    if (!open) return;
    const id = ++request.current;
    const timer = window.setTimeout(() => {
      api.listSymbols(source, query, page * 50, 50).then(data => {
        if (id !== request.current) return;
        setResult(data); setError('');
      }).catch(error => {
        if (id !== request.current) return;
        setError(String(error)); setResult(null);
      });
    }, 220);
    return () => window.clearTimeout(timer);
  }, [open, source, query, page]);

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
    ? '目录临时不可用；仍可输入交易代码。'
    : result ? '已检索 ' + result.total.toLocaleString() + ' 个匹配标的，目录共 ' + result.universe_count.toLocaleString() + ' 个。' : '';

  return (
    <div className="ax-symbol-picker" ref={root}>
      <div className="ax-symbol-input-wrap">
        <input aria-label="搜索交易对" value={query} placeholder={value || '搜代码或名称'}
          onFocus={() => setOpen(true)}
          onChange={event => { setQuery(event.target.value); setPage(0); setOpen(true); }}
          onKeyDown={submit} />
        {value && <span className="ax-symbol-current" title={value}>已选 {value}</span>}
      </div>
      {open && (
        <div className="ax-symbol-menu" role="listbox" aria-label="交易对搜索结果">
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
