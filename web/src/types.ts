// 共享类型定义
export type Side = 'BUY' | 'SELL' | 'HOLD';
export type TabId = 'learn' | 'data' | 'backtest' | 'paper' | 'compare';

export interface KnowledgeEntry {
  id: string;
  category: string;
  name: string;
  summary: string;
  formula: string;
  meaning: string;
  example: string;
  signals: string;
  pitfalls: string;
  related: string[];
  code_url: string;
  implementation: string;
}

export interface KnowledgeResponse {
  total: number;
  categories: Record<string, KnowledgeEntry[]>;
}

export interface StrategyMeta {
  name: string;
  display_name: string;
  description: string;
  params: Array<{
    key: string;
    label: string;
    default: number;
    min?: number;
    max?: number;
  }>;
}

export interface Bar {
  timestamp: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface EquityPoint {
  timestamp: string;
  cash: number;
  position_value: number;
  equity: number;
}

export interface Trade {
  symbol: string;
  side: 'BUY' | 'SELL';
  entry_time: string;
  exit_time: string | null;
  entry_price: number;
  exit_price: number | null;
  size: number;
  entry_commission: number;
  exit_commission: number;
  pnl: number;
  pnl_pct: number;
}

export interface BacktestResult {
  config: Record<string, unknown>;
  equity_curve: EquityPoint[];
  trades: Trade[];
  signals: Array<{ i: number; timestamp: string; side: string; strength: number; reason: string }>;
  fills: Array<{ timestamp: string; side: string; size: number; price: number; commission: number }>;
  metrics: Record<string, number | null>;
}

export interface PaperSnapshot {
  is_running: boolean;
  current_bar: Bar | null;
  cash: number;
  position_size: number;
  position_value: number;
  equity: number;
  last_signal: { timestamp: string; side: string; strength: number; reason: string } | null;
  last_fill: { timestamp: string; side: string; size: number; price: number; commission: number } | null;
  equity_curve: EquityPoint[];
  trades_count: number;
  log: Array<{ timestamp: string; level: string; message: string }>;
}

export interface CustomStrategy {
  name: string;
  base: string;
  params: Record<string, number>;
  sl: number;
}

export type ChartType = 'candle' | 'heikin_ashi';
export type SourceType = 'real' | 'synthetic';
