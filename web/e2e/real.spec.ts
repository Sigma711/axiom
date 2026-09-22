import { expect, test } from './v8-coverage';

test('real Rust service supports the four-module learning journey', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  const selectSynthetic = async () => {
    await page.getByRole('button', { name: '数据源', exact: true }).click();
    await page.getByRole('option', { name: '加密货币 · Binance' }).click();
  };
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '学习中心' })).toBeVisible();
  await page.getByRole('button', { name: '在数据探索中实践' }).first().click();
  await expect(page.getByRole('heading', { name: '数据探索' })).toBeVisible();
  await expect(page.getByLabel('概念实践')).toBeVisible();
  await page.getByRole('button', { name: '数据源', exact: true }).click();
  await page.getByRole('option', { name: '加密货币 · Binance' }).click();
  await page.getByRole('button', { name: '加载数据' }).click();
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
  await page.getByRole('button', {name:'运行实践'}).click();
  await expect(page.locator('.ax-practice-result')).toContainText('已计算');
  await page.getByRole('button', { name: '回测', exact: true }).click();
  await expect(page.getByRole('button', { name: '运行回测' })).toBeEnabled();
  await page.getByRole('button', { name: '数据源' }).click();
  await page.getByRole('option', { name: '加密货币 · Binance' }).click();
  const [backtestResponse] = await Promise.all([
    page.waitForResponse(response => response.url().includes('/api/backtest')),
    page.getByRole('button', { name: '运行回测' }).click(),
  ]);
  expect(backtestResponse.ok()).toBe(true);
  await expect(page.locator('.ax-metrics')).toBeVisible({ timeout: 20_000 });
  await page.getByRole('button', { name: '策略对比', exact: true }).click();
  await expect(page.getByRole('button', { name: '跑对比' })).toBeEnabled();
  await selectSynthetic();
  await page.getByRole('button', { name: '跑对比' }).click();
  await expect.poll(() => page.locator('.ax-cmp-table tbody tr').count(), { timeout: 30_000 }).toBeGreaterThanOrEqual(2);
  await page.getByRole('button', { name: '模拟盘', exact: true }).click();
  await expect(page.getByRole('heading', { name: '模拟盘' })).toBeVisible();
  await page.getByRole('button', { name: '策略', exact: true }).click();
  await page.getByText('买入持有 (基准)', { exact: true }).click();
  await expect(page.getByRole('button', { name: /启动/ })).toBeEnabled();
  await Promise.all([
    page.waitForResponse(response => response.url().includes('/api/paper/config') && response.ok()),
    page.waitForResponse(response => response.url().includes('/api/paper/start') && response.ok()),
    page.getByRole('button', { name: /启动/ }).click(),
  ]);
  await expect(page.getByRole('button', { name: /停止/ })).toBeEnabled({ timeout: 8_000 });
  await expect(page.locator('.ax-paper-stats')).not.toContainText('最新价 —', { timeout: 8_000 });
  await expect(page.locator('.ax-paper-stats')).toContainText(/成交笔数\s*1/, { timeout: 12_000 });
  await expect(page.locator('.ax-paper-stats')).toContainText(/交易对\s*\S+/);
  await expect(page.locator('.ax-paper-stats')).toContainText(/数据源\s*(?!—)\S+/);
  await Promise.all([page.waitForResponse(response => response.url().includes('/api/paper/stop') && response.ok()), page.getByRole('button', { name: /停止/ }).click()]);
  await expect(page.getByRole('button', { name: /启动/ })).toBeEnabled({ timeout: 8_000 });
  await expect(page).toHaveScreenshot('real-four-module-journey.png', { fullPage: false, maxDiffPixelRatio: 0.03 });
  const pages = [{ label: 'data', tab: '数据探索' }, { label: 'backtest', tab: '回测' }, { label: 'paper', tab: '模拟盘' }, { label: 'compare', tab: '策略对比' }];
  const waitForChartResize = async () => {
    await expect.poll(() => page.locator('.ax-chart').evaluateAll(charts => charts.every(chart => {
      const svg = chart.querySelector('svg.main-svg');
      return !svg || (svg.getBoundingClientRect().width >= chart.clientWidth * 0.85 && svg.getBoundingClientRect().width <= chart.getBoundingClientRect().width);
    }))).toBe(true);
  };
  for (const theme of ['dark', 'light'] as const) {
    if (theme === 'light') await page.getByLabel('切换到浅色模式').click();
    for (const item of pages) {
      await page.getByRole('button', { name: item.tab, exact: true }).click();
      if (item.label !== 'paper') await selectSynthetic();
      if (item.label === 'backtest' && await page.locator('.ax-metrics').count() === 0) {
        await page.getByRole('button', { name: '运行回测' }).click();
        await expect(page.locator('.ax-metrics')).toBeVisible({ timeout: 20_000 });
      }
      if (item.label === 'compare' && await page.locator('.ax-cmp-table tbody tr').count() === 0) {
        await page.getByRole('button', { name: '跑对比' }).click();
        await expect.poll(() => page.locator('.ax-cmp-table tbody tr').count(), { timeout: 30_000 }).toBeGreaterThanOrEqual(2);
      }
      await expect(page.locator('main')).toBeVisible();
      await page.setViewportSize({ width: 1440, height: 1000 });
      await waitForChartResize();
      await page.screenshot({ path: testInfo.outputPath(`${theme}-${item.label}-desktop.png`), fullPage: false });
      await page.setViewportSize({ width: 390, height: 844 });
      await waitForChartResize();
      await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      await page.screenshot({ path: testInfo.outputPath(`${theme}-${item.label}-mobile.png`), fullPage: false });
    }
  }
  expect(errors).toEqual([]);
});

test('every rendered knowledge card opens one meaningful SVG illustration without retaining hidden charts', async ({ page }) => {
  test.setTimeout(180_000);
  await page.goto('/');
  const cards = page.locator('.ax-kb-card');
  await expect.poll(() => cards.count(), { timeout: 30_000 }).toBeGreaterThan(0);
  const total = await cards.count();
  expect(total).toBeGreaterThan(0);
  for (let index = 0; index < total; index += 1) {
    await cards.nth(index).locator('.ax-kb-details').click();
    await expect(cards.nth(index).locator('.ax-knowledge-chart svg')).toBeVisible({ timeout: 30_000 });
    if (process.env.CI) await expect(cards.nth(index).locator('.ax-code-link')).toHaveAttribute('href', /^https:\/\/github\.com\/Sigma711\/axiom\/blob\/[0-9a-f]{40}\/src\/.+#L[1-9]\d*-L[1-9]\d*$/);
  }
  await expect(page.locator('.ax-knowledge-chart svg')).toHaveCount(1);
});

test('browser switches among three live markets with matching symbols and valid candles', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/data');
  await expect(page.getByRole('heading', { name: '数据探索' })).toBeVisible();
  for (const market of [
    { source: 'a_share', label: 'A 股 · 公开行情', symbol: '600519' },
    { source: 'us_stock', label: '美股 · 公开行情', symbol: 'AAPL' },
    { source: 'binance', label: '加密货币 · Binance', symbol: 'BTCUSDT' },
  ]) {
    await page.getByRole('button', { name: '数据源', exact: true }).click();
    const responsePromise = page.waitForResponse(response => {
      const url = new URL(response.url());
      return url.pathname.endsWith('/api/indicators')
        && url.searchParams.get('source') === market.source
        && url.searchParams.get('symbol') === market.symbol;
    }, { timeout: 35_000 });
    await page.getByRole('option', { name: market.label }).click();
    await expect(page.getByRole('button', { name: '交易对' })).toContainText(market.symbol);
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const payload = await response.json();
    expect(payload.source).toBe(market.source);
    expect(payload.symbol).toBe(market.symbol);
    expect(payload.bars.length).toBeGreaterThan(40);
    for (const bar of payload.bars) {
      expect(Number.isFinite(bar.open)).toBe(true);
      expect(Number.isFinite(bar.high)).toBe(true);
      expect(Number.isFinite(bar.low)).toBe(true);
      expect(Number.isFinite(bar.close)).toBe(true);
      expect(bar.low).toBeLessThanOrEqual(Math.min(bar.open, bar.close));
      expect(bar.high).toBeGreaterThanOrEqual(Math.max(bar.open, bar.close));
    }
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible();
    await expect(page.locator('.ax-summary')).toContainText(market.symbol);
  }
});


test('real market selections stay aligned across backtest comparison and paper', async ({ page }) => {
  test.setTimeout(180_000);
  const markets = [
    { source: 'a_share', symbol: '600519', label: '\u0041 \u80a1 \u00b7 \u516c\u5f00\u884c\u60c5' },
    { source: 'us_stock', symbol: 'AAPL', label: '\u7f8e\u80a1 \u00b7 \u516c\u5f00\u884c\u60c5' },
  ] as const;
  const selectMarket = async (market: (typeof markets)[number]) => {
    await page.getByRole('button', { name: /\u6570\u636e\u6e90/ }).click();
    await page.getByRole('option', { name: market.label, exact: true }).click();
    await expect(page.getByRole('button', { name: /\u4ea4\u6613\u5bf9/ })).toContainText(market.symbol);
  };
  const assertBars = (payload: any) => {
    expect(payload.bars.length).toBeGreaterThan(40);
    expect(payload.bars.every((bar: any) =>
      [bar.open, bar.high, bar.low, bar.close, bar.volume].every((value: number) => Number.isFinite(value)) &&
      bar.low <= Math.min(bar.open, bar.close) &&
      bar.high >= Math.max(bar.open, bar.close),
    )).toBe(true);
  };

  await page.goto('/backtest');
  await expect(page.getByRole('button', { name: /\u8fd0\u884c\u56de\u6d4b/ })).toBeEnabled();
  for (const market of markets) {
    await selectMarket(market);
    const requestPromise = page.waitForRequest(request => {
      const url = new URL(request.url());
      return request.method() === 'POST' && url.pathname.endsWith('/api/backtest');
    });
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/backtest') && response.request().method() === 'POST');
    await page.getByRole('button', { name: /\u8fd0\u884c\u56de\u6d4b/ }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    const body = request.postDataJSON() as { source: string; symbol: string };
    expect(body.source).toBe(market.source);
    expect(body.symbol).toBe(market.symbol);
    expect(response.ok()).toBe(true);
    const payload = await response.json();
    assertBars(payload);
    expect(payload.equity_curve.length).toBeGreaterThan(40);
    await expect(page.locator('.ax-metrics')).toBeVisible({ timeout: 20_000 });
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
  }

  await page.goto('/compare');
  await expect.poll(() => page.locator('.ax-pool-item').count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);
  await page.getByRole('button', { name: /\u5168\u4e0d\u9009/ }).click();
  const pool = page.locator('.ax-pool-item');
  await expect(page.locator('.ax-pool-item.checked')).toHaveCount(0);
  await pool.nth(0).locator('input').click();
  await pool.nth(1).locator('input').click();
  await expect(page.locator('.ax-pool-item.checked')).toHaveCount(2);
  const comparisonRequests: any[] = [];
  const onRequest = (request: import('@playwright/test').Request) => {
    const url = new URL(request.url());
    if (request.method() === 'POST' && url.pathname.endsWith('/api/backtest')) comparisonRequests.push(request.postDataJSON());
  };
  page.on('request', onRequest);
  for (const market of markets) {
    comparisonRequests.length = 0;
    await selectMarket(market);
    await page.getByRole('button', { name: /\u8dd1\u5bf9\u6bd4/ }).click();
    await expect.poll(() => comparisonRequests.length, { timeout: 45_000 }).toBe(2);
    for (const body of comparisonRequests) {
      expect(body.source).toBe(market.source);
      expect(body.symbol).toBe(market.symbol);
    }
    await expect.poll(() => page.locator('.ax-cmp-table tbody tr').count(), { timeout: 30_000 }).toBe(2);
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
  }
  page.off('request', onRequest);

  await page.goto('/paper');
  await expect(page.getByRole('heading', { name: /\u6a21\u62df\u76d8/ })).toBeVisible();
  await expect(page.locator('.ax-paper-stats')).toBeVisible({ timeout: 20_000 });
  for (const market of markets) {
    await selectMarket(market);
    const requestPromise = page.waitForRequest(request => {
      const url = new URL(request.url());
      return request.method() === 'POST' && url.pathname.endsWith('/api/paper/config');
    });
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/paper/config') && response.request().method() === 'POST');
    await page.getByRole('button', { name: /\u5e94\u7528\u5e02\u573a/ }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    const body = request.postDataJSON() as { source: string; symbol: string };
    expect(body.source).toBe(market.source);
    expect(body.symbol).toBe(market.symbol);
    expect(response.ok()).toBe(true);
    const payload = await response.json();
    expect(payload.source).toBe(market.source);
    expect(payload.symbol).toBe(market.symbol);
    await expect(page.locator('.ax-paper-stats')).toContainText(market.symbol);
    await expect(page.locator('.ax-paper-stats')).not.toContainText(/\u4ea4\u6613\u5bf9\s+—/);
    await expect(page.locator('.ax-paper-stats')).not.toContainText(/\u6570\u636e\u6e90\s+—/);
  }
});


test('knowledge practice opens the applicable module and market instead of forcing data exploration', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/');
  const search = page.getByPlaceholder('搜索 概念 / 公式 / 关键词');
  await search.fill('Sharpe 夏普比率');
  const sharpe = page.locator('.ax-kb-card').filter({ has: page.getByRole('heading', { name: 'Sharpe 夏普比率', exact: true }) });
  await sharpe.getByRole('button', { name: '在回测中实践' }).click();
  await expect(page).toHaveURL(/\/backtest\?concept=sharpe&source=binance/);
  await expect(page.getByLabel('概念实践')).toContainText('Sharpe 夏普比率');
  await page.goto('/');
  await search.fill('ROE 净资产收益率');
  const roe = page.locator('.ax-kb-card').filter({ has: page.getByRole('heading', { name: 'ROE 净资产收益率', exact: true }) });
  await roe.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=roe&source=a_share/);
  await expect(page.getByRole('button', { name: '数据源', exact: true })).toContainText('A 股');
  await expect(page.getByRole('button', { name: '交易对' })).toContainText('600519');
  await expect(page.getByLabel('概念实践')).toContainText('ROE 净资产收益率');
  await expect(page.getByLabel('概念实践')).toContainText('教学示例');
});


test('performance practice derives Sharpe and benchmark metrics from the selected real backtest', async ({ page }) => {
  test.setTimeout(120_000);
  for (const concept of ['sharpe', 'information_ratio']) {
    await page.goto(`/backtest?concept=${concept}&source=binance`);
    const panel = page.getByLabel('概念实践');
    await expect(panel.getByRole('button', { name: '运行实践' })).toBeVisible();
    const premature = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST', { timeout: 500 }).catch(() => null);
    await panel.getByRole('button', { name: '运行实践' }).click();
    expect(await premature).toBeNull();
    await expect(panel).toContainText('先在当前模块运行真实回测');
    await page.getByRole('button', { name: '运行回测' }).click();
    await expect(page.locator('.ax-metrics')).toBeVisible({ timeout: 30_000 });
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const request = response.request().postDataJSON();
    expect(request.module).toBe('backtest');
    expect(request.bars.length).toBeGreaterThan(40);
    if (concept === 'information_ratio') {
      expect(request.inputs.strategy_returns.length).toBeGreaterThan(1);
      expect(request.inputs.benchmark_returns.length).toBe(request.inputs.strategy_returns.length);
    }
    const payload = await response.json();
    expect(payload.provenance).toBe('provided_result_context');
    await expect(panel).toContainText('使用当前模块真实结果');
  }
});

test('five nonstandard chart practices draw only observed market reconstructions', async ({ page }) => {
  test.setTimeout(120_000);
  const cases = [
    ['book_chart_heikin_ashi', 'heikin_ashi', 'ohlc'],
    ['book_chart_renko', 'renko', 'close'],
    ['book_chart_point_figure', 'point_figure', 'close'],
    ['book_chart_kagi', 'kagi', 'close'],
    ['book_chart_three_line_break', 'three_line_break', 'close'],
  ] as const;
  for (const [concept, kind, sourcePrice] of cases) {
    await page.goto(`/data?concept=${concept}&source=binance`);
    await page.getByRole('button', { name: '加载数据' }).click();
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
    const panel = page.getByLabel('概念实践');
    await expect(panel).not.toContainText('有序收盘价格（元）');
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const payload = await response.json();
    expect(payload.provenance).toBe('provided_market_bars');
    expect(payload.chart.source_price).toBe(sourcePrice);
    expect(payload.chart.source_bar_count).toBeGreaterThan(20);
    await expect(panel.locator(`svg[data-chart-kind="${kind}"]`)).toBeVisible();
    await expect(panel).toContainText('图形价不是成交价');
    await expect(panel).toContainText('基于当前标的已收盘行情');
    await expect(panel).toContainText('报价单位');
  }
});

test('rolling 24-hour volume sums the latest completed Binance hours without editable fixtures', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/data?concept=book_volume_24h&source=binance');
  await page.getByRole('button', { name: '加载数据' }).click();
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBe(true);
  const request = response.request().postDataJSON();
  const bars = request.bars as Array<{ volume: number }>;
  const expected = bars.slice(-24).reduce((sum, bar) => sum + bar.volume, 0);
  const payload = await response.json();
  expect(payload.provenance).toBe('provided_market_bars');
  expect(payload.values.rolling_24h_volume).toBeCloseTo(expected, 8);
  await expect(panel).toContainText('基于当前标的已收盘行情');
  await expect(panel.getByRole('img', { name: /24 小时成交量.*全部序列/ })).toBeVisible();
  await expect(panel).toContainText('每小时成交量');
});

test('log-return practice uses the latest two observed closes across the current market', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/data?concept=book_log_return&source=binance');
  await page.getByRole('button', { name: '加载数据' }).click();
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBe(true);
  const request = response.request().postDataJSON();
  const closes = (request.bars as Array<{ close: number }>).map(bar => bar.close);
  const payload = await response.json();
  expect(payload.provenance).toBe('provided_market_bars');
  expect(payload.values.book_log_return).toBeCloseTo(Math.log(closes.at(-1)! / closes.at(-2)!), 10);
  await expect(panel).toContainText('基于当前标的已收盘行情');
});
