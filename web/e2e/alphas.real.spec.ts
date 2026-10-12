import { expect, test } from './v8-coverage';

test('verified Alpha exercises run against actual stock bars and expose provenance', async ({ page }) => {
  test.setTimeout(180_000);
  const catalogPromise = page.waitForResponse(response => response.url().includes('/api/alphas/catalog'));
  await page.goto('/learn/alphas');
  const catalogResponse = await catalogPromise;
  expect(catalogResponse.ok(), await catalogResponse.text()).toBe(true);
  const catalog = await catalogResponse.json();
  expect(catalog.collections.find((item: { id: string }) => item.id === 'worldquant101').factors)
    .toEqual(expect.arrayContaining([expect.objectContaining({ id: 'alpha101', status: 'available' })]));

  for (const exercise of [
    { collection: 'worldquant101', factor: 'alpha101', factorName: 'Alpha 101', source: 'us_stock', symbol: 'AAPL', name: 'WorldQuant Alpha101' },
    { collection: 'qlib_alpha158', factor: 'kmid', factorName: 'KMID', source: 'a_share', symbol: '600519', name: 'Microsoft Qlib Alpha158' },
  ]) {
    await page.getByRole('button', { name: '选择因子库' }).click();
    await page.getByRole('listbox', { name: '选择因子库' }).getByRole('option', { name: exercise.name, exact: true }).click();
    await page.getByRole('button', { name: '选择因子', exact: true }).click();
    await page.getByRole('listbox', { name: '选择因子' }).getByRole('option', { name: exercise.factorName, exact: true }).click();
    await expect(page.getByLabel('研究标的')).toHaveValue(exercise.symbol);
    await page.getByLabel('日线根数').fill('80');
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/alphas/research') && response.request().method() === 'POST');
    await page.getByRole('button', { name: '运行实证' }).click();
    const response = await responsePromise;
    expect(response.ok(), `${exercise.name}: ${await response.text()}`).toBe(true);
    const payload = await response.json();
    expect(payload).toMatchObject({ collection: exercise.collection, source: exercise.source, symbol: exercise.symbol, status: 'computed', factor: { id: exercise.factor, status: 'available' } });
    expect(payload.bars.length).toBeGreaterThanOrEqual(30);
    expect(payload.series.length).toBe(payload.bars.length);
    expect(payload.forward_returns.length).toBe(payload.bars.length);
    expect(payload.stats.paired_count).toBeGreaterThan(0);
    expect(payload.market_provenance.provider).toBeTruthy();
    await expect(page.getByLabel('因子研究结果')).toBeVisible();
    await expect(page.getByRole('img', { name: '因子时间序列' })).toBeVisible();
  }
});
