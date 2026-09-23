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
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('.ax-summary')).toContainText('BTCUSDT');
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
  await page.getByRole('option', { name: '买入持有 (基准)', exact: true }).click();
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

test('repainting practice fetches completed real bars and keeps confirmation after the pivot', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('重画');
  const card = page.locator('.ax-kb-card').filter({ hasText: 'ZigZag、分形和部分自动形态会重画或延迟确认' });
  await expect(card).toHaveCount(1, { timeout: 30_000 });
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page.getByRole('heading', { name: '数据探索' })).toBeVisible();
  await expect(page.getByLabel('概念实践')).toContainText('ZigZag、分形和部分自动形态会重画或延迟确认');
  await expect(page.locator('.ax-practice-inputs')).toHaveCount(0);

  await page.getByRole('button', { name: '数据源', exact: true }).click();
  await page.getByRole('option', { name: '加密货币 · Binance' }).click();
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
  await page.getByRole('button', { name: '运行实践' }).click();
  const [request, response] = await Promise.all([requestPromise, responsePromise]);
  const body = request.postDataJSON() as { concept_id: string; source: string; bars?: unknown; inputs: unknown };
  expect(body.concept_id).toBe('book_pitfall_repainting');
  expect(body.source).toBe('binance');
  expect(body.bars).toBeUndefined();
  expect(body.inputs).toEqual({});
  expect(response.ok(), await response.text()).toBe(true);
  const payload = await response.json();
  expect(payload.provenance).toBe('provided_market_bars');
  expect(payload.context).toBe('selected_dataset');
  expect(payload.bar_origin).toBe('server_fetched_completed_source_bars');
  expect(payload.bars.length).toBeGreaterThan(40);
  expect(payload.bars.every((bar: { timestamp: string }) => new Date(bar.timestamp).getTime() <= Date.now())).toBe(true);
  const series = Object.fromEntries(payload.series.map((item: { name: string; values: Array<number | null> }) => [item.name, item.values]));
  const confirmedHigh = series.confirmed_pivot_high as Array<number | null>;
  const confirmedLow = series.confirmed_pivot_low as Array<number | null>;
  const occurrenceHigh = series.pivot_high_occurrence as Array<number | null>;
  const occurrenceLow = series.pivot_low_occurrence as Array<number | null>;
  const delay = series.confirmation_delay_bars as Array<number | null>;
  const confirmations = [
    { confirmed: confirmedHigh, occurrence: occurrenceHigh },
    { confirmed: confirmedLow, occurrence: occurrenceLow },
  ];
  let confirmationCount = 0;
  for (const { confirmed, occurrence } of confirmations) {
    for (const index of confirmed.map((value, index) => value == null ? -1 : index).filter(index => index >= 0)) {
      confirmationCount += 1;
      expect(index).toBeGreaterThanOrEqual(2);
      expect(occurrence[index - 2]).not.toBeNull();
      expect(confirmed[index - 2]).toBeNull();
      expect(delay[index]).toBe(2);
    }
  }
  expect(confirmationCount).toBeGreaterThan(0);
  await expect(page.locator('.ax-practice-result')).toContainText('服务器重新获取并过滤已收盘行情');
  await expect.poll(() => page.locator('.ax-practice-result circle[data-series-marker]').count()).toBeGreaterThan(0);
  await expect.poll(() => page.locator('.ax-practice-result circle[data-series-marker="pivot_high_occurrence"], .ax-practice-result circle[data-series-marker="pivot_low_occurrence"]').count()).toBeGreaterThan(0);
  await expect(page.locator('.ax-practice-result')).toContainText('t+2');
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
  for (const concept of ['sharpe', 'information_ratio', 'book_r_squared']) {
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
    if (concept === 'information_ratio' || concept === 'book_r_squared') {
      expect(request.inputs.strategy_returns.length).toBeGreaterThan(1);
      expect(request.inputs.benchmark_returns.length).toBe(request.inputs.strategy_returns.length);
    }
    if (concept === 'book_r_squared') {
      expect(request.inputs.equity.length === request.bars.length || request.inputs.equity.length === request.bars.length + 1).toBe(true);
      expect(request.inputs.equity_points).toHaveLength(request.bars.length);
      expect(request.inputs.equity_points.map((point: { timestamp: string }) => Date.parse(point.timestamp))).toEqual(request.bars.map((bar: { timestamp: string }) => Date.parse(bar.timestamp)));
      for (let index = 1; index < request.bars.length; index += 1) {
        const offset = request.inputs.equity.length === request.bars.length + 1 ? 1 : 0;
        expect(request.inputs.strategy_returns[index - 1]).toBeCloseTo(request.inputs.equity[offset + index] / request.inputs.equity[offset + index - 1] - 1, 12);
        expect(request.inputs.benchmark_returns[index - 1]).toBeCloseTo(request.bars[index].close / request.bars[index - 1].close - 1, 12);
      }
    }
    const payload = await response.json();
    if (concept === 'book_r_squared') {
      const strategy = request.inputs.strategy_returns as number[], benchmark = request.inputs.benchmark_returns as number[];
      const mean = (values: number[]) => values.reduce((total, value) => total + value, 0) / values.length;
      const a = mean(strategy), b = mean(benchmark);
      const correlation = strategy.reduce((total, value, index) => total + (value - a) * (benchmark[index] - b), 0) /
        Math.sqrt(strategy.reduce((total, value) => total + (value - a) ** 2, 0) * benchmark.reduce((total, value) => total + (value - b) ** 2, 0));
      expect(payload.values.r_squared).toBeCloseTo(correlation ** 2, 10);
    }
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

test('CDP practice derives the last completed A-share session levels from its predecessor', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/data?concept=book_cdp&source=a_share');
  await page.getByRole('button', { name: '加载数据' }).click();
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBe(true);
  const request = response.request().postDataJSON();
  expect(request.source).toBe('a_share');
  expect(request.inputs).toEqual({});
  const bars = request.bars as Array<{ timestamp: string; high: number; low: number; close: number }>;
  expect(bars.length).toBeGreaterThanOrEqual(2);
  const previous = bars.at(-2)!;
  const p = (previous.high + previous.low + 2 * previous.close) / 4;
  const payload = await response.json();
  expect(payload.provenance).toBe('provided_market_bars');
  expect(payload.values.cdp).toBeCloseTo(p, 8);
  expect(payload.values.ah).toBeCloseTo(p + previous.high - previous.low, 8);
  expect(payload.values.al).toBeCloseTo(p - previous.high + previous.low, 8);
  expect(payload.values.nh).toBeCloseTo(2 * p - previous.low, 8);
  expect(payload.values.nl).toBeCloseTo(2 * p - previous.high, 8);
  expect(JSON.stringify(payload.notes)).toContain(bars.at(-1)!.timestamp.slice(0, 10));
  await expect(panel).toContainText('基于当前标的已收盘行情');
});

test('annualized volatility binds its calculation and annualization to the selected real market cadence', async ({ page }) => {
  test.setTimeout(120_000);
  for (const market of [
    { source: 'binance', periodsPerYear: 8760, note: '8760小时/年' },
    { source: 'a_share', periodsPerYear: 252, note: '252个交易日/年' },
  ]) {
    await page.goto(`/data?concept=book_annualized_volatility&source=${market.source}`);
    await page.getByRole('button', { name: '加载数据' }).click();
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 30_000 });
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const request = response.request().postDataJSON();
    expect(request.source).toBe(market.source);
    expect(request.inputs).toEqual({});
    const closes = (request.bars as Array<{ close: number }>).map(bar => bar.close);
    const returns = closes.slice(1).map((close, index) => close / closes[index] - 1);
    const mean = returns.reduce((sum, value) => sum + value, 0) / returns.length;
    const sampleStddev = Math.sqrt(returns.reduce((sum, value) => sum + (value - mean) ** 2, 0) / (returns.length - 1));
    const payload = await response.json();
    expect(payload.provenance).toBe('provided_market_bars');
    expect(payload.values.periods_per_year).toBe(market.periodsPerYear);
    expect(payload.values.annualized_volatility).toBeCloseTo(sampleStddev * Math.sqrt(market.periodsPerYear), 8);
    expect(JSON.stringify(payload.notes)).toContain(market.note);
    await expect(panel).toContainText('年化波动率');
    await expect(panel).toContainText('每年期数');
    await expect(panel).toContainText('年化比例（小数）');
  }
});


test('RVAT uses completed Binance UTC-hour prefixes without editable observations', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/data?concept=book_relative_volume_at_time&source=binance');
  await page.getByRole('button', { name: '加载数据' }).click();
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBe(true);
  const request = response.request().postDataJSON();
  const payload = await response.json();
  expect(payload.provenance).toBe('provided_market_bars');
  expect(payload.values.historical_sample_count).toBe(7);
  expect(payload.notes.join(' ')).toContain('Binance 1小时已收盘K线');
  const bars = request.bars as { timestamp: string; volume: number }[];
  const cutoff = new Date(bars.at(-1)!.timestamp).getUTCHours();
  const needed = 7 * 24 + cutoff + 1;
  const observed = bars.slice(-needed);
  expect(observed).toHaveLength(needed);
  for (let index = 1; index < observed.length; index += 1) {
    expect(Date.parse(observed[index].timestamp) - Date.parse(observed[index - 1].timestamp)).toBe(3_600_000);
  }
  const history = Array.from({ length: 7 }, (_, day) =>
    observed.slice(day * 24, day * 24 + cutoff + 1).reduce((sum, bar) => sum + bar.volume, 0));
  const current = observed.slice(7 * 24).reduce((sum, bar) => sum + bar.volume, 0);
  const expected = current / (history.reduce((sum, value) => sum + value, 0) / 7);
  expect(payload.values.current_cumulative_volume).toBeCloseTo(current, 10);
  expect(payload.values.relative_volume_at_time).toBeCloseTo(expected, 10);
  expect(request.inputs).toEqual({});
});


test('nonstandard OHLC4 practice uses observed candles and separates synthetic from executable price', async ({ page }) => {
  test.setTimeout(120_000);
  for (const source of ['binance', 'a_share', 'us_stock']) {
    await page.goto(`/data?concept=book_nonstandard_bar&source=${source}`);
    await page.getByRole('button', { name: '加载数据' }).click();
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 30_000 });
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const request = response.request().postDataJSON();
    expect(request.source).toBe(source);
    expect(request.inputs).toEqual({});
    const last = (request.bars as Array<{ open: number; high: number; low: number; close: number }>).at(-1)!;
    const expected = (last.open + last.high + last.low + last.close) / 4;
    const payload = await response.json();
    expect(payload.provenance).toBe('provided_market_bars');
    expect(payload.values.book_nonstandard_bar).toBeCloseTo(expected, 10);
    expect(payload.values.actual_close).toBeCloseTo(last.close, 10);
    expect(payload.values.synthetic_minus_close).toBeCloseTo(expected - last.close, 10);
    expect(JSON.stringify(payload.notes)).toContain('不可作为成交价');
  }
});

test('period practice uses each real source contract and labels observation gaps', async ({ page }) => {
  test.setTimeout(120_000);
  for (const source of ['binance', 'a_share', 'us_stock'] as const) {
    await page.goto(`/data?concept=book_period&source=${source}`);
    await page.getByRole('button', { name: '加载数据' }).click();
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 30_000 });
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const request = response.request().postDataJSON();
    const payload = await response.json();
    const bars = request.bars as Array<{ timestamp: string }>;
    const expected = source === 'binance' ? 3_600 : 86_400;
    expect(request.inputs).toEqual({});
    expect(payload.provenance).toBe('provided_market_bars');
    expect(payload.values.book_period).toBe(expected);
    expect(payload.values.bar_count).toBe(bars.length);
    if (bars.length > 1) {
      expect(payload.values.last_observed_interval_seconds).toBe((Date.parse(bars.at(-1)!.timestamp) - Date.parse(bars.at(-2)!.timestamp)) / 1000);
    }
    for (const bar of bars) {
      const date = new Date(bar.timestamp);
      if (source === 'binance') expect(date.getUTCMinutes()).toBe(0);
      else expect([date.getUTCHours(), date.getUTCMinutes(), date.getUTCSeconds()]).toEqual([0, 0, 0]);
    }
  }
});

test('rolling correlation rebuilds strategy and benchmark returns from each live market result', async ({ page }) => {
  test.setTimeout(180_000);
  const correlation = (x: number[], y: number[]) => {
    const xMean = x.reduce((sum, value) => sum + value, 0) / x.length;
    const yMean = y.reduce((sum, value) => sum + value, 0) / y.length;
    const numerator = x.reduce((sum, value, index) => sum + (value - xMean) * (y[index] - yMean), 0);
    const denominator = Math.sqrt(
      x.reduce((sum, value) => sum + (value - xMean) ** 2, 0) *
      y.reduce((sum, value) => sum + (value - yMean) ** 2, 0),
    );
    return denominator === 0 ? null : numerator / denominator;
  };
  for (const market of [
    { source: 'binance', label: '加密货币 · Binance' },
    { source: 'a_share', label: 'A 股 · 公开行情' },
    { source: 'us_stock', label: '美股 · 公开行情' },
  ]) {
    await page.goto('/backtest?concept=rolling_correlation&source=' + market.source);
    await page.getByRole('button', { name: '数据源', exact: true }).click();
    await page.getByRole('option', { name: market.label, exact: true }).click();
    await page.getByRole('button', { name: '运行回测' }).click();
    await expect(page.locator('.ax-metrics')).toBeVisible({ timeout: 35_000 });
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs input')).toHaveCount(1);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const request = response.request().postDataJSON();
    expect(request.source).toBe(market.source);
    expect(request.inputs.series_x).toEqual(request.inputs.strategy_returns);
    expect(request.inputs.series_y).toEqual(request.inputs.benchmark_returns);
    expect(request.inputs.equity_points.map((point: { timestamp: string }) => Date.parse(point.timestamp))).toEqual(request.bars.map((bar: { timestamp: string }) => Date.parse(bar.timestamp)));
    const strategy = request.inputs.strategy_returns as number[];
    const benchmark = request.inputs.benchmark_returns as number[];
    const period = request.inputs.period as number;
    expect(period).toBeGreaterThanOrEqual(2);
    expect(period).toBeLessThanOrEqual(strategy.length);
    const expected = strategy.map((_: number, index: number) => index < period - 1 ? null : correlation(
      strategy.slice(index + 1 - period, index + 1),
      benchmark.slice(index + 1 - period, index + 1),
    ));
    const payload = await response.json();
    const actual = payload.series.find((series: { name: string }) => series.name === 'rolling_correlation').values;
    expect(actual).toHaveLength(expected.length);
    for (let index = 0; index < expected.length; index += 1) {
      const expectedValue = expected[index];
      if (expectedValue == null) expect(actual[index]).toBeNull();
      else expect(actual[index]).toBeCloseTo(expectedValue, 10);
    }
    expect(payload.provenance).toBe('provided_result_context');
    await expect(panel).toContainText('不是双资产配对交易证据');
  }
});

test('rolling correlation accepts aligned real compare and paper results', async ({ page }) => {
  test.setTimeout(150_000);
  const assertPractice = async (module: 'compare' | 'paper') => {
    const panel = page.getByLabel('概念实践');
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const request = response.request().postDataJSON();
    expect(request.module).toBe(module);
    expect(request.inputs.series_x).toEqual(request.inputs.strategy_returns);
    expect(request.inputs.series_y).toEqual(request.inputs.benchmark_returns);
    expect(request.inputs.equity_points.map((point: { timestamp: string }) => Date.parse(point.timestamp))).toEqual(request.bars.map((bar: { timestamp: string }) => Date.parse(bar.timestamp)));
    const offset = request.inputs.equity.length === request.bars.length + 1 ? 1 : 0;
    for (let index = 1; index < request.bars.length; index += 1) {
      expect(request.inputs.strategy_returns[index - 1]).toBeCloseTo(
        request.inputs.equity[offset + index] / request.inputs.equity[offset + index - 1] - 1, 12,
      );
      expect(request.inputs.benchmark_returns[index - 1]).toBeCloseTo(
        request.bars[index].close / request.bars[index - 1].close - 1, 12,
      );
    }
    const strategy = request.inputs.strategy_returns as number[];
    const benchmark = request.inputs.benchmark_returns as number[];
    const period = request.inputs.period as number;
    const correlation = (x: number[], y: number[]) => {
      const xMean = x.reduce((sum, value) => sum + value, 0) / x.length;
      const yMean = y.reduce((sum, value) => sum + value, 0) / y.length;
      const numerator = x.reduce((sum, value, index) => sum + (value - xMean) * (y[index] - yMean), 0);
      const denominator = Math.sqrt(
        x.reduce((sum, value) => sum + (value - xMean) ** 2, 0) *
        y.reduce((sum, value) => sum + (value - yMean) ** 2, 0),
      );
      return denominator === 0 ? null : numerator / denominator;
    };
    const payload = await response.json();
    const values = payload.series.find((series: { name: string }) => series.name === 'rolling_correlation').values;
    expect(values.slice(0, period - 1).every((value: unknown) => value == null)).toBe(true);
    for (const index of [period - 1, strategy.length - 1]) {
      const expected = correlation(
        strategy.slice(index + 1 - period, index + 1),
        benchmark.slice(index + 1 - period, index + 1),
      );
      if (expected == null) expect(values[index]).toBeNull();
      else expect(values[index]).toBeCloseTo(expected, 10);
    }
    expect(payload.provenance).toBe('provided_result_context');
  };

  await page.goto('/compare?concept=rolling_correlation&source=binance');
  await expect.poll(() => page.locator('.ax-pool-item').count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);
  await page.getByRole('button', { name: '跑对比' }).click();
  await expect.poll(() => page.locator('.ax-cmp-table tbody tr').count(), { timeout: 35_000 }).toBeGreaterThanOrEqual(2);
  await assertPractice('compare');

  await page.goto('/paper?concept=rolling_correlation&source=binance');
  await page.getByRole('button', { name: '策略', exact: true }).click();
  await page.getByRole('option', { name: '买入持有 (基准)', exact: true }).click();
  await expect(page.getByRole('button', { name: /启动/ })).toBeEnabled({ timeout: 20_000 });
  await Promise.all([
    page.waitForResponse(response => response.url().includes('/api/paper/start') && response.ok()),
    page.getByRole('button', { name: /启动/ }).click(),
  ]);
  await expect(page.locator('.ax-paper-stats')).toContainText(/成交笔数\s*1/, { timeout: 20_000 });
  await assertPractice('paper');
  await Promise.all([
    page.waitForResponse(response => response.url().includes('/api/paper/stop') && response.ok()),
    page.getByRole('button', { name: /停止/ }).click(),
  ]);
});


test('timeframe practice uses one completed real series and visualizes both horizons', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('同一时间尺度');
  const card = page.locator('.ax-kb-card').filter({ hasText: '指标必须使用同一时间尺度' });
  await expect(card).toHaveCount(1, { timeout: 30_000 });
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  const panel = page.getByLabel('概念实践');
  await expect(panel).toContainText('不同时间尺度可以同时成立');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);

  const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await page.getByRole('button', { name: '运行实践' }).click();
  const [request, response] = await Promise.all([requestPromise, responsePromise]);
  const body = request.postDataJSON() as { concept_id: string; source: string; bars?: unknown; inputs: unknown };
  expect(body).toMatchObject({ concept_id: 'book_pitfall_timeframe', source: 'binance', inputs: {} });
  expect(body.bars).toBeUndefined();
  expect(response.ok(), await response.text()).toBe(true);
  const payload = await response.json();
  expect(payload.context).toBe('selected_dataset');
  expect(payload.provenance).toBe('provided_market_bars');
  expect(payload.bar_origin).toBe('server_fetched_completed_source_bars');
  expect(payload.values.source_bar_seconds).toBe(3600);
  expect(payload.values.same_completed_asof).toBe(1);
  expect(payload.values.short_horizon_bars).toBe(5);
  expect(payload.values.long_horizon_bars).toBe(20);
  expect(Number.isFinite(payload.values.short_horizon_return)).toBe(true);
  expect(Number.isFinite(payload.values.long_horizon_return)).toBe(true);
  expect(payload.bars.length).toBeGreaterThanOrEqual(21);
  expect(payload.bars.every((bar: { timestamp: string }) => new Date(bar.timestamp).getTime() <= Date.now())).toBe(true);
  const series = Object.fromEntries(payload.series.map((item: { name: string; values: Array<number | null> }) => [item.name, item.values]));
  expect(series.close_price).toHaveLength(payload.bars.length);
  expect(series.short_horizon_start.filter((value: number | null) => value != null)).toHaveLength(1);
  expect(series.long_horizon_start.filter((value: number | null) => value != null)).toHaveLength(1);
  await expect(panel).toContainText('短期窗口收盘价变化');
  await expect(panel).toContainText('同一根已收盘 K 线为截止');
  await expect(panel.locator('.ax-series-illustration svg')).toBeVisible();
});
