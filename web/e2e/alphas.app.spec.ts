import { expect, test } from './v8-coverage';

const qlibIds = [
  'kmid', 'klen', 'kmid2', 'kup', 'kup2', 'klow', 'klow2', 'ksft', 'ksft2',
  'open0', 'high0', 'low0', 'vwap0',
  ...['roc', 'ma', 'std', 'beta', 'rsqr', 'resi', 'max', 'min', 'qtlu', 'qtld', 'rank', 'rsv', 'imax', 'imin', 'imxd', 'corr', 'cord', 'cntp', 'cntn', 'cntd', 'sump', 'sumn', 'sumd', 'vma', 'vstd', 'wvma', 'vsump', 'vsumn', 'vsumd'].flatMap(family => [5, 10, 20, 30, 60].map(window => `${family}${window}`)),
];
const qlibFactors = qlibIds.map(id => ({ id, name: id.toUpperCase(), status: id === 'vwap0' ? 'unavailable' : 'available', required_fields: id.startsWith('k') ? ['open', 'high', 'low', 'close'] : ['close'], summary: id === 'vwap0' ? '需要真实 VWAP，当前不可运行。' : `${id.toUpperCase()} 的逐项学习说明。`, formula: `${id.toUpperCase()} verified formula`, source_url: 'https://qlib.readthedocs.io/en/latest/component/data.html', code_url: id === 'vwap0' ? null : 'https://github.test/qlib' }));

const catalog = { collections: [
  { id: 'worldquant101', name: 'WorldQuant Alpha101', source_url: 'https://arxiv.org/pdf/1601.00991', total: 101, factors: [
    { id: 'alpha1', name: 'Alpha 1', status: 'unavailable', required_fields: [], summary: '尚未进入可运行练习。', source_url: 'https://arxiv.org/pdf/1601.00991', code_url: null },
    { id: 'alpha101', name: 'Alpha 101', status: 'available', required_fields: ['open', 'high', 'low', 'close'], summary: '比较开收盘变化与日内振幅。', source_url: 'https://arxiv.org/pdf/1601.00991', code_url: 'https://github.com/Sigma711/axiom/blob/test/src/alpha_research.rs#L1' },
  ] },
  { id: 'qlib_alpha158', name: 'Microsoft Qlib Alpha158', source_url: 'https://qlib.readthedocs.io/en/latest/component/data.html', total: 158, factors: qlibFactors },
] };

test('Alpha research route runs a verified exercise with mocked real-data provenance', async ({ page }) => {
  let requestBody: unknown;
  await page.route('**/api/code_loc**', route => route.fulfill({ json: { path: 'src/example.rs', line: 1, url: 'https://github.test/source' } }));
  await page.route('**/api/alphas/catalog', route => route.fulfill({ json: catalog }));
  await page.route('**/api/alphas/research', route => {
    requestBody = route.request().postDataJSON();
    return route.fulfill({ json: {
      collection: 'qlib_alpha158', source: 'a_share', symbol: '600519', factor: qlibFactors[0], status: 'computed',
      bars: [], series: [{ timestamp: '2026-01-01T00:00:00Z', value: .2 }, { timestamp: '2026-01-02T00:00:00Z', value: null }, { timestamp: '2026-01-03T00:00:00Z', value: -.1 }],
      forward_returns: [{ timestamp: '2026-01-01T00:00:00Z', value: .01 }, { timestamp: '2026-01-02T00:00:00Z', value: -.02 }, { timestamp: '2026-01-03T00:00:00Z', value: null }],
      stats: { paired_count: 2, time_series_correlation: -.25 }, notes: ['仅是单标的时间序列诊断。'],
      market_provenance: { provider: 'yahoo', endpoint: 'https://query.example/AAPL', price_basis: 'provider_adjustment_unverified', corporate_actions: 'not_simulated' },
    } });
  });

  await page.goto('/learn/path');
  await page.getByRole('button', { name: '进入 Alpha 研究' }).click();
  await expect(page).toHaveURL(/\/learn\/alphas$/);
  await expect(page.getByRole('button', { name: 'Alpha 研究', exact: true })).toHaveClass(/active/);
  await page.getByLabel('选择因子库').selectOption('qlib_alpha158');
  await expect(page.locator('[data-factor-card]')).toHaveCount(158);
  await expect(page.locator('[data-factor-card]').first()).toHaveCSS('content-visibility', 'visible');
  await page.getByLabel('搜索因子').fill('ROC5');
  await expect(page.locator('[data-factor-card]')).toHaveCount(1);
  await page.locator('[data-factor-card]').click();
  await expect(page.getByLabel('ROC5 因子示意图')).toBeVisible();
  const factor = page.getByLabel('选择因子', { exact: true });
  await expect(factor.locator('option')).toHaveCount(157);
  await factor.selectOption('kmid');
  await page.getByRole('button', { name: '运行实证' }).click();

  await expect(page.getByLabel('因子研究结果')).toBeVisible();
  await expect(page.getByText('-0.2500', { exact: true })).toBeVisible();
  await expect(page.getByRole('img', { name: '因子时间序列' }).locator('path.ax-alpha-line')).toHaveCount(2);
  expect(requestBody).toEqual({ collection: 'qlib_alpha158', id: 'kmid', source: 'a_share', symbol: '600519', limit: 160 });

  await page.screenshot({ path: test.info().outputPath('alpha-research-dark-desktop.png'), fullPage: true });
  await page.getByLabel('切换到浅色模式').click();
  await page.screenshot({ path: test.info().outputPath('alpha-research-light-desktop.png'), fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  const studio = page.locator('.ax-alpha');
  expect(await studio.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
  await expect(page.getByRole('img', { name: '前瞻收益时间序列' })).toBeVisible();
  await expect(page.getByRole('img', { name: '因子与前瞻收益散点图' })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('alpha-research-light-mobile.png'), fullPage: true });
});
