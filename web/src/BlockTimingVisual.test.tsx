import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { BlockTimingVisual } from './BlockTimingVisual';
import type { PracticeResult } from './types';

const timestamps = [0, 600, 1200, 1000, 1000, 1600, 2200, 2800, 3400, 4000];
const hash = (index: number) => index.toString(16).padStart(64, '0');
const fixture = (conceptId: string): PracticeResult => ({
  concept_id: conceptId, status: 'computed', reason: null, input_kind: 'market_bars', provenance: 'server_fetched_bitcoin_block_snapshot', values: {}, units: {}, series: [], notes: [], module: 'data', source: 'binance', symbol: 'BTCUSDT', bars: [],
  block_snapshot: { network: 'bitcoin_mainnet', provider: 'Blockstream Esplora', endpoint: 'https://blockstream.info/api/blocks', fetched_at: '2026-09-23T01:00:00Z', first_height: 100, last_height: 109, observed_block_count: 10, first_hash: hash(1), last_hash: hash(10) },
  blocks: timestamps.map((seconds, index) => ({ height: 100 + index, hash: hash(index + 1), previous_hash: hash(index), timestamp: new Date((1_700_000_000 + seconds) * 1000).toISOString(), size_bytes: 1_000_000, tx_count: (index + 1) * 10 })),
  intervals: timestamps.slice(1).map((seconds, index) => ({ from_height: 100 + index, to_height: 101 + index, seconds: seconds - timestamps[index] })),
});

describe('BlockTimingVisual', () => {
  it('draws all nine signed header timestamp differences, including negative and zero', () => {
    const result = fixture('book_block_interval');
    result.values = { interval_count: 9, total_declared_span_seconds: 4000, mean_block_interval_seconds: 4000 / 9, median_block_interval_seconds: 600, nonpositive_interval_count: 2 };
    const html = renderToStaticMarkup(<BlockTimingVisual result={result} />);
    expect(html.match(/data-height=/g)).toHaveLength(9);
    expect(html).toContain('data-value="-200"');
    expect(html).toContain('data-value="0"');
    expect(html).toContain('九段头时间戳差合计 4,000 秒');
    expect(html).toContain('2 段不大于零');
    expect(html).toContain('不是实测出块耗时');
  });

  it('shows transaction counts for later blocks, excludes anchor, and avoids a misleading rate when undefined', () => {
    const result = fixture('book_transaction_rate');
    result.values = { confirmed_transaction_count: 540, elapsed_seconds: 4000, transaction_rate: 0.135, included_block_count: 9, nonpositive_interval_count: 2 };
    result.anchor_block_excluded = true;
    let html = renderToStaticMarkup(<BlockTimingVisual result={result} />);
    expect(html.match(/data-height=/g)).toHaveLength(9);
    expect(html).toContain('data-value="20"');
    expect(html).not.toContain('data-value="10"');
    expect(html).toContain('样本速率 0.135 笔/秒');
    expect(html).toContain('包含每块的 coinbase');
    result.status = 'undefined';
    result.values.transaction_rate = null;
    html = renderToStaticMarkup(<BlockTimingVisual result={result} />);
    expect(html).toContain('本次速率无法定义');
    expect(html).not.toContain('样本速率');
  });

  it('does not invent a timing figure without a complete linked window', () => {
    const result = fixture('book_block_interval');
    delete result.intervals;
    expect(renderToStaticMarkup(<BlockTimingVisual result={result} />)).toBe('');
    result.intervals = fixture('book_block_interval').intervals;
    result.blocks = result.blocks!.slice(1);
    expect(renderToStaticMarkup(<BlockTimingVisual result={result} />)).toBe('');
    result.blocks = fixture('book_block_interval').blocks;
    delete result.block_snapshot;
    expect(renderToStaticMarkup(<BlockTimingVisual result={result} />)).toBe('');
  });
});
