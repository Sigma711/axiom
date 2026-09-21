// API 客户端
import type {
  KnowledgeResponse,
  StrategyMeta,
  BacktestResult,
  PaperSnapshot,
  Bar,
  SourceType,
  PracticeConcept,
  PracticeResult,
} from './types';

export const appBase = (import.meta.env.VITE_APP_BASE ?? '').replace(/\/+$/, '');
export const appPath = (path: string) => `${appBase}${path}`;
const BASE = appBase;  // 同源或受部署子路径约束

export type CodeLocation = { url?: string; github_url?: string; source_url?: string; path: string; line: number; end_line?: number; };
export function preferredCodeLocationUrl(location: CodeLocation): string {
  return location.github_url || location.url || location.source_url || '';
}

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  const opts: RequestInit = {
    method,
    headers: { 'Content-Type': 'application/json' },
  };
  if (body) opts.body = JSON.stringify(body);
  let r: Response;
  try {
    r = await fetch(BASE + path, opts);
  } catch (e) {
    throw new Error('Network error: ' + (e as Error).message);
  }
  if (!r.ok) {
    let detail = '';
    try { detail = (await r.text()).slice(0, 300); } catch {}
    throw new Error(`HTTP ${r.status}: ${detail}`);
  }
  return r.json();
}

export const api = {
  listStrategies: () => call<{ strategies: StrategyMeta[] }>('GET', '/api/strategies'),
  listSymbols: () => call<{ symbols: string[]; count: number; source: string }>('GET', '/api/symbols'),
  getConfig: () => call<Record<string, unknown>>('GET', '/api/config'),
  listKnowledge: () => call<KnowledgeResponse>('GET', '/api/knowledge'),
  getData: (symbol: string, limit: number, source: SourceType) =>
    call<{ bars: Bar[]; count: number; symbol: string; source: string }>(
      'GET', `/api/data?symbol=${encodeURIComponent(symbol)}&limit=${limit}&source=${source}`),
  getIndicators: (symbol: string, indicators: string, limit: number, source: SourceType) =>
    call<{ bars: Bar[]; indicators: Record<string, Array<{ x: string; y: number } | null>>; symbol: string; source: string }>(
      'GET', `/api/indicators?symbol=${encodeURIComponent(symbol)}&indicators=${encodeURIComponent(indicators)}&limit=${limit}&source=${source}`),
  getPatterns: (symbol: string, limit: number, source: SourceType) =>
    call<{ symbol: string; patterns: Array<{ timestamp: string; close: number; pattern: string; pattern_code: string }> }>(
      'GET', `/api/patterns?symbol=${encodeURIComponent(symbol)}&limit=${limit}&source=${source}`),
  getHeikinAshi: (symbol: string, limit: number, source: SourceType) =>
    call<{ symbol: string; bars: Bar[]; chart: string }>(
      'GET', `/api/heikin_ashi?symbol=${encodeURIComponent(symbol)}&limit=${limit}&source=${source}`),
  getCodeLocation: (ref: string) => call<CodeLocation>('GET', `/api/code_loc?ref=${encodeURIComponent(ref)}`),
  runBacktest: (req: {
    strategy: string; params?: Record<string, number>;
    symbol: string; source: SourceType; limit: number; initial_capital: number;
    stop_loss_pct?: number; take_profit_pct?: number; max_position_pct?: number;
    bars?: Bar[];
  }) => call<BacktestResult>('POST', '/api/backtest', req),
  paperSnapshot: () => call<PaperSnapshot>('GET', '/api/paper/snapshot'),
  paperStart: () => call<{ status: string }>('POST', '/api/paper/start'),
  paperStop: () => call<{ status: string }>('POST', '/api/paper/stop'),
  paperStrategy: (strategy: string) =>
    call<{ status: string; strategy: string }>('POST', '/api/paper/strategy', { strategy }),
  listPractice: () => call<{ concepts: PracticeConcept[]; modules: string[]; total: number }>('GET', '/api/practice'),
  runPractice: (req: {
    concept_id: string; module: 'data' | 'backtest' | 'paper' | 'compare'; symbol: string; source: SourceType; limit: number;
    inputs: Record<string, unknown>; bars?: Bar[];
  }) => call<PracticeResult>('POST', '/api/practice', req),
};

// 工具函数
export function fmtPct(v: number | null | undefined, d = 2): string {
  if (v == null) return '—';
  return (v * 100).toFixed(d) + '%';
}
export function fmtNum(v: number | null | undefined, d = 2): string {
  if (v == null) return '—';
  return Number(v).toFixed(d);
}
export function fmtMoney(v: number | null | undefined): string {
  if (v == null) return '—';
  return '$' + Number(v).toLocaleString(undefined, { maximumFractionDigits: 2 });
}
export function fmtPctSigned(v: number, d = 2): string {
  return (v * 100 >= 0 ? '+' : '') + (v * 100).toFixed(d) + '%';
}
