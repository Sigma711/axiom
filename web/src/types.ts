// 共享类型定义
import type { ExecutionAssumption, MarketProvenance } from './ExecutionAssumptionsVisual';
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
  execution_assumptions?: ExecutionAssumption[];
  market_provenance?: MarketProvenance;
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
  execution_assumptions?: ExecutionAssumption[];
  market_provenance?: MarketProvenance;
}

export interface CustomStrategy {
  name: string;
  base: string;
  params: Record<string, number>;
  sl: number;
}

export type ChartType = 'candle' | 'heikin_ashi';
export type SourceType = 'binance' | 'a_share' | 'us_stock';
export type PracticeRouteSource = SourceType | 'issuer_disclosure';

export interface PracticeConcept {
  id: string;
  name: string;
  category: string;
  input_kind: 'market_bars' | 'independent_inputs' | 'manual_annotation' | 'filing_case' | 'industry_case';
  inputs: Array<{ key: string; label: string; default: unknown }>;
  notes: string;
  plan?: {
    markets: Array<'crypto' | 'cn_equity' | 'us_equity' | 'issuer_disclosure'>;
    modules: Array<'data' | 'backtest' | 'paper' | 'compare'>;
    required_datasets: string[];
    source_policy: 'real_required' | 'result_required' | 'evidence_required';
    goal: string;
    fixed_source?: 'issuer_disclosure';
    fixed_symbol?: string;
  };
}

export interface PracticeResult {
  concept_id: string;
  status: 'computed' | 'partial' | 'undefined' | 'insufficient_data';
  reason: string | null;
  input_kind: PracticeConcept['input_kind'];
  provenance: 'provided_market_bars' | 'provided_result_context' | 'editable_teaching_inputs' | 'server_fetched_provisional_snapshot' | 'server_fetched_completed_binance_usdt_spot_1h_klines' | 'server_fetched_binance_spot_order_book' | 'server_fetched_completed_stock_daily_bars' | 'server_fetched_binance_recent_trades' | 'server_fetched_bitcoin_block_snapshot' | 'server_fetched_bitcoin_transaction_sample' | 'server_fetched_bitcoin_transaction_first_page' | 'verified_issuer_filing_case' | 'verified_original_issuer_disclosure';
  filing_case?: {
    case_id: string; issuer: string; ticker: string; scope: string;
    period: { start: string; end: string; fiscal_year: number };
    comparison_period: { start: string; end: string; fiscal_year: number };
    published: string; audited: false; status: string; source_kind: string;
    url: string; sha256: string; bytes: number;
    pages: { income_statement: number; balance_sheet: number; cash_flow: number };
    verification: { cache_status: string; verified_at: string; requested_url: string; matched_sha256: string; matched_bytes: number };
  };
  facts?: {
    units: Record<string, string>;
    annual_income_statement: Record<string, Record<string, number | string>>;
    balance_sheets: Record<string, Record<string, number | string>>;
    annual_cash_flows: Record<string, Record<string, number | string>>;
    calculation_boundaries: unknown;
  };
  industry_case?: {
    case_id: string;
    issuer: { name: string; ticker: string; alternate_tickers: string[]; reporting_entity: string; metric_entity: string };
    period: { label: string; start: string; end: string };
    published: string;
    audited: boolean;
    audit_boundary: string;
    status: string;
    source_kind: string;
    url: string;
    pdf_pages: number[];
    sha256: string;
    bytes: number;
    verification: { status: 'verified_immutable_cache' | 'verified_then_cached'; verified_at: string; requested_url: string; matched_sha256: string; matched_bytes: number };
  };
  industry_facts?: {
    currency: string;
    scale: string;
    reported_facts: Array<{ key: string; label: string; value: number; unit: string; pdf_page: number }>;
    calculation: { formula: string; result_key: string; operands: string[]; symbol_mapping: Record<string, string> };
    field_provenance: Record<string, { kind: 'reported' | 'derived'; pdf_page?: number; note: string }>;
    definitions: { case_boundary: string; metric: string };
  };
  values: Record<string, number | null>;
  units?: Record<string, string>;
  series: Array<{ name: string; unit?: string; values: Array<number | null> }>;
  chart?: { kind: string; source?: string; input?: string; source_price?: 'close' | 'ohlc'; source_bar_count?: number; bars: Array<{ open: number; high: number; low: number; close: number; direction?: number; column?: number; line_style?: 'neutral' | 'yin' | 'yang'; switch_price?: number | null }> };
  notes: string[];
  module: string;
  source: string;
  symbol: string;
  bars: Bar[];
  second_bars?: Bar[];
  pair?: { first_symbol: string; second_symbol: string; interval: string; quote_asset: string; matched_count: number; dropped_first: number; dropped_second: number; start: string; end: string; completion_cutoff: string };
  year_range?: { window_start: string; window_end: string; as_of: string; bar_count: number; price_basis: string; source: string; pre_window_observation: string; window_start_inclusive: false };
  recent_trades?: { source: string; endpoint: string; requested_limit: number; trade_count: number; analyzed_trade_count: number; first_trade_id: number; last_trade_id: number; first_time: string; last_time: string; fetched_at: string; anchor_excluded: boolean; zero_tick_policy: string; window_kind: string };
  trades?: Array<{ id: number; price: number; quantity: number; timestamp: string }>;
  profile_levels?: Array<{ price: number; volume: number; in_value_area: boolean; is_poc: boolean }>;
  block_snapshot?: { network: 'bitcoin_mainnet'; provider: string; endpoint: string; fetched_at: string; first_height: number; last_height: number; observed_block_count: number; first_hash: string; last_hash: string };
  blocks?: Array<{ height: number; hash: string; previous_hash: string; timestamp: string | number; size_bytes: number; tx_count?: number }>;
  intervals?: Array<{ from_height: number; to_height: number; seconds: number }>;
  anchor_block_excluded?: boolean;
  transaction_sample?: { network: 'bitcoin_mainnet'; provider: string; endpoint: string; fetched_at: string; block_hash: string; block_height: number; block_time: string | number; page_start: 0; returned_count: number; analyzed_count: number; excluded_coinbase_count: number; scope: 'first_page_non_coinbase_transactions'; observed_newer_blocks: number; confirmation_note: string };
  transactions?: Array<{ txid: string; fee_sats: number; size_bytes: number }>;
  utxo_sample?: { network: 'bitcoin_mainnet'; provider: string; endpoint: string; fetched_at: string; block_hash: string; block_height: number; block_time: string; page_start: 0; returned_count: number; excluded_coinbase_count: number; sampled_noncoinbase_transaction_count: number; scope: 'confirmed_pinned_block_first_page_noncoinbase_transactions'; observed_newer_blocks: number; confirmation_note: string; total_utxo_scope: 'undefined_not_derived_from_first_page_sample' };
  address_sample?: { network: 'bitcoin_mainnet'; provider: string; endpoint: string; fetched_at: string; block_hash: string; block_height: number; block_time: string; page_start: 0; returned_count: number; excluded_coinbase_count: number; sampled_noncoinbase_transaction_count: number; scope: 'confirmed_pinned_block_first_page_noncoinbase_transactions'; observed_newer_blocks: number; confirmation_note: string; address_semantics?: { sending: string; receiving: string; receiving_includes_change: boolean; identity_inference: 'not_supported' } };
  address_transactions?: Array<{ txid: string; input_addresses: string[]; output_addresses: string[]; missing_input_address_slots: number; missing_output_address_slots: number; excluded_op_return_output_count: number }>;
  utxo_transactions?: Array<{ txid: string; input_prevout_values_sats: number[]; non_op_return_output_values_sats: number[]; spent_prevout_count: number; spent_prevout_value_sats: number; created_output_count: number; created_output_value_sats: number; created_non_op_return_output_count: number; created_non_op_return_value_sats: number; excluded_op_return_output_count: number; excluded_op_return_output_value_sats: number; unclassified_non_op_return_output_count: number }>;
  asset_units?: { base_asset: string; quote_asset: string };
  levels?: { bids: Array<{ price: number; quantity: number }>; asks: Array<{ price: number; quantity: number }> };
  depth_snapshot?: { source: string; endpoint: string; symbol: string; depth_limit: number; update_id: number; timestamp: null; timestamp_note: string };
  provisional_snapshot?: { candle: Bar; is_closed: false; fetched_at: string; expected_close_at: string; completion_evidence: string };
  context?: 'module_snapshot' | 'selected_dataset' | 'editable_teaching_inputs' | 'provided_result_context' | 'historical_filing_case' | 'historical_industry_disclosure';
  bar_origin?: 'server_fetched_completed_source_bars' | 'server_fetched_completed_binance_usdt_spot_bars' | 'server_fetched_completed_binance_usdt_spot_1h_klines' | 'server_fetched_binance_provisional_snapshot' | 'server_verified_issuer_filing_pdf';
  execution_assumptions?: ExecutionAssumption[];
  market_provenance?: MarketProvenance;
}
