import { useEffect, useRef, useState } from 'react';

interface DropdownProps<T> {
  label: string;
  options: readonly { v: T; l: string }[];
  value: T;
  onChange: (v: T) => void;
  minWidth?: number;
}
export function Dropdown<T extends string | number>({ label, options, value, onChange, minWidth = 100 }: DropdownProps<T>) {
  const [open, setOpen] = useState(false);
  const [openUp, setOpenUp] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  useEffect(() => {
    if (open && options.length <= 5) {
      ref.current?.querySelector<HTMLButtonElement>('.ax-dd-item.selected')?.focus();
    }
  }, [open, options.length]);

  const current = options.find(o => o.v === value) || options[0];
  const toggle = () => {
    if (open) { setOpen(false); return; }
    const rect = ref.current?.getBoundingClientRect();
    const menuHeight = Math.min(360, options.length * 42 + (options.length > 5 ? 44 : 0) + 8);
    if (rect) setOpenUp(window.innerHeight - rect.bottom < menuHeight && rect.top > window.innerHeight - rect.bottom);
    setOpen(true);
  };

  const visibleOptions = () => Array.from(ref.current?.querySelectorAll<HTMLButtonElement>('.ax-dd-item') || [])
    .filter(item => item.style.display !== 'none');

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      setOpen(false);
      triggerRef.current?.focus();
      return;
    }
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    if (!open) {
      toggle();
      return;
    }
    const items = visibleOptions();
    if (!items.length) return;
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1
      : event.key === 'ArrowDown' ? (index + 1) % items.length
      : (index < 0 ? items.length - 1 : index - 1 + items.length) % items.length;
    items[next]?.focus();
  };

  return (
    <div className={'ax-dropdown' + (open ? ' open' : '') + (openUp ? ' open-up' : '')} ref={ref} style={{ minWidth }} onKeyDown={onKeyDown}>
      <button ref={triggerRef} type="button" className="ax-dd-trigger" aria-label={label} aria-haspopup="listbox" aria-expanded={open} onClick={toggle}>
        <span>{current?.l || '选择...'}</span>
      </button>
      {open && (
        <div className="ax-dd-menu" role="listbox" aria-label={label}>
          {options.length > 5 && (
            <input className="ax-dd-search" placeholder="搜索..." autoFocus
              onChange={e => {
                const q = e.target.value.toLowerCase();
                e.target.dataset.filtered = q;
                const items = (e.target.parentElement?.querySelectorAll('.ax-dd-item') || []) as NodeListOf<HTMLElement>;
                items.forEach(it => {
                  const label = (it.textContent || '').toLowerCase();
                  (it as HTMLElement).style.display = label.includes(q) ? '' : 'none';
                });
              }}
            />
          )}
          {options.map(o => (
            <button
              type="button"
              role="option"
              tabIndex={-1}
              aria-selected={o.v === value}
              key={String(o.v)}
              className={'ax-dd-item' + (o.v === value ? ' selected' : '')}
              onClick={() => { onChange(o.v); setOpen(false); triggerRef.current?.focus(); }}
            >
              {o.l}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

