// @vitest-environment jsdom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { renderToStaticMarkup } from 'react-dom/server';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MarketBreadthSnapshotView, MarketBreadthVisual, MarketTickSnapshotView, type MarketBreadthSnapshot } from './MarketBreadthVisual';

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const fixture = (): MarketBreadthSnapshot => ({
  schema_version: 1,
  as_of: '2026-10-02',
  universe: { id: 'dow_30_2024_11_08', name: 'Dow Jones Industrial Average fixed constituent snapshot', member_count: 30, constituents_as_of: '2024-11-08', constituents_source: 'https://press.spglobal.com/dow-change', symbols: ['AAPL', 'MSFT', 'NVDA'], scope_note: 'Fixed 2024-11-08 basket applied retrospectively; not official historical DJIA membership.', calendar: 'provider-observed member sessions', missing_policy: 'exclude a missing member from that session; drop the entire session below 90% coverage' },
  coverage: { latest_eligible_members: 30, total_members: 30, minimum_required: 27 },
  source: { retrieval: 'live public US-stock daily OHLCV', retrieved_at: '2026-10-03T02:10:00Z', cache_status: 'live_refresh', members: [{ symbol: 'AAPL', provider: 'Yahoo', endpoint: 'https://query1.finance.yahoo.com/AAPL', price_basis: 'unadjusted', corporate_actions: 'split boundaries disclosed', split_event_dates: ['2020-08-31'] }, { symbol: 'MSFT', provider: 'Stooq', endpoint: 'https://stooq.com/MSFT', price_basis: 'provider adjustment unverified' }] },
  observations: [
    { date: '2026-09-29', eligible_members: 30, advances: 18, declines: 10, unchanged: 2, up_volume: 1000, down_volume: 500, new_highs: 4, new_lows: 1 },
    { date: '2026-09-30', eligible_members: 29, advances: 12, declines: 16, unchanged: 1, up_volume: 700, down_volume: 900, new_highs: 2, new_lows: 3 },
    { date: '2026-10-02', eligible_members: 30, advances: 20, declines: 9, unchanged: 1, up_volume: 1200, down_volume: 450, new_highs: 5, new_lows: 1 },
  ],
  concepts: [
    { id: 'ad_line', status: 'available', latest: 43, triggered: null, unit: 'issues', definition: 'Cumulative advances minus declines.', inputs: { initial_value: 0 }, reason: null, series: [{ date: '2026-09-29', value: 10, eligible_members: 30 }, { date: '2026-10-02', value: 43, eligible_members: 30 }] },
    { id: 'trin', status: 'available', latest: 0.82, triggered: null, unit: 'ratio', definition: '(advances/declines)/(up volume/down volume).', inputs: { advances: 20, declines: 9 }, reason: null, series: [] },
    { id: 'mcclellan', status: 'available', latest: 12.6, triggered: null, unit: 'issues', definition: 'EMA19 - EMA39.', inputs: {}, reason: null, series: [] },
    { id: 'new_high_low', status: 'available', latest: 4, triggered: null, unit: 'issues', definition: 'New highs minus new lows.', inputs: {}, reason: null, series: [] },
    { id: 'tick', status: 'unavailable', latest: null, triggered: null, unit: 'issues', definition: 'Contemporaneous upticks minus downticks.', inputs: {}, series: [], reason: 'daily OHLCV has no contemporaneous intraday observations' },
    { id: 'breadth_thrust', status: 'available', latest: 0.57, triggered: false, unit: 'fraction', definition: '10-session EMA.', inputs: {}, reason: null, series: [] },
    { id: 'bullish_percent', status: 'unavailable', latest: null, triggered: null, unit: 'percent', definition: 'Persistent P&F buy signals.', inputs: {}, series: [], reason: 'not enough Point & Figure signals' },
    { id: 'up_down_volume', status: 'available', latest: 1.22, triggered: null, unit: 'ratio', definition: 'Up volume divided by down volume.', inputs: {}, reason: null, series: [] },
  ],
});

describe('MarketBreadthSnapshotView', () => {
  it('shows the selected real series, source, fixed-universe boundary and missing observations', () => {
    const html = renderToStaticMarkup(<MarketBreadthSnapshotView conceptId="ad_line" snapshot={fixture()} />);
    expect(html).toContain('Advance–Decline Line'); expect(html).toContain('2024-11-08 道指 30 股固定篮子'); expect(html).toContain('成分基准日 2024-11-08');
    expect(html).toContain('最低日覆盖率 96.7%'); expect(html).toContain('below 90% coverage'); expect(html).toContain('AAPL · Yahoo'); expect(html).toContain('抓取于 2026-10-03 02:10 UTC'); expect(html).toContain('live_refresh'); expect(html).toContain('公司行动 split boundaries disclosed');
    expect(html).toContain('缺测 1 个点'); expect(html).toContain('data-breadth-missing="2026-09-30"'); expect(html).toContain('data-breadth-path-segment');
  });
  it('states why unavailable concepts cannot be computed and never turns them into zero', () => {
    const html = renderToStaticMarkup(<MarketBreadthSnapshotView conceptId="tick" snapshot={fixture()} />);
    expect(html).toContain('TICK 指数'); expect(html).toContain('当前不可计算'); expect(html).toContain('没有同一时刻的逐笔 uptick / downtick'); expect(html).not.toContain('>0<'); expect(html).not.toContain('aria-label="TICK 指数趋势图"');
  });
  it('keeps all eight concepts auditable while focusing the requested one', () => {
    const html = renderToStaticMarkup(<MarketBreadthSnapshotView conceptId="trin" snapshot={fixture()} />);
    expect(html.match(/data-breadth-concept=/g)).toHaveLength(8); expect(html).toContain('data-breadth-concept="trin"'); expect(html).toContain('aria-current="true"'); expect(html).toContain('3 个有效交易日');
  });
  it('draws the real three-member crypto TICK at its own UTC sample time', () => {
    const result = fixture(); const tick = result.concepts.find(item => item.id === 'tick')!;
    Object.assign(tick, { status: 'available', latest: 1, reason: null, inputs: { market: 'binance_spot_usdt_fixed_three', sample_at: '2026-10-03T03:04:05Z', up_ticks: 2, down_ticks: 1, unchanged_ticks: 0, members: [
      { symbol: 'BTCUSDT', previous_price: 100, latest_price: 101, direction: 'up', source_url: 'https://api.binance.com/btc' },
      { symbol: 'ETHUSDT', previous_price: 50, latest_price: 49, direction: 'down', source_url: 'https://api.binance.com/eth' },
      { symbol: 'BNBUSDT', previous_price: 25, latest_price: 26, direction: 'up', source_url: 'https://api.binance.com/bnb' },
    ] } });
    const html = renderToStaticMarkup(<MarketBreadthSnapshotView conceptId="tick" snapshot={result} />);
    expect(html).toContain('截至 2026-10-03 03:04:05 UTC'); expect(html).toContain('三成员逐笔价格方向'); expect(html).toContain('BTCUSDT'); expect(html).toContain('2 上跳 − 1 下跳 = 1'); expect(html).not.toContain('接口没有返回可绘制的历史序列');
  });
  it('renders BPI as verified point-and-figure states rather than an empty time series', () => {
    const result = fixture(); const bpi = result.concepts.find(item => item.id === 'bullish_percent')!;
    Object.assign(bpi, { status: 'available', latest: 60, reason: null, inputs: { buy_signals: 18, eligible_signals: 30, box_method: 'logarithmic 1% close-only', reversal_boxes: 3 } });
    const html = renderToStaticMarkup(<MarketBreadthSnapshotView conceptId="bullish_percent" snapshot={result} />);
    expect(html).toContain('点数图多头状态占比'); expect(html).toContain('18 个明确买入状态'); expect(html).toContain('12 个明确非买入状态'); expect(html).toContain('width:60%'); expect(html).not.toContain('接口没有返回可绘制的历史序列');
  });
  it('renders the original-book McClellan summation from real oscillator points rather than the A-D line', () => {
    const result = fixture();
    result.concepts.find(item => item.id === 'mcclellan')!.series = [
      { date: '2026-09-29', value: 2, eligible_members: 30 },
      { date: '2026-09-30', value: -1, eligible_members: 29 },
      { date: '2026-10-02', value: 3, eligible_members: 30 },
    ];
    const html = renderToStaticMarkup(<MarketBreadthSnapshotView conceptId="book_mcclellan_sum" snapshot={result} />);
    expect(html).toContain('McClellan Summation Index');
    expect(html).toContain('data-breadth-selected="book_mcclellan_sum"');
    expect(html).toContain('data-breadth-point="2026-10-02"');
    expect(html).toContain('>4</strong>');
    expect(html).not.toContain('data-breadth-selected="ad_line"');
  });
  it('renders the independent intraday TICK practice snapshot', () => {
    const html = renderToStaticMarkup(<MarketTickSnapshotView snapshot={{ market: 'binance_spot_usdt_fixed_three', definition: '同一 UTC 时点最近两笔成交价方向', sample_at: '2026-10-03T03:04:05Z', universe: ['BTCUSDT', 'ETHUSDT', 'BNBUSDT'], up_ticks: 2, down_ticks: 1, unchanged_ticks: 0, net_tick: 1, members: [
      { symbol: 'BTCUSDT', previous_price: 100, latest_price: 101, direction: 'up', source_url: 'https://api.binance.com/btc' },
      { symbol: 'ETHUSDT', previous_price: 50, latest_price: 49, direction: 'down', source_url: 'https://api.binance.com/eth' },
      { symbol: 'BNBUSDT', previous_price: 25, latest_price: 26, direction: 'up', source_url: 'https://api.binance.com/bnb' },
    ] }} />);
    expect(html).toContain('真实逐笔快照'); expect(html).toContain('2026-10-03 03:04 UTC'); expect(html).toContain('2 上跳 − 1 下跳 = 1'); expect(html).toContain('BTCUSDT · Binance 聚合成交');
  });
});

describe('MarketBreadthVisual loading', () => {
  let host: HTMLDivElement; let root: Root;
  beforeEach(() => { host = document.createElement('div'); document.body.append(host); root = createRoot(host); });
  afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });
  it('loads the public snapshot endpoint and renders the requested concept', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(fixture()), { status: 200 })); vi.stubGlobal('fetch', fetchMock);
    await act(async () => { root.render(<MarketBreadthVisual conceptId="ad_line" />); }); await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(fetchMock).toHaveBeenCalledWith('/api/market-breadth/snapshot', expect.objectContaining({ signal: expect.any(AbortSignal) })); expect(host.textContent).toContain('Advance–Decline Line'); expect(host.textContent).toContain('43');
  });
  it('surfaces an HTTP failure and retries through the same user-visible control', async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(new Response('provider unavailable', { status: 503 })).mockResolvedValueOnce(new Response(JSON.stringify(fixture()), { status: 200 })); vi.stubGlobal('fetch', fetchMock);
    await act(async () => { root.render(<MarketBreadthVisual conceptId="trin" />); }); await act(async () => { await Promise.resolve(); await Promise.resolve(); }); expect(host.textContent).toContain('HTTP 503');
    await act(async () => host.querySelector<HTMLButtonElement>('button')!.click()); await act(async () => { await Promise.resolve(); await Promise.resolve(); }); expect(host.textContent).toContain('TRIN / Arms Index'); expect(fetchMock).toHaveBeenCalledTimes(2);
  });
  it('opens TICK from its independent Binance practice even when Dow data is unavailable', async () => {
    const tick = { market: 'binance_spot_usdt_fixed_three', definition: '同一 UTC 时点最近两笔成交价方向', sample_at: '2026-10-03T03:04:05Z', universe: ['BTCUSDT', 'ETHUSDT', 'BNBUSDT'], up_ticks: 1, down_ticks: 0, unchanged_ticks: 2, net_tick: 1, members: [
      { symbol: 'BTCUSDT', previous_price: 100, latest_price: 101, direction: 'up', source_url: 'https://data-api.binance.vision/btc' },
      { symbol: 'ETHUSDT', previous_price: 50, latest_price: 50, direction: 'unchanged', source_url: 'https://data-api.binance.vision/eth' },
      { symbol: 'BNBUSDT', previous_price: 25, latest_price: 25, direction: 'unchanged', source_url: 'https://data-api.binance.vision/bnb' },
    ] };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ market_tick: tick }), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    await act(async () => { root.render(<MarketBreadthVisual conceptId="tick" />); });
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(fetchMock).toHaveBeenCalledOnce();
    expect(fetchMock).toHaveBeenCalledWith('/api/practice', expect.objectContaining({ method: 'POST' }));
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({ concept_id: 'tick', module: 'data', source: 'market_breadth', inputs: {} });
    expect(host.textContent).toContain('TICK 指数');
    expect(host.textContent).toContain('BTCUSDT');
  });
});
