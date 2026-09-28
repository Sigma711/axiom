import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import type { PracticeResult } from './types';
import { AddressSetVisual } from './AddressSetVisual';

const blockHash = 'a'.repeat(64);
const fixture = (): PracticeResult => ({
  concept_id: 'book_sending_receiving', status: 'computed', reason: null, input_kind: 'market_bars',
  provenance: 'server_fetched_bitcoin_transaction_first_page', values: {}, units: {}, series: [], notes: [], module: 'data', source: 'bitcoin', symbol: 'BTC', bars: [],
  address_sample: { network: 'bitcoin_mainnet', provider: 'Blockstream Esplora', endpoint: 'https://blockstream.info/api/block/' + blockHash + '/txs/0', fetched_at: '2026-09-28T00:00:00Z', block_hash: blockHash, block_height: 840000, block_time: '2026-09-28T00:00:00Z', page_start: 0, returned_count: 3, excluded_coinbase_count: 1, sampled_noncoinbase_transaction_count: 2, scope: 'confirmed_pinned_block_first_page_noncoinbase_transactions', observed_newer_blocks: 6, confirmation_note: 'six newer blocks' },
  address_transactions: [
    { txid: '1'.repeat(64), input_addresses: ['bc1qin', 'bc1qshared'], output_addresses: ['bc1qout', 'bc1qshared'], missing_input_address_slots: 1, missing_output_address_slots: 0, excluded_op_return_output_count: 1 },
    { txid: '2'.repeat(64), input_addresses: ['bc1qsecond'], output_addresses: ['bc1qout2'], missing_input_address_slots: 0, missing_output_address_slots: 1, excluded_op_return_output_count: 0 },
  ],
});

describe('AddressSetVisual', () => {
  it('renders independently derived input, output, and overlap address sets', () => {
    const html = renderToStaticMarkup(<AddressSetVisual result={fixture()} />);
    expect(html).toContain('输入脚本中解出的地址');
    expect(html).toContain('输出脚本中解出的地址');
    expect(html).toContain('aria-label="两边都出现"');
    expect(html).toContain('aria-label="仅输入"');
    expect(html).toContain('aria-label="仅输出"');
    expect(html).toContain('合并去重 <b>5</b>');
    expect(html).toContain('缺少 1 个输入槽位、1 个输出槽位');
    expect(html).toContain('https://blockstream.info/block/' + blockHash);
    expect(html.match(/data-address=/g)).toHaveLength(5);
    expect(html.match(/data-address-txid=/g)).toHaveLength(2);
  });

  it('treats absent decoded addresses as unavailable evidence rather than a person or a zero address', () => {
    const result = fixture();
    result.address_transactions = [{ txid: '3'.repeat(64), input_addresses: [], output_addresses: [], missing_input_address_slots: 2, missing_output_address_slots: 3, excluded_op_return_output_count: 0 }];
    const html = renderToStaticMarkup(<AddressSetVisual result={result} />);
    expect(html).toContain('没有可解出的地址');
    expect(html).toContain('缺少 2 个输入槽位、3 个输出槽位');
    expect(html).toContain('地址不是人或实体');
  });

  it('does not render without address-sample metadata', () => {
    const result = fixture();
    delete result.address_sample;
    expect(renderToStaticMarkup(<AddressSetVisual result={result} />)).toBe('');
  });
  it('links back to the provider that actually supplied the block after failover', () => {
    const result = fixture();
    result.address_sample!.endpoint = `https://mempool.space/api/block/${blockHash}/txs/0`;
    result.address_sample!.provider = 'mempool Esplora';
    const html = renderToStaticMarkup(<AddressSetVisual result={result} />);
    expect(html).toContain(`https://mempool.space/block/${blockHash}`);
    expect(html).not.toContain('https://blockstream.info/block/');
  });
});
