import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it } from 'vitest';
import { SpotDepthVisual } from './SpotDepthVisual';
import type { PracticeResult } from './types';

const snapshot = (concept_id: string, values: Record<string, number | null>): PracticeResult => ({
  concept_id, input_kind: 'market_bars', status: 'computed', reason: null,
  provenance: 'server_fetched_binance_spot_order_book', values, units: {}, series: [], notes: [],
  module: 'data', source: 'binance', symbol: 'BTCUSDT', bars: [], asset_units: { base_asset: 'BTC', quote_asset: 'USDT' },
  depth_snapshot: { source: 'binance_usdt_spot', endpoint: 'https://data-api.binance.vision/api/v3/depth', symbol: 'BTCUSDT', depth_limit: 5, update_id: 123, timestamp: null, timestamp_note: 'No exchange timestamp' },
  levels: { bids: [{ price: 100, quantity: 3 }, { price: 99, quantity: 1 }], asks: [{ price: 101, quantity: 1 }, { price: 102, quantity: 2 }] },
});

it('draws a single observed book with bid and ask bars scaled by the same quantity', () => {
  const html = renderToStaticMarkup(<SpotDepthVisual result={snapshot('book_order_imbalance', { best_bid: 100, best_ask: 101, top_n: 2, top_n_bid_quantity: 4, top_n_ask_quantity: 3, order_imbalance: 1 / 7 })} />);
  expect(html).toContain('买盘 · Bid');
  expect(html).toContain('卖盘 · Ask');
  expect(html).toContain('14.29%');
  expect(html).toContain('交易所快照更新编号');
  expect(html).toContain('没有交易所历史时间戳');
  const bid = html.match(/<rect[^>]*data-depth-bar="bid"[^>]*>/)?.[0] || '';
  const ask = html.match(/<rect[^>]*data-depth-bar="ask"[^>]*>/)?.[0] || '';
  expect(Number(bid.match(/width="([^"]+)"/)?.[1])).toBeCloseTo(116, 6);
  expect(Number(ask.match(/width="([^"]+)"/)?.[1])).toBeCloseTo(116 / 3, 6);
  expect((html.match(/data-depth-bar="bid"/g) || [])).toHaveLength(2);
  expect((html.match(/data-depth-bar="ask"/g) || [])).toHaveLength(2);
});

it('labels the mid-price denominator and does not turn an imbalance snapshot into a forecast', () => {
  const spread = renderToStaticMarkup(<SpotDepthVisual result={snapshot('bid_ask_spread', { best_bid: 100, best_ask: 101, absolute_spread: 1, relative_spread: 1 / 100.5 })} />);
  expect(spread).toContain('以中间价为分母');
  expect(spread).toContain('0.99502488%');
  const caution = renderToStaticMarkup(<SpotDepthVisual result={snapshot('book_pitfall_order_imbalance', { best_bid: 100, best_ask: 101, top_n: 2, top_n_bid_quantity: 4, top_n_ask_quantity: 3, order_imbalance: 1 / 7 })} />);
  expect(caution).toContain('委比高不代表价格随后必涨');
});
