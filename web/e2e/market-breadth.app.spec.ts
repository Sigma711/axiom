import { expect, test } from './v8-coverage';

const knowledge = {
  id: 'ad_line', category: '市场宽度', name: 'Advance-Decline Line 腾落线', summary: '上涨家数减下跌家数的累计。',
  formula: 'ADL(t) = ADL(t-1) + Advances(t) - Declines(t)', meaning: '观察指数上涨是否得到多数成员参与。',
  example: '指数上涨而 ADL 下跌时，参与面正在收窄。', signals: '结合趋势与背离观察。', pitfalls: 'A-D 是当日净家数，ADL 才是累计线。',
  related: ['TRIN', 'McClellan Oscillator'], code_url: 'https://example.test/src/indicators/breadth.rs', implementation: 'market_advance_decline_line()',
};
const practiceConcept = { id: 'ad_line', name: 'Advance-Decline Line 腾落线', category: '市场宽度', input_kind: 'market_breadth_case', inputs: [], notes: '固定成员样本。', plan: { markets: ['market_breadth'], modules: ['data'], required_datasets: ['fixed_dow30_2024_11_08'], source_policy: 'real_required', fixed_source: 'market_breadth', goal: '核对固定成员市场宽度。' } };

const observations = [
  ['2026-09-25', 30, 18, 10, 2, 1000, 500, 3, 1], ['2026-09-26', 30, 16, 12, 2, 900, 600, 2, 2],
  ['2026-09-29', 29, 12, 16, 1, 700, 900, 2, 3], ['2026-09-30', 30, 20, 9, 1, 1200, 450, 5, 1],
].map(([date, eligible_members, advances, declines, unchanged, up_volume, down_volume, new_highs, new_lows]) => ({ date, eligible_members, advances, declines, unchanged, up_volume, down_volume, new_highs, new_lows }));

const common = { triggered: null, reason: null };
const snapshot = {
  schema_version: 1, as_of: '2026-09-30',
  universe: { id: 'dow_30_2024_11_08', name: 'Dow Jones Industrial Average fixed constituent snapshot', member_count: 30, constituents_as_of: '2024-11-08', constituents_source: 'https://press.spglobal.com/dow-change', symbols: Array.from({ length: 30 }, (_, index) => `DOW${index + 1}`), scope_note: 'Fixed 2024-11-08 basket applied retrospectively; not official historical DJIA membership.', calendar: 'union of provider-observed member sessions; retain dates with at least 27 members', missing_policy: 'exclude a missing member from that session; never count it as unchanged or declining; drop the entire session below 90% coverage' },
  coverage: { latest_eligible_members: 30, total_members: 30, minimum_required: 27 },
  source: { retrieval: 'live public US-stock daily OHLCV; provider selected independently per member', members: Array.from({ length: 30 }, (_, index) => ({ symbol: `DOW${index + 1}`, provider: index % 2 ? 'Stooq' : 'Yahoo', endpoint: `https://example.test/market/DOW${index + 1}`, price_basis: 'provider_adjustment_unverified' })) },
  observations,
  concepts: [
    { ...common, id: 'ad_line', status: 'available', unit: 'issues', latest: 17, definition: 'Cumulative daily advances minus declines.', inputs: { initial_value: 0 }, series: [{ date: '2026-09-25', value: 8, eligible_members: 30 }, { date: '2026-09-26', value: 12, eligible_members: 30 }, { date: '2026-09-30', value: 17, eligible_members: 30 }] },
    { ...common, id: 'trin', status: 'available', unit: 'ratio', latest: .8333, definition: '(advances/declines)/(up volume/down volume).', inputs: { advances: 20 }, series: [] },
    { ...common, id: 'mcclellan', status: 'available', unit: 'issues', latest: 4.2, definition: 'EMA19 - EMA39.', inputs: {}, series: [] },
    { ...common, id: 'new_high_low', status: 'available', unit: 'issues', latest: 4, definition: 'New highs minus new lows.', inputs: {}, series: [] },
    { id: 'tick', status: 'unavailable', unit: 'issues', latest: null, triggered: null, definition: 'Contemporaneous upticks minus downticks.', inputs: {}, series: [], reason: 'Daily OHLCV has no contemporaneous intraday observations.' },
    { ...common, id: 'breadth_thrust', status: 'available', unit: 'fraction', latest: .574, definition: 'Zweig 10-session EMA.', inputs: {}, series: [] },
    { id: 'bullish_percent', status: 'unavailable', unit: 'percent', latest: null, triggered: null, definition: 'Persistent P&F signals.', inputs: {}, series: [], reason: 'Not enough determinate Point & Figure signals.' },
    { ...common, id: 'up_down_volume', status: 'available', unit: 'ratio', latest: 2.6667, definition: 'Up volume divided by down volume.', inputs: {}, series: [] },
  ],
};

test('real market breadth mounts from knowledge, switches themes instantly, and stays readable at 390px', async ({ page }) => {
  let breadthGets = 0;
  await page.route('**/api/**', route => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/api/knowledge') return route.fulfill({ json: { total: 1, categories: { 市场宽度: [knowledge] } } });
    if (path === '/api/practice' && route.request().method() === 'GET') return route.fulfill({ json: { concepts: [practiceConcept], modules: ['data'], total: 1 } });
    if (path === '/api/practice') {
      expect(route.request().postDataJSON()).toEqual({ concept_id: 'ad_line', module: 'data', source: 'market_breadth', inputs: {} });
      return route.fulfill({ json: { concept_id: 'ad_line', status: 'computed', reason: null, input_kind: 'market_breadth_case', provenance: 'server_fetched_market_breadth_snapshot', context: 'fixed_market_breadth_snapshot', module: 'data', source: 'market_breadth', symbol: null, source_markets: { daily: 'us_equity', tick: 'crypto_spot' }, values: { ad_line: 17, triggered: null }, units: { ad_line: 'issues' }, series: [], notes: [], inputs: {}, bars: [], market_breadth: snapshot } });
    }
    if (path === '/api/market-breadth/snapshot') { breadthGets += 1; return route.fulfill({ json: snapshot }); }
    return route.fulfill({ json: {} });
  });
  await page.goto('/learn');
  const card = page.locator('[data-concept-id="ad_line"]');
  await card.locator('summary').click();
  const visual = card.getByLabel('Advance–Decline Line 腾落线真实市场宽度实践');
  await expect(visual).toBeVisible();
  const knowledgeGets = breadthGets;
  await expect(visual.locator('[data-breadth-concept]')).toHaveCount(8);
  await expect(visual).toContainText('最低日覆盖率 96.7%');
  await expect(visual).toContainText('缺测 1 个点');
  await expect(visual.locator('[data-breadth-missing="2026-09-29"]')).toBeVisible();
  await visual.locator('summary').click();
  await expect(visual.getByRole('link', { name: 'DOW1 · Yahoo ↗' })).toHaveAttribute('href', 'https://example.test/market/DOW1');
  await visual.locator('summary').click();
  // Element screenshots scroll tall cards in segments; disable the unrelated
  // sticky tab bar so it cannot be stitched through the middle of the card.
  await page.addStyleTag({ content: '.ax-tabs{position:static!important}' });

  const dark = await visual.evaluate(node => ({ background: getComputedStyle(node).backgroundColor, color: getComputedStyle(node).color }));
  await visual.scrollIntoViewIfNeeded();
  await expect(visual).toHaveScreenshot('market-breadth-dark.png', { maxDiffPixelRatio: .01 });
  await page.getByLabel('切换到浅色模式').click();
  await expect.poll(() => visual.evaluate(node => getComputedStyle(node).backgroundColor)).not.toBe(dark.background);
  expect(await visual.evaluate(node => getComputedStyle(node).color)).not.toBe(dark.color);
  await expect(visual).toHaveScreenshot('market-breadth-light.png', { maxDiffPixelRatio: .01 });

  await page.setViewportSize({ width: 390, height: 844 });
  await visual.scrollIntoViewIfNeeded();
  expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
  expect(await visual.evaluate(node => node.getBoundingClientRect().right <= window.innerWidth)).toBe(true);
  await expect(visual).toHaveScreenshot('market-breadth-mobile.png', { maxDiffPixelRatio: .01 });

  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=ad_line&source=market_breadth$/);
  const panel = page.getByLabel('概念实践');
  await expect(panel).toContainText('不采用本页单一标的或手动输入');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  await expect(page.locator('.ax-controls')).toHaveCount(0);
  await panel.getByRole('button', { name: '运行实践' }).click();
  const practiceVisual = panel.getByLabel('Advance–Decline Line 腾落线真实市场宽度实践');
  await expect(practiceVisual).toBeVisible();
  await expect(practiceVisual.locator('[data-breadth-concept]')).toHaveCount(8);
  expect(breadthGets).toBe(knowledgeGets);
});
