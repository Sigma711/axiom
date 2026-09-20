import { expect, test, type Page } from '@playwright/test';

const bars = Array.from({ length: 60 }, (_, index) => ({ timestamp: new Date(Date.UTC(2025, 0, 1, index)).toISOString(), open: 100 + index, high: 102 + index, low: 99 + index, close: 101 + index, volume: 1000 + index * 10 }));
const strategies = [{ name: 'sma_cross', display_name: '均线交叉', description: 'demo', params: [{ key: 'fast', label: '快线', default: 5, min: 2, max: 20 }] }, { name: 'rsi', display_name: 'RSI', description: 'demo', params: [] }];
const metrics = { '总收益率': 0.12, '年化收益率': 0.2, '最大回撤_pct': -0.08, '夏普比率': 1.4, '索提诺比率': 1.7, 'Calmar比率': 2.1, '交易笔数': 3, '胜率': 0.66, '最终净值': 11200 };
const backtest = { config: {}, bars, equity_curve: bars.map((bar, index) => ({ timestamp: bar.timestamp, cash: 10_000, position_value: index * 20, equity: 10_000 + index * 20 })), trades: [], signals: [], fills: [], metrics };
const concepts = [{ id: 'rsi_14', name: 'RSI', category: '动量', input_kind: 'market_bars', inputs: [], notes: 'RSI 衡量最近上涨和下跌的相对强度。' }, { id: 'earnings_per_share', name: '每股收益（EPS）', category: '财务', input_kind: 'independent_inputs', inputs: [{ key: 'net_income', label: '净利润', default: 3000000 }, { key: 'preferred_dividends', label: '优先股股息', default: 0 }, { key: 'shares', label: '普通股股数', default: 1000000 }], notes: '每股收益采用可编辑教学数据。' }];

async function mockApi(page: Page) {
  await page.route('**/api/**', async route => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    const json = (body: unknown) => route.fulfill({ contentType: 'application/json', body: JSON.stringify(body) });
    if (path === '/api/knowledge') return json({ total: 2, categories: { 动量: [{ ...concepts[0], summary: '衡量动量', formula: 'RS = avg(gain) / avg(loss)', meaning: '强弱', example: '70 偏高', signals: '观察趋势', pitfalls: '不是单独买卖信号', related: [], code_url: 'https://example.test/rsi', implementation: 'rsi()' }, { id: 'earnings_per_share', name: '每股收益（EPS）', category: '财务', input_kind: 'independent_inputs', inputs: [{ key: 'net_income', label: '净利润', default: 3000000 }, { key: 'preferred_dividends', label: '优先股股息', default: 0 }, { key: 'shares', label: '普通股股数', default: 1000000 }], notes: '每股收益采用可编辑教学数据。', summary: '把归属于普通股股东的利润平摊到每一股。', formula: '(净利润 − 优先股股息) ÷ 普通股股数', meaning: '每股盈利能力', example: '3 元/股', signals: '用于比较盈利能力', pitfalls: '需结合股本变化', related: [], code_url: 'https://example.test/eps', implementation: 'earnings_per_share()' }] } });
    if (path === '/api/symbols') return json({ symbols: ['BTCUSDT'], count: 1, source: 'fixture' });
    if (path === '/api/strategies') return json({ strategies });
    if (path === '/api/indicators') return json({ symbol: 'BTCUSDT', source: 'synthetic', bars, indicators: { sma_20: bars.map((bar, index) => index < 19 ? null : { x: bar.timestamp, y: bar.close - 3 }), rsi_14: bars.map((bar, index) => index < 14 ? null : { x: bar.timestamp, y: 40 + index % 30 }), macd: bars.map((bar, index) => ({ x: bar.timestamp, y: index - 30 })), atr_14: bars.map((bar, index) => ({ x: bar.timestamp, y: 2 + index / 50 })) } });
    if (path === '/api/patterns') return json({ symbol: 'BTCUSDT', patterns: [] });
    if (path === '/api/code_loc') return json({ ok: true, url: 'https://example.test/repo/src/indicator.rs#L42', path: 'src/indicator.rs', line: 42 });
    if (path === '/api/backtest') return json(backtest);
    if (path === '/api/practice' && route.request().method() === 'GET') return json({ concepts, modules: ['data', 'backtest', 'paper', 'compare'], total: 2 });
    if (path === '/api/practice') {
      const request = JSON.parse(route.request().postData() || '{}');
      if (request.concept_id === 'earnings_per_share') return json({ concept_id: 'earnings_per_share', status: 'computed', reason: null, input_kind: 'independent_inputs', provenance: 'editable_teaching_inputs', values: { eps: 3 }, units: { eps: 'currency' }, series: [], notes: ['每股收益来自教学输入。'], module: 'data', source: 'synthetic', symbol: 'BTCUSDT', bars: [] });
      return json({ concept_id: 'rsi_14', status: 'computed', reason: null, input_kind: 'market_bars', provenance: 'provided_market_bars', values: { rsi: 62.5 }, series: [{ name: 'RSI', values: Array.from({ length: 40 }, (_, index) => 35 + index) }], notes: ['RSI 用同一组行情计算。'], module: 'data', source: 'synthetic', symbol: 'BTCUSDT', bars });
    }
    if (path === '/api/paper/snapshot') return json({ is_running: false, current_bar: bars.at(-1), cash: 10000, position_size: 0, position_value: 0, equity: 10000, last_signal: null, last_fill: null, equity_curve: [], trades_count: 0, log: [], bars });
    return json({ status: 'ok' });
  });
}

test.beforeEach(async ({ page }) => { await mockApi(page); });

test('data exploration separates price, oscillator, momentum, and volatility axes', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '数据探索', exact: true }).click();
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible();
  const layout = await page.locator('.ax-chart').evaluate((node: any) => node.layout);
  expect(layout.yaxis.title.text).toBe('价格');
  expect(layout.yaxis2.title.text).toBe('振荡器');
  expect(layout.yaxis3.title.text).toBe('动量');
  expect(layout.yaxis5.title.text).toBe('波动率');
  await expect(page.locator('.ax-chart')).toHaveScreenshot('data-exploration.png', { maxDiffPixelRatio: 0.02 });
});

test('knowledge card leads to an in-context practice result', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '在数据探索中实践' }).first().click();
  await expect(page.getByRole('heading', { name: '数据探索' })).toBeVisible();
  await expect(page.getByLabel('概念实践')).toContainText('RSI');
  await page.getByRole('button', { name: '运行实践' }).click();
  await expect(page.getByLabel('概念实践')).toContainText('已计算');
  await expect(page.getByLabel('概念实践')).toContainText('使用当前模块行情上下文');
});

test('each rendered knowledge concept expands to an explanatory SVG', async ({ page }) => {
  await page.goto('/');
  const cards = page.locator('.ax-kb-card');
  await expect(cards).toHaveCount(2);
  await cards.nth(0).locator('.ax-kb-details').click();
  await expect(cards.nth(0).locator('.ax-knowledge-chart svg')).toBeVisible();
  await expect(cards.nth(0).locator('.ax-practice-reading')).toContainText('教学行情');
  await cards.nth(1).locator('.ax-kb-details').click();
  await expect(cards.nth(1).locator('.ax-knowledge-chart svg')).toBeVisible();
  await expect(cards.nth(1).locator('.ax-practice-reading')).toContainText('单次计算');
});

test('knowledge detail state is visually distinct in both themes', async ({ page }) => {
  await page.goto('/');
  const summary = page.locator('.ax-kb-details').first();
  const darkClosed = await summary.evaluate(node => getComputedStyle(node).color);
  await summary.click();
  await expect(summary).toHaveClass(/is-open/);
  const darkOpen = await summary.evaluate(node => getComputedStyle(node).color);
  expect(darkOpen).not.toBe(darkClosed);
  await page.getByLabel('切换到浅色模式').click();
  const lightOpen = await summary.evaluate(node => getComputedStyle(node).color);
  expect(lightOpen).not.toBe(darkOpen);
  await summary.click();
  const lightClosed = await summary.evaluate(node => getComputedStyle(node).color);
  expect(lightClosed).not.toBe(lightOpen);
});

test('backtest and comparison show the returned metrics', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '回测', exact: true }).click();
  await page.getByRole('button', { name: '运行回测' }).click();
  await expect(page.getByText('12.00%')).toBeVisible();
  await page.getByRole('button', { name: '策略对比' }).click();
  await page.getByRole('button', { name: '跑对比' }).click();
  await expect(page.locator('.ax-cmp-table')).toContainText('均线交叉');
});

test('chart retains warm-up gaps, rejects invalid points, and exposes separate axes', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '数据探索', exact: true }).click();
  const chart = page.locator('.ax-chart');
  const semantic = await chart.evaluate((node: any) => ({
    traces: node.data.map((trace: any) => ({ name: trace.name, y: trace.y, connectgaps: trace.connectgaps })),
    axes: Object.keys(node.layout).filter(key => key.startsWith('yaxis')),
  }));
  expect(semantic.axes).toEqual(expect.arrayContaining(['yaxis', 'yaxis2', 'yaxis3', 'yaxis5']));
  expect(semantic.traces.find((trace: any) => trace.name === 'rsi_14').connectgaps).toBe(false);
  expect(semantic.traces.flatMap((trace: any) => trace.y ?? []).every((value: unknown) => value == null || Number.isFinite(value))).toBe(true);
  expect(semantic.traces.find((trace: any) => trace.name === 'rsi_14').y).toContain(null);
});

test('comparison sends every strategy default and reuses the first returned bars', async ({ page }) => {
  const requests: any[] = [];
  const fourteenStrategies = Array.from({ length: 14 }, (_, index) => ({ name: `strategy_${index + 1}`, display_name: `策略 ${index + 1}`, description: 'fixture', params: [{ key: `period_${index + 1}`, label: `周期 ${index + 1}`, default: index + 2 }] }));
  await page.route('**/api/strategies', route => route.fulfill({ contentType: 'application/json', body: JSON.stringify({ strategies: fourteenStrategies }) }));
  await page.route('**/api/backtest', async route => {
    requests.push(JSON.parse(route.request().postData() || '{}'));
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify(backtest) });
  });
  await page.goto('/');
  await page.getByRole('button', { name: '策略对比', exact: true }).click();
  await page.getByRole('button', { name: '跑对比' }).click();
  await expect.poll(() => requests.length).toBe(14);
  expect(requests.map(request => request.params)).toEqual(fourteenStrategies.map(strategy => ({ [strategy.params[0].key]: strategy.params[0].default })));
  expect(requests[0].bars).toBeUndefined();
  for (const request of requests.slice(1)) expect(request.bars).toEqual(bars);
});

test('practice uses the current bars in data, backtest, paper, and comparison', async ({ page }) => {
  const requests: any[] = [];
  await page.route('**/api/practice', async route => {
    if (route.request().method() === 'GET') return route.fulfill({ contentType: 'application/json', body: JSON.stringify({ concepts, modules: ['data', 'backtest', 'paper', 'compare'], total: 2 }) });
    requests.push(JSON.parse(route.request().postData() || '{}'));
    return route.fulfill({ contentType: 'application/json', body: JSON.stringify({ concept_id: 'rsi_14', status: 'computed', reason: null, input_kind: 'market_bars', provenance: 'provided_market_bars', values: { rsi: 50 }, series: [], notes: [], module: 'data', source: 'synthetic', symbol: 'BTCUSDT', bars }) });
  });
  await page.goto('/');
  for (const setup of [
    async () => page.getByRole('button', { name: '数据探索', exact: true }).click(),
    async () => { await page.getByRole('button', { name: '回测', exact: true }).click(); await page.getByRole('button', { name: '运行回测' }).click(); },
    async () => page.getByRole('button', { name: '模拟盘', exact: true }).click(),
    async () => { await page.getByRole('button', { name: '策略对比', exact: true }).click(); await page.getByRole('button', { name: '跑对比' }).click(); await expect(page.locator('.ax-cmp-table')).toBeVisible(); },
  ]) {
    await setup();
    await expect(page.getByLabel('概念实践')).toBeVisible();
    const request = page.waitForRequest(value => value.url().includes('/api/practice') && value.method() === 'POST');
    await page.getByRole('button', { name: '运行实践' }).click();
    await request;
    await page.goto('/');
  }
  expect(requests).toHaveLength(4);
  for (const request of requests) expect(request.bars).toEqual(bars);
});

test('empty data and stale requests do not replace the latest chart', async ({ page }) => {
  let calls = 0;
  await page.route('**/api/indicators**', async route => {
    calls += 1;
    const symbol = new URL(route.request().url()).searchParams.get('symbol');
    if (calls === 1) await new Promise(resolve => setTimeout(resolve, 250));
    const response = symbol === 'EMPTY' ? { symbol, source: 'synthetic', bars: [], indicators: {} } : { symbol, source: 'synthetic', bars, indicators: {} };
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify(response) });
  });
  await page.goto('/');
  await page.getByRole('button', { name: '数据探索', exact: true }).click();
  await page.getByLabel('自定义').fill('EMPTY');
  await page.getByRole('button', { name: '加载数据' }).click();
  await expect(page.locator('.ax-chart-empty')).toBeVisible();
  await page.getByLabel('自定义').fill('BTCUSDT');
  await page.getByRole('button', { name: '加载数据' }).click();
  await expect(page.locator('.ax-summary')).toContainText('BTCUSDT');
  await expect(page.locator('.ax-chart-empty')).toHaveCount(0);
});

test('mobile controls remain reachable without viewport overflow', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await page.getByRole('button', { name: '数据探索', exact: true }).click();
  await page.getByRole('button', { name: '数据源' }).click();
  await expect(page.getByText('合成 (随机)', { exact: true })).toBeVisible();
  expect(await page.locator('.ax-dd-arrow').count()).toBe(0);
  expect(await page.getByRole('button', { name: '数据源' }).evaluate(button => getComputedStyle(button, '::after').content)).not.toBe('none');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});

test('source links resolve precise anchors and theme selection persists', async ({ page }) => {
  await page.goto('/');
  const source = page.locator('.ax-gh-btn[href]').first();
  await expect(source).toHaveAttribute('href', /#L42$/);
  await page.getByLabel('切换到浅色模式').click();
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe('light');
  expect(await page.evaluate(() => localStorage.getItem('axiom-theme'))).toBe('light');
  await page.reload();
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe('light');
  const contrast = await page.evaluate(() => {
    const parse = (value: string) => {
      const hex = value.trim().replace('#', '');
      const parts = hex.length === 3 ? hex.split('').map(part => parseInt(part + part, 16)) : [hex.slice(0, 2), hex.slice(2, 4), hex.slice(4, 6)].map(part => parseInt(part, 16));
      return parts.map(channel => channel / 255).map(channel => channel <= .03928 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2);
    };
    const luminance = (value: string) => { const [r, g, b] = parse(value); return .2126 * r + .7152 * g + .0722 * b; };
    const style = getComputedStyle(document.documentElement);
    const a = luminance(style.getPropertyValue('--text')), b = luminance(style.getPropertyValue('--bg'));
    return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
  });
  expect(contrast).toBeGreaterThanOrEqual(4.5);
});
