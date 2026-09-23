import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { BlockSnapshotVisual } from './BlockSnapshotVisual';
import type { PracticeResult } from './types';

const hash = (index: number) => index.toString(16).padStart(64, '0');
const fixture = (conceptId: string): PracticeResult => ({
  concept_id: conceptId, status: 'computed', reason: null, input_kind: 'independent_inputs', provenance: 'server_fetched_bitcoin_block_snapshot',
  values: {}, units: {}, series: [], notes: [], module: 'data', source: 'binance', symbol: 'BTCUSDT', bars: [],
  block_snapshot: { network: 'bitcoin_mainnet', provider: 'Blockstream Esplora', endpoint: 'https://blockstream.info/api/blocks', fetched_at: '2026-09-23T01:00:00Z', first_height: 100, last_height: 109, observed_block_count: 10, first_hash: hash(1), last_hash: hash(10) },
  blocks: Array.from({ length: 10 }, (_, index) => ({ height: 100 + index, hash: hash(index + 1), previous_hash: hash(index), timestamp: '2026-09-23T01:00:00Z', size_bytes: 1_000_000 + index * 100_000 })),
});

describe('BlockSnapshotVisual', () => {
  it('renders every linked observed block and makes the nine-interval distinction explicit', () => {
    const result = fixture('book_block_height');
    result.values = { latest_height: 109, reference_height: 100, blocks_since_reference: 9, observed_block_count: 10 };
    const html = renderToStaticMarkup(<BlockSnapshotVisual result={result} />);
    expect(html.match(/data-height=/g)).toHaveLength(10);
    expect(html).toContain('十个区块只有九段相邻关系');
    expect(html).toContain('100 → 最新块 109');
    expect(html).toContain('不是独立节点验证');
  });

  it('uses actual serialized bytes and a sample mean for the size chart', () => {
    const result = fixture('book_block_size');
    result.values = { total_size_bytes: 14_500_000, mean_size_bytes: 1_450_000, sample_block_count: 10 };
    const html = renderToStaticMarkup(<BlockSnapshotVisual result={result} />);
    expect(html.match(/data-size-bytes=/g)).toHaveLength(10);
    expect(html).toContain('14,500,000 字节');
    expect(html).toContain('1,450,000 字节');
    expect(html).toContain('虚拟字节和区块权重是不同口径');
  });

  it('omits a diagram without an observed snapshot', () => {
    const result = fixture('book_block_size');
    delete result.block_snapshot;
    expect(renderToStaticMarkup(<BlockSnapshotVisual result={result} />)).toBe('');
    result.block_snapshot = fixture('book_block_size').block_snapshot;
    result.blocks = [];
    expect(renderToStaticMarkup(<BlockSnapshotVisual result={result} />)).toBe('');
  });
});
