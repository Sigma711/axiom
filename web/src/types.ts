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
  code_ref?: string;
  source_refs?: Array<{ source_id: string; title: string; pdf_page: number }>;
  /** Optional teaching inputs supplied by newer knowledge payloads. */
  inputs?: PracticeConcept['inputs'];
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
  metrics: Record<string, number | null | Record<string, string>>;
  bars?: Bar[];
}

export interface PaperSnapshot {
  strategy?: string;
  completed_trades_count?: number;
  symbol?: string;
  source?: SourceType;
  initial_capital?: number;
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
  bars?: Bar[];
}

export interface CustomStrategy {
  name: string;
  base: string;
  params: Record<string, number>;
  sl: number;
}

export type ChartType = 'candle' | 'heikin_ashi';
export type SourceType = 'binance' | 'a_share' | 'us_stock';

export interface PracticeConcept {
  id: string;
  name: string;
  category: string;
  input_kind: 'market_bars' | 'independent_inputs' | 'manual_annotation';
  inputs: Array<{ key: string; label: string; default: unknown }>;
  notes: string;
  plan?: {
    markets: Array<'crypto' | 'cn_equity' | 'us_equity'>;
    modules: Array<'data' | 'backtest' | 'paper' | 'compare'>;
    required_datasets: string[];
    source_policy: 'real_required' | 'result_required' | 'evidence_required';
    goal: string;
  };
}

export interface PracticeResult {
  concept_id: string;
  status: 'computed' | 'undefined';
  reason: string | null;
  input_kind: PracticeConcept['input_kind'];
  provenance: 'provided_market_bars' | 'provided_result_context' | 'editable_teaching_inputs' | 'server_fetched_provisional_snapshot';
  values: Record<string, number | null>;
  units?: Record<string, string>;
  series: Array<{ name: string; values: Array<number | null> }>;
  chart?: { kind: string; source?: string; input?: string; source_price?: 'close' | 'ohlc'; source_bar_count?: number; bars: Array<{ open: number; high: number; low: number; close: number; direction?: number; column?: number; line_style?: 'neutral' | 'yin' | 'yang'; switch_price?: number | null }> };
  notes: string[];
  module: string;
  source: string;
  symbol: string;
  bars: Bar[];
  asset_units?: { base_asset: string; quote_asset: string };
  provisional_snapshot?: { candle: Bar; is_closed: false; fetched_at: string; expected_close_at: string; completion_evidence: string };
  context?: 'module_snapshot' | 'selected_dataset' | 'editable_teaching_inputs' | 'provided_result_context';
  bar_origin?: 'server_fetched_completed_source_bars' | 'server_fetched_completed_binance_usdt_spot_bars' | 'server_fetched_binance_provisional_snapshot';
}
