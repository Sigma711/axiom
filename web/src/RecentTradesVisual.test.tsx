import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { RecentTradesVisual } from './RecentTradesVisual';
import type { PracticeResult } from './types';

const base = (conceptId: string): PracticeResult => ({
  concept_id: conceptId, status: 'computed', reason: null, input_kind: 'market_bars', provenance: 'server_fetched_binance_recent_trades',
  values: {}, series: [], notes: [], module: 'data', source: 'binance', symbol: 'BTCUSDT', bars: [], asset_units: { base_asset: 'BTC', quote_asset: 'USDT' },
  recent_trades: { source: 'binance_usdt_spot', endpoint: 'https://data-api.binance.vision/api/v3/trades', requested_limit: 1000, trade_count: 4, analyzed_trade_count: 3, first_trade_id: 1, last_trade_id: 4, first_time: '2026-09-23T01:00:00Z', last_time: '2026-09-23T01:00:03Z', fetched_at: '2026-09-23T01:00:04Z', anchor_excluded: true, zero_tick_policy: 'equal_price_is_neutral', window_kind: 'recent_observed_trades' },
});

describe('RecentTradesVisual', () => {
  it('separates equal-price prints from up and down tick volume', () => {
    const result = base('book_net_volume');
    result.values = { uptick_volume: 2, downtick_volume: 1, neutral_volume: 3, net_volume: 1, analyzed_volume: 6 };
    const html = renderToStaticMarkup(<RecentTradesVisual result={result} />);
    expect(html).toContain('上涨 Tick');
    expect(html).toContain('下跌 Tick');
    expect(html).toContain('同价 Tick');
    expect(html).toContain('首笔只作前价锚点');
    expect(html).toContain('width:66.666');
  });

  it('renders every observed price level and identifies the POC and value area', () => {
    const result = base('volume_profile');
    result.values = { poc: 101, value_area_low: 100, value_area_high: 101, included_fraction: 0.8, total_volume: 10, price_level_count: 3 };
    result.profile_levels = [
      { price: 100, volume: 3, in_value_area: true, is_poc: false },
      { price: 101, volume: 5, in_value_area: true, is_poc: true },
      { price: 102, volume: 2, in_value_area: false, is_poc: false },
    ];
    const html = renderToStaticMarkup(<RecentTradesVisual result={result} />);
    expect(html.match(/data-price-level=/g)).toHaveLength(3);
    expect(html).toContain('class="poc"');
    expect(html).toContain('class="value-area"');
    expect(html).toContain('class="outside"');
    expect(html).toContain('覆盖 80%');
    expect(html).toContain('不是全天成交分布');
  });

  it('does not draw a window when its provenance is absent', () => {
    const result = base('book_net_volume');
    delete result.recent_trades;
    expect(renderToStaticMarkup(<RecentTradesVisual result={result} />)).toBe('');
  });
});
