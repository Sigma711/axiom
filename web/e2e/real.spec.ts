import { expect, test } from '@playwright/test';

test('real Rust service supports the four-module learning journey', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  const selectSynthetic = async () => {
    await page.getByRole('button', { name: '数据源', exact: true }).click();
    await page.getByText('合成 (随机)', { exact: true }).click();
  };
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '学习中心' })).toBeVisible();
  await page.getByRole('button', { name: '在数据探索中实践' }).first().click();
  await expect(page.getByRole('heading', { name: '数据探索' })).toBeVisible();
  await expect(page.getByLabel('概念实践')).toBeVisible();
  await page.getByRole('button', { name: '数据源', exact: true }).click();
  await page.getByText('合成 (随机)', { exact: true }).click();
  await page.getByRole('button', { name: '加载数据' }).click();
  await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
  await page.getByRole('button', {name:'运行实践'}).click();
  await expect(page.locator('.ax-practice-result')).toContainText('已计算');
  await page.getByRole('button', { name: '回测', exact: true }).click();
  await expect(page.getByRole('button', { name: '运行回测' })).toBeEnabled();
  await page.getByRole('button', { name: '数据源' }).click();
  await page.getByText('合成 (随机)', { exact: true }).click();
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
  await Promise.all([page.waitForResponse(response => response.url().includes('/api/paper/strategy') && response.ok()), page.getByRole('button', { name: /启动/ }).click()]);
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

test('every live knowledge concept opens a meaningful SVG illustration', async ({ page }) => {
  test.setTimeout(180_000);
  await page.goto('/');
  const cards = page.locator('.ax-kb-card');
  await expect.poll(() => cards.count(), { timeout: 30_000 }).toBeGreaterThan(0);
  const total = await cards.count();
  expect(total).toBeGreaterThan(0);
  for (let index = 0; index < total; index += 1) {
    await cards.nth(index).locator('.ax-kb-details').click();
    await expect(cards.nth(index).locator('.ax-knowledge-chart svg')).toBeVisible({ timeout: 30_000 });
    if (process.env.CI) await expect(cards.nth(index).locator('.ax-gh-btn')).toHaveAttribute('href', /^https:\/\/github\.com\/Sigma711\/axiom\/blob\/[0-9a-f]{40}\/src\/.+#L[1-9]\d*-L[1-9]\d*$/);
  }
  await expect(page.locator('.ax-knowledge-chart svg')).toHaveCount(total);
});
