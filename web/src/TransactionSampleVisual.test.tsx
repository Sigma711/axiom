import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { TransactionSampleVisual } from './TransactionSampleVisual';
import type { PracticeResult } from './types';

const blockHash = 'a'.repeat(64);
const fixture = (conceptId: string): PracticeResult => ({
  concept_id: conceptId, status: 'computed', reason: null, input_kind: 'independent_inputs', provenance: 'server_fetched_bitcoin_transaction_sample',
  values: {}, units: {}, series: [], notes: [], module: 'data', source: 'binance', symbol: 'BTCUSDT', bars: [],
  transaction_sample: { network: 'bitcoin_mainnet', provider: 'Blockstream Esplora', endpoint: `https://blockstream.info/api/block/${blockHash}/txs/0`, fetched_at: '2026-09-23T01:00:00Z', block_hash: blockHash, block_height: 100, block_time: '2026-09-23T00:55:00Z', page_start: 0, returned_count: 3, analyzed_count: 2, excluded_coinbase_count: 1, scope: 'first_page_non_coinbase_transactions', observed_newer_blocks: 6, confirmation_note: 'six newer observed blocks' },
  transactions: [{ txid: '1'.repeat(64), fee_sats: 0, size_bytes: 120 }, { txid: '2'.repeat(64), fee_sats: 200, size_bytes: 280 }],
});

describe('TransactionSampleVisual', () => {
  it('renders exact fee amounts including zero fee and clarifies scope and units', () => {
    const result = fixture('book_transaction_fees');
    result.values = { sample_count: 2, total_fee_sats: 200, mean_fee_sats: 100, median_fee_sats: 100 };
    const html = renderToStaticMarkup(<TransactionSampleVisual result={result} />);
    expect(html.match(/data-txid=/g)).toHaveLength(2);
    expect(html).toContain('data-value="0"');
    expect(html).toContain('短横条代表 1 笔零手续费交易');
    expect(html).toContain('合计 200 sat，均值 100 sat，中位数 100 sat');
    expect(html).toContain('手续费金额不同于每虚拟字节费率');
    expect(html).toContain(`https://blockstream.info/block/${blockHash}`);
    expect(html).toContain('矿工的交易排序会影响结果');
  });

  it('renders actual serialized bytes rather than virtual bytes', () => {
    const result = fixture('book_transaction_bytes');
    result.values = { sample_count: 2, total_size_bytes: 400, mean_size_bytes: 200, median_size_bytes: 200 };
    const html = renderToStaticMarkup(<TransactionSampleVisual result={result} />);
    expect(html).toContain('data-value="120"');
    expect(html).toContain('data-value="280"');
    expect(html).toContain('合计 400 字节，均值 200 字节，中位数 200 字节');
    expect(html).toContain('实际字节不同于虚拟字节和区块权重');
  });

  it('shows an empty sample without fabricating transactions and handles invalid fetch time', () => {
    const result = fixture('book_transaction_fees');
    result.transactions = [];
    result.transaction_sample!.fetched_at = 'unknown';
    const html = renderToStaticMarkup(<TransactionSampleVisual result={result} />);
    expect(html).toContain('没有可分析的普通交易');
    expect(html).toContain('抓取于 unknown');
    expect(html).not.toContain('data-txid');
    expect(html).not.toContain('本页普通交易合计');
    delete result.transaction_sample;
    expect(renderToStaticMarkup(<TransactionSampleVisual result={result} />)).toBe('');
  });
});
