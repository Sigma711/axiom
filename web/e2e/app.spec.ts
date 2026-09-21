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
    if (path === '/api/knowledge') return json({ total: 2, categories: { 动量: [{ ...concepts[0], summary: '衡量动量', formula: 'RS = avg(gain) / avg(loss)', meaning: '强弱', example: '70 偏高', signals: '观察趋势', pitfalls: '不是单独买卖信号', related: [], code_url: 'https://example.test/repo/src/indicator.rs#L42', implementation: 'rsi()' }, { id: 'earnings_per_share', name: '每股收益（EPS）', category: '财务', input_kind: 'independent_inputs', inputs: [{ key: 'net_income', label: '净利润', default: 3000000 }, { key: 'preferred_dividends', label: '优先股股息', default: 0 }, { key: 'shares', label: '普通股股数', default: 1000000 }], notes: '每股收益采用可编辑教学数据。', summary: '把归属于普通股股东的利润平摊到每一股。', formula: '(净利润 − 优先股股息) ÷ 普通股股数', meaning: '每股盈利能力', example: '3 元/股', signals: '用于比较盈利能力', pitfalls: '需结合股本变化', related: [], code_url: 'https://example.test/repo/src/indicator.rs#L42', implementation: 'earnings_per_share()' }] } });
    if (path === '/api/symbols') return json({ symbols: ['BTCUSDT'], count: 1, source: 'fixture' });
    if (path === '/api/strategies') return json({ strategies });
    if (path === '/api/indicators') return json({ symbol: 'BTCUSDT', source: 'synthetic', bars, indicators: { sma_20: bars.map((bar, index) => index < 19 ? null : { x: bar.timestamp, y: bar.close - 3 }), rsi_14: bars.map((bar, index) => index < 14 ? null : { x: bar.timestamp, y: 40 + index % 30 }), macd: bars.map((bar, index) => ({ x: bar.timestamp, y: index - 30 })), atr_14: bars.map((bar, index) => ({ x: bar.timestamp, y: 2 + index / 50 })) } });
    if (path === '/api/patterns') return json({ symbol: 'BTCUSDT', patterns: [] });
    if (path === '/api/code_loc') return json({ ok: true, url: 'https://example.test/repo/src/indicator.rs#L42', github_url: 'https://github.com/Sigma711/axiom/blob/abc123/src/indicator.rs#L42-L48', path: 'src/indicator.rs', line: 42 });
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
  expect(await page.locator('.ax-chart').evaluate(node => node.querySelector('svg.main-svg')!.getBoundingClientRect().height <= node.getBoundingClientRect().height)).toBe(true);
  await expect(page.locator('.ax-chart')).toHaveScreenshot('data-exploration.png', { maxDiffPixelRatio: 0.02 });
});

test('price-only overlays use the full chart and dates sit below all panels', async ({ page }) => {
  await page.route('**/api/indicators**', route => route.fulfill({ json: { symbol: 'BTCUSDT', source: 'synthetic', bars, indicators: { sma_20: bars.map(bar => ({x:bar.timestamp,y:bar.close - 3})) } } }));
  await page.goto('/');
  await page.getByRole('button', { name: '数据探索', exact: true }).click();
  const chart = page.locator('.ax-chart');
  await expect(chart.locator('svg.main-svg').first()).toBeVisible();
  const layout = await chart.evaluate((node:any) => node.layout);
  expect(layout.yaxis.domain).toEqual([0, 1]);
  expect(layout.xaxis.anchor).toBe('free');
  expect(layout.xaxis.position).toBe(0);
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
  await expect(cards.nth(0).locator('.ax-knowledge-chart svg, .ax-series-illustration svg')).toBeVisible();
  await expect(cards.nth(0).locator('.ax-practice-reading')).toContainText('教学行情');
  await cards.nth(1).locator('.ax-kb-details').click();
  await expect(cards.nth(1).locator('.ax-knowledge-chart svg, .ax-series-illustration svg')).toBeVisible();
  await expect(cards.nth(1).locator('.ax-practice-reading')).toContainText('单次计算');
});

test('knowledge detail state is visually distinct in both themes', async ({ page }) => {
  await page.goto('/');
  await page.addInitScript(() => localStorage.setItem('axiom-theme', 'dark'));
  const summary = page.locator('.ax-kb-details').first();
  const darkClosed = await summary.evaluate(node => getComputedStyle(node).color);
  await summary.click();
  await expect(summary).toHaveClass(/is-open/);
  await expect(summary).toContainText('收起详情');
  const darkOpen = await summary.evaluate(node => getComputedStyle(node).color);
  expect(darkOpen).not.toBe(darkClosed);
  await page.getByLabel('切换到浅色模式').click();
  const lightOpen = await summary.evaluate(node => getComputedStyle(node).color);
  expect(lightOpen).not.toBe(darkOpen);
  await summary.click();
  await expect(summary).not.toHaveClass(/is-open/);
  await expect(summary).toContainText('展开详情');
  const lightClosed = await summary.evaluate(node => getComputedStyle(node).color);
  expect(lightClosed).not.toBe(lightOpen);
});

test('book chart visuals keep P&F boxes in columns and Kagi as orthogonal segments', async ({ page }) => {
  const bookConcepts = [{ id: 'book_chart_pnf', name: '点数图', category: '非标准图', input_kind: 'independent_inputs', inputs: [], notes: '' }, { id: 'book_chart_kagi', name: '卡吉线', category: '非标准图', input_kind: 'independent_inputs', inputs: [], notes: '' }];
  await page.route('**/api/knowledge', async route => {
    const body = { total: bookConcepts.length, categories: { 非标准图: bookConcepts.map(entry => ({ ...entry, summary: '', formula: '', meaning: '', example: '', signals: '', pitfalls: '', related: [], code_url: '', implementation: '' })) } };
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify(body) });
  });
  await page.route('**/api/practice', route => {
    if (route.request().method() === 'GET') return route.fulfill({ contentType: 'application/json', body: JSON.stringify({ concepts: bookConcepts, modules: ['data'], total: bookConcepts.length }) });
    const id = JSON.parse(route.request().postData() || '{}').concept_id;
    const chart = id === 'book_chart_pnf' ? { kind: 'point_figure', bars: [{ open: 10, high: 12, low: 10, close: 10, direction: 1 }, { open: 10, high: 12, low: 10, close: 12, direction: 1 }, { open: 12, high: 12, low: 9, close: 9, direction: -1 }] } : { kind: 'kagi', bars: [{ open: 10, high: 12, low: 10, close: 12, direction: 1, line_style: 'yin' }, { open: 12, high: 14, low: 12, close: 14, direction: 1, line_style: 'yang', switch_price: 13 }, { open: 14, high: 14, low: 9, close: 9, direction: -1, line_style: 'yin' }] };
    return route.fulfill({ contentType: 'application/json', body: JSON.stringify({ concept_id: id, status: 'computed', reason: null, input_kind: 'independent_inputs', provenance: 'editable_teaching_inputs', values: {}, series: [], notes: [], module: 'data', source: 'synthetic', symbol: 'BTCUSDT', bars: [], chart }) });
  });
  await page.goto('/');
  const cards = page.locator('.ax-kb-card');
  await cards.nth(0).locator('.ax-kb-details').click();
  const pnf = cards.nth(0).locator('[data-chart-kind="point_figure"]');
  await expect(pnf).toBeVisible();
  const pnfXs = await pnf.locator('[data-pnf-column="0"]').evaluateAll(nodes => nodes.map(node => node.getAttribute('x')));
  expect(new Set(pnfXs).size).toBe(1);
  await cards.nth(1).locator('.ax-kb-details').click();
  const kagi = cards.nth(1).locator('[data-chart-kind="kagi"]');
  await expect(kagi.locator('[data-kagi-horizontal="true"]')).toHaveCount(1);
  await expect(kagi.locator('[data-kagi-vertical="true"]')).toHaveCount(3);
  await expect(kagi.locator('[data-kagi-switch="true"]')).toHaveCount(1);
  const sameColumnXs = await kagi.locator('[data-kagi-vertical="true"]').evaluateAll(nodes => nodes.slice(0, 2).map(node => node.getAttribute('x1')));
  expect(new Set(sameColumnXs).size).toBe(1);
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

test('canonical data-exploration practice uses the current bars', async ({ page }) => {
  const requests: any[] = [];
  await page.route('**/api/practice', async route => {
    if (route.request().method() === 'GET') return route.fulfill({ contentType: 'application/json', body: JSON.stringify({ concepts, modules: ['data', 'backtest', 'paper', 'compare'], total: 2 }) });
    requests.push(JSON.parse(route.request().postData() || '{}'));
    return route.fulfill({ contentType: 'application/json', body: JSON.stringify({ concept_id: 'rsi_14', status: 'computed', reason: null, input_kind: 'market_bars', provenance: 'provided_market_bars', values: { rsi: 50 }, series: [], notes: [], module: 'data', source: 'synthetic', symbol: 'BTCUSDT', bars }) });
  });
  await page.goto('/data?concept=rsi_14');
  for (const setup of [
    async () => expect(page).toHaveURL(/\/data\?concept=rsi_14/),
    async () => { await page.getByRole('button', { name: '回测', exact: true }).click(); await page.getByRole('button', { name: '运行回测' }).click(); },
    async () => page.getByRole('button', { name: '模拟盘', exact: true }).click(),
    async () => { await page.getByRole('button', { name: '策略对比', exact: true }).click(); await page.getByRole('button', { name: '跑对比' }).click(); await expect(page.locator('.ax-cmp-table')).toBeVisible(); },
  ].slice(0, 1)) {
    await setup();
    await expect(page.getByLabel('概念实践')).toBeVisible();
    const request = page.waitForRequest(value => value.url().includes('/api/practice') && value.method() === 'POST');
    await page.getByRole('button', { name: '运行实践' }).click();
    await request;
    await page.goto('/');
  }
  expect(requests).toHaveLength(1);
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

test('selecting a dropdown option closes it and Escape dismisses it', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '数据探索', exact: true }).click();
  const trigger = page.getByRole('button', { name: '数据源', exact: true });
  await trigger.click();
  await page.getByText('合成 (随机)', { exact: true }).click();
  await expect(page.locator('.ax-dd-menu')).toHaveCount(0);
  await expect(trigger).toContainText('合成 (随机)');
  await trigger.click();
  await trigger.press('Escape');
  await expect(page.locator('.ax-dd-menu')).toHaveCount(0);
});

test('primary controls retain readable contrast on hover in both themes', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', {name:'数据探索', exact:true}).click();
  for (const theme of ['dark', 'light']) {
    if (theme === 'light') await page.getByLabel('切换到浅色模式').click();
    const button = page.getByRole('button', {name:'加载数据'});
    await button.hover();
    const contrast = await button.evaluate(node => {
      const luminance = (color:string) => {
        const channels = color.match(/[\d.]+/g)!.slice(0,3).map(Number).map(value => value / 255).map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4);
        return .2126 * channels[0] + .7152 * channels[1] + .0722 * channels[2];
      };
      const style = getComputedStyle(node), foreground = luminance(style.color), background = luminance(style.backgroundColor);
      return (Math.max(foreground, background) + .05) / (Math.min(foreground, background) + .05);
    });
    expect(contrast).toBeGreaterThanOrEqual(4.5);
  }
});

test('source links resolve precise anchors and theme selection persists', async ({ page }) => {
  await page.goto('/');
  const source = page.locator('.ax-code-link[href]').first();
  await page.locator('.ax-kb-details').first().click();
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


test('knowledge navigation opens one direct practice without a selector', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '在数据探索中实践' }).first().click();
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-dd-trigger')).toHaveCount(0);
  await expect(panel).toContainText('RSI');
  await expect(panel.getByLabel('净利润', { exact: true })).toHaveCount(0);
  const request = page.waitForRequest(r => r.url().includes('/api/practice') && r.method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  expect((await request).postDataJSON().concept_id).toBe('rsi_14');
});

test('performance context is visible and manual edits reach the request', async ({ page }) => {
  const inputs = [{key:'returns',label:'收益率序列',default:[0.01,0.02]}, {key:'periods_per_year',label:'年化周期数',default:252}, {key:'risk_free_annual',label:'年化无风险利率',default:0.02}];
  await page.route('**/api/practice', async route => {
    if (route.request().method() === 'GET') return route.fulfill({json:{concepts:[{id:'sharpe',name:'夏普',category:'绩效',input_kind:'independent_inputs',inputs,notes:'绩效教学'}]}});
    return route.fulfill({json:{status:'computed',provenance:'editable_teaching_inputs',values:{sharpe:1},units:{sharpe:'ratio'},series:[],notes:[]}});
  });
  await page.goto('/backtest?concept=sharpe');
  await expect(page).toHaveURL(/\/backtest\?concept=sharpe/);
  await page.getByRole('button', { name: '运行回测' }).click();
  await expect(page.locator('.ax-metrics')).toBeVisible();
  const panel = page.getByLabel('概念实践');
  await panel.getByRole('button', {name:'使用本页上下文',exact:true}).click();
  await expect(panel.getByLabel('年化周期数', {exact:true})).toHaveValue('8760');
  await expect(panel.getByLabel('收益率序列', {exact:true})).toBeDisabled();
  await panel.getByRole('button', {name:'改用手动输入',exact:true}).click();
  await panel.getByLabel('收益率序列', {exact:true}).fill('[0.1,-0.1]');
  const request = page.waitForRequest(r => r.url().includes('/api/practice') && r.method() === 'POST');
  await panel.getByRole('button', {name:'运行实践'}).click();
  expect((await request).postDataJSON().inputs.returns).toEqual([0.1,-0.1]);
});


test('leaving a fully rendered data chart preserves the application', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await page.getByRole('button', {name:'数据探索',exact:true}).click();
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible();
  await page.getByRole('button', {name:'回测',exact:true}).click();
  await expect(page.getByRole('button', {name:'运行回测'})).toBeVisible();
  expect(errors).toEqual([]);
});

test('charts and concept diagrams remain readable across themes and widths', async ({ page }) => {
  test.setTimeout(120_000);
  for (const theme of ['dark', 'light']) {
    await page.goto('/');
    if (theme === 'light') await page.getByLabel('切换到浅色模式').click();
    for (const [module, label] of [['data', '数据探索'], ['backtest', '回测'], ['compare', '策略对比']]) {
      await page.getByRole('button', {name: label, exact:true}).click();
      if (module === 'backtest') await page.getByRole('button', {name:'运行回测'}).click();
      if (module === 'compare') await page.getByRole('button', {name:'跑对比'}).click();
      const chart = page.locator('.ax-chart');
      await expect(chart.locator('svg.main-svg').first()).toBeVisible();
      for (const width of [1440, 390]) {
        await page.setViewportSize({width, height:1000});
        await expect.poll(() => chart.evaluate(node => {
          const svg = node.querySelector('svg.main-svg')!.getBoundingClientRect();
          return svg.width >= node.clientWidth * .85 && svg.width <= node.getBoundingClientRect().width && svg.height <= node.getBoundingClientRect().height;
        })).toBe(true);
        if (module === 'data') expect(await chart.evaluate(node => node.clientHeight)).toBeGreaterThanOrEqual(698);
        expect(await chart.evaluate(node => {
          const frame = node.getBoundingClientRect();
          return [...node.querySelectorAll('.xtick text')].every(label => {
            const box = label.getBoundingClientRect();
            return box.left >= frame.left && box.right <= frame.right && box.bottom <= frame.bottom;
          });
        })).toBe(true);
        await expect(chart).toHaveScreenshot(`${theme}-${module}-${width}.png`, {maxDiffPixelRatio:0.01});
      }
    }
    await page.getByRole('button', {name:'学习中心', exact:true}).click();
    await page.getByPlaceholder('搜索 概念 / 公式 / 关键词').fill('EPS');
    const card = page.locator('.ax-kb-card');
    await card.locator('.ax-kb-details').click();
    await expect(card.locator('.ax-practice-reading')).toContainText('单次计算');
    for (const width of [1440, 390]) {
      await page.setViewportSize({width, height:1000});
      await expect(card).toHaveScreenshot(`${theme}-eps-${width}.png`, {maxDiffPixelRatio:0.01});
    }
  }
});

test.describe('local market time', () => {
  test.use({timezoneId:'Asia/Shanghai'});
  test('mobile date labels remain inside the plot in Shanghai time', async ({page}) => {
    await page.setViewportSize({width:390, height:844});
    await page.goto('/');
    await page.getByRole('button', {name:'回测', exact:true}).click();
    await page.getByRole('button', {name:'运行回测'}).click();
    const chart = page.locator('.ax-chart');
    await expect(chart.locator('.xtick text').first()).toBeVisible();
    expect(await chart.evaluate(node => {
      const frame = node.getBoundingClientRect();
      return [...node.querySelectorAll('.xtick text')].every(label => label.getBoundingClientRect().bottom <= frame.bottom);
    })).toBe(true);
  });
});
test('navigation, direct practice, and book reader have shareable URLs', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: '数据探索', exact: true }).click();
  await expect(page).toHaveURL(/\/data$/);
  await page.getByRole('button', { name: '学习中心', exact: true }).click();
  await expect(page).toHaveURL(/\/learn$/);
  await page.getByRole('button', { name: '在数据探索中实践' }).first().click();
  await expect(page).toHaveURL(/\/data\?concept=/);
  await expect(page.getByLabel('概念实践').locator('.ax-dd-trigger')).toHaveCount(0);
  await page.getByRole('button', { name: '学习中心', exact: true }).click();
  await page.getByRole('button', { name: '原书阅读', exact: true }).click();
  await expect(page).toHaveURL(/\/learn\/book$/);
  await expect(page.getByRole('navigation', { name: '原书目录' })).toBeVisible();
  await expect(page.locator('iframe[title="股票交易软件专业指标全解"]')).toHaveAttribute('src', /\/api\/book\/pdf/);
  await page.getByRole('button', { name: '指标大全', exact: true }).click();
  await expect(page.locator('.ax-kb-card').first().getByRole('link', { name: '↗ 源码' })).toHaveCount(0);
});

test('already-rendered Plotly chart immediately adopts the selected theme', async ({ page }) => {
  await page.goto('/data');
  const chart = page.locator('.ax-chart');
  await expect(chart.locator('svg.main-svg').first()).toBeVisible();
  const before = await chart.evaluate((node: any) => node.layout.paper_bgcolor);
  await page.getByLabel('切换到浅色模式').click();
  await expect.poll(() => chart.evaluate((node: any) => node.layout.paper_bgcolor)).not.toBe(before);
  expect(await chart.evaluate((node: any) => node.layout.font.color)).toBe(await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--text').trim()));
});

test('each of the four K-line patterns draws its actual candle structure', async ({ page }) => {
  const patterns = [['k_pattern_hammer', '锤子线'], ['k_pattern_doji', '十字星'], ['k_pattern_engulfing', '吞没形态'], ['k_pattern_star', '早晨之星']].map(([id, name]) => ({ id, name, category: 'K线形态', input_kind: 'market_bars', inputs: [], notes: '', summary: '', formula: '', meaning: '', example: '', signals: '', pitfalls: '', related: [], code_url: '', implementation: '' }));
  await page.route('**/api/knowledge', route => route.fulfill({ json: { total: 4, categories: { K线形态: patterns } } }));
  await page.goto('/');
  const cards = page.locator('.ax-kb-card');
  for (let index = 0; index < 4; index += 1) {
    await cards.nth(index).locator('.ax-kb-details').click();
    await expect(cards.nth(index).locator(`[data-candle-pattern="${patterns[index].id.replace('k_pattern_', '')}"]`)).toBeVisible();
  }
});

test('learning path has a shareable URL and remains selected after navigation', async ({ page }) => {
  await page.goto('/learn');
  await page.getByRole('button', { name: '学习路径', exact: true }).click();
  await expect(page).toHaveURL(/\/learn\/path$/);
  await expect(page.getByRole('heading', { name: '学习路径' })).toBeVisible();
  await expect(page.getByRole('button', { name: '学习路径', exact: true })).toHaveClass(/active/);
});

test('concept overview visualizes the full research-to-operation workflow and opens real module explanations', async ({ page }) => {
  await page.goto('/learn/concepts');
  const workflow = page.getByLabel('量化交易全流程');
  await expect(workflow).toBeVisible();
  await expect(workflow).toContainText('数据接入与治理');
  await expect(workflow).toContainText('清洗、复权、时间对齐');
  await expect(workflow).toContainText('回测与稳健性');
  await expect(workflow).toContainText('机器学习与大模型');
  await expect(workflow).toContainText('上线、监控与复盘');

  await page.getByRole('button', { name: /K线 \(Bar\).*查看模块说明/ }).click();
  const dialog = page.getByRole('dialog', { name: /K线 \(Bar\) 模块说明/ });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('src/types.rs');
  await expect(dialog.locator('svg')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
});

test('book contents uses an arrow-only accessible collapse control', async ({ page }) => {
  await page.goto('/learn/book');
  const toggle = page.getByRole('button', { name: '收起原书目录' });
  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveText('‹');
  await toggle.click();
  await expect(page.getByRole('button', { name: '展开原书目录' })).toHaveText('›');
  await expect(page.locator('.ax-book-reader')).toHaveClass(/toc-collapsed/);
});


test('learning path covers the runnable system and resolves a precise source link', async ({ page }) => {
  await page.goto('/learn/path');
  await expect(page.getByRole('heading', { name: '学习路径' })).toBeVisible();
  const steps = page.locator('.ax-learning-path > li');
  await expect(steps).toHaveCount(9);
  await expect(steps.first()).toContainText('动手验证');
  await expect(steps.nth(8)).toContainText('研究扩展：ML / LLM 与数据存储');
  const source = steps.first().getByRole('link', { name: '打开精确 GitHub 源码 ↗' });
  await expect(source).toHaveAttribute('href', /github\.com\/Sigma711\/axiom\/blob\/abc123\/src\/indicator\.rs#L42-L48$/);
});


test('every visible learning source reference is requested through the AST-backed resolver', async ({ page }) => {
  const refs = new Set<string>();
  page.on('request', request => {
    const url = new URL(request.url());
    if (url.pathname === '/api/code_loc') refs.add(url.searchParams.get('ref') || '');
  });
  await page.goto('/learn/concepts');
  const cards = page.locator('.ax-module-card');
  await expect(cards).toHaveCount(10);
  for (let index = 0; index < await cards.count(); index += 1) {
    await cards.nth(index).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('button', { name: '关闭模块说明' }).click();
  }
  await page.goto('/learn/path');
  await expect(page.locator('.ax-learning-path > li')).toHaveCount(9);
  await expect.poll(() => refs.size).toBe(11);
  expect([...refs].sort()).toEqual([
    'src/broker.rs::Broker', 'src/data.rs::DataFeed', 'src/engine.rs::BacktestEngine::run',
    'src/indicators/ma.rs::sma', 'src/metrics.rs::compute_metrics', 'src/paper.rs::run_paper_loop',
    'src/portfolio.rs::Portfolio::on_signal', 'src/risk.rs::RiskManager::allow_order',
    'src/strategy.rs::Strategy', 'src/types.rs::Bar', 'src/workflows.rs::entries',
  ]);
});


test('long dropdown menus stay above subsequent control groups', async ({ page }) => {
  const longStrategies = Array.from({ length: 12 }, (_, index) => ({ name: `long_${index}`, display_name: index === 8 ? '一目均衡表 Ichimoku' : `策略 ${index + 1}`, description: 'fixture', params: index === 8 ? [{ key: 'fast', label: '转换线周期', default: 9, min: 2, max: 30 }] : [] }));
  await page.route('**/api/strategies', route => route.fulfill({ json: { strategies: longStrategies } }));
  await page.goto('/backtest');
  const trigger = page.getByRole('button', { name: '策略', exact: true });
  await trigger.click();
  await page.getByRole('option', { name: '一目均衡表 Ichimoku', exact: true }).click();
  await expect(page.getByLabel('转换线周期')).toBeVisible();
  await trigger.click();
  const menu = page.locator('.ax-dd-menu');
  await expect(menu).toBeVisible();
  const coveredByMenu = await menu.evaluate(node => {
    const rect = node.getBoundingClientRect();
    const point = document.elementFromPoint(rect.left + 20, rect.top + Math.min(155, rect.height - 12));
    return point?.closest('.ax-dd-menu') === node;
  });
  expect(coveredByMenu).toBe(true);
});
