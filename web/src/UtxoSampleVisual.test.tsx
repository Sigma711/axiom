import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import type { PracticeResult } from './types';
import { UtxoSampleVisual } from './UtxoSampleVisual';

const blockHash = 'a'.repeat(64);
const fixture = (concept_id: string): PracticeResult => ({
  concept_id, status: 'computed', reason: null, input_kind: 'market_bars', provenance: 'server_fetched_bitcoin_transaction_first_page',
  values: {}, units: {}, series: [], notes: [], module: 'data', source: 'binance', symbol: 'BTCUSDT', bars: [],
  utxo_sample: { network: 'bitcoin_mainnet', provider: 'Blockstream Esplora', endpoint: `https://blockstream.info/api/block/${blockHash}/txs/0`, fetched_at: '2026-09-23T01:00:00Z', block_hash: blockHash, block_height: 840000, block_time: '2026-09-23T00:55:00Z', page_start: 0, returned_count: 3, excluded_coinbase_count: 1, sampled_noncoinbase_transaction_count: 2, scope: 'confirmed_pinned_block_first_page_noncoinbase_transactions', observed_newer_blocks: 6, confirmation_note: 'six newer blocks', total_utxo_scope: 'undefined_not_derived_from_first_page_sample' },
  utxo_transactions: [
    { txid: '1'.repeat(64), input_prevout_values_sats: [20_000_000, 30_000_000], non_op_return_output_values_sats: [25_000_000, 20_000_000], spent_prevout_count: 2, spent_prevout_value_sats: 50_000_000, created_output_count: 3, created_output_value_sats: 45_000_000, created_non_op_return_output_count: 2, created_non_op_return_value_sats: 45_000_000, excluded_op_return_output_count: 1, excluded_op_return_output_value_sats: 0, unclassified_non_op_return_output_count: 0 },
    { txid: '2'.repeat(64), input_prevout_values_sats: [25_000_000, 25_000_000, 25_000_000], non_op_return_output_values_sats: [70_000_000], spent_prevout_count: 3, spent_prevout_value_sats: 75_000_000, created_output_count: 1, created_output_value_sats: 70_000_000, created_non_op_return_output_count: 1, created_non_op_return_value_sats: 70_000_000, excluded_op_return_output_count: 0, excluded_op_return_output_value_sats: 0, unclassified_non_op_return_output_count: 1 },
  ],
});

describe('UtxoSampleVisual', () => {
  it('shows output and spent-input counts while refusing a global total', () => {
    const result = fixture('book_utxo_counts');
    result.status = 'partial';
    result.values = { created_non_op_return_output_count: 3, spent_prevout_count: 5, total_utxo_count: null };
    const html = renderToStaticMarkup(<UtxoSampleVisual result={result} />);
    expect(html.match(/data-utxo-txid=/g)).toHaveLength(2);
    expect(html).toContain('data-spent="2" data-created="2"');
    expect(html).toContain('新建非 OP_RETURN 输出 3 个');
    expect(html).toContain('全网当前 UTXO 总数：无法从这页样本得出');
    expect(html).toContain(`https://blockstream.info/block/${blockHash}`);
  });

  it('shows exact sample value totals and keeps values separate from counts', () => {
    const result = fixture('book_utxo_totals');
    result.values = { created_non_op_return_value_sats: 115_000_000, spent_prevout_value_sats: 125_000_000, total_utxo_value_sats: null };
    const html = renderToStaticMarkup(<UtxoSampleVisual result={result} />);
    expect(html).toContain('1.15 BTC');
    expect(html).toContain('1.25 BTC');
    expect(html).toContain('全网当前 UTXO 总价值：无法从这页样本得出');
    expect(html).toContain('未知脚本类型不能保证可花费');
  });

  it('renders separate input and output value distributions and an empty page truthfully', () => {
    const result = fixture('book_utxo_value_stats');
    result.values = { created_mean_value_sats: 115_000_000 / 3, created_median_value_sats: 25_000_000, spent_mean_value_sats: 25_000_000, spent_median_value_sats: 25_000_000 };
    const html = renderToStaticMarkup(<UtxoSampleVisual result={result} />);
    expect(html).toContain('均值 0.38333333 BTC');
    expect(html).toContain('中位数 0.25 BTC');
    expect(html).toContain('花费输入：均值 0.25 BTC');
    result.utxo_transactions = [];
    expect(renderToStaticMarkup(<UtxoSampleVisual result={result} />)).toContain('没有普通交易');
    delete result.utxo_sample;
    expect(renderToStaticMarkup(<UtxoSampleVisual result={result} />)).toBe('');
  });
});
