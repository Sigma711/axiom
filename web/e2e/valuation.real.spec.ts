import { expect, test } from './v8-coverage';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

test.beforeAll(async ({ request }) => {
  test.setTimeout(90_000); // Two independent provider probes each have a 40s bound.
  // This case deliberately uses a reviewed post-revision snapshot, not a
  // point-in-time backtest. Provider reachability is recorded separately.
  const url = 'https://fred.stlouisfed.org/graph/fredgraph.csv?id=CPIAUCSL&cosd=2016-09-01&coed=2025-09-01';
  const reviewed = await readFile(resolve(process.cwd(), '../data/verified-sources/fred-cpi-2026-10-03.csv'));
  expect(reviewed.byteLength).toBe(2097);
  expect(createHash('sha256').update(reviewed).digest('hex')).toBe('d4f940d3358dd45bb74e61cf0a4cfe06194b35050d34a6a122f23f86304577e3');
  const canonical = (bytes: Buffer) => {
    const lines = bytes.toString('utf8').trim().split(/\r?\n/);
    expect(lines.shift()).toBe('observation_date,CPIAUCSL');
    const rows = new Map(lines.map(line => line.split(',') as [string, string]));
    return Array.from({ length: 10 }, (_, i) => {
      const date = `${2016 + i}-09-01`;
      expect(rows.has(date)).toBe(true);
      return `CPIAUCSL|${date}|${Number(rows.get(date)).toFixed(3)}\n`;
    }).join('');
  };
  const verify = (bytes: Buffer) => {
    const records = canonical(bytes);
    expect(Buffer.byteLength(records)).toBe(280);
    expect(createHash('sha256').update(records).digest('hex')).toBe('538d5fb7aae05cda7831de199af10f3e8ab8d45c9f59f4fb89473724865cb7ce');
  };
  verify(reviewed);
  let bytes: Buffer = reviewed;
  try {
    const response = await request.get(url, { timeout: 40_000 });
    expect(response.ok()).toBe(true);
    bytes = await response.body();
    verify(bytes);
  } catch (error) {
    bytes = reviewed;
    test.info().annotations.push({ type: 'reviewed-snapshot-fallback', description: `FRED retrospective CPI case: ${String(error)}` });
  }
  const cache = resolve(process.cwd(), '../target/e2e-data/valuation-sources');
  await mkdir(cache, { recursive: true });
  await writeFile(resolve(cache, 'fred_cpi.source'), bytes);

  const nuveenUrl = 'https://documents.nuveen.com/Documents/Nuveen/Viewer.aspx?download=1&uniqueId=0779f60a-86ee-4128-87a3-1db9363e171e';
  const nuveenReviewed = await readFile(resolve(process.cwd(), '../data/verified-sources/nuveen-proxy-2025.pdf'));
  const verifyNuveen = (document: Buffer) => {
    expect(document.subarray(0, 5).toString('ascii')).toBe('%PDF-');
    expect(document.byteLength).toBe(1_438_050);
    expect(createHash('sha256').update(document).digest('hex')).toBe('328251373d35c20d0450538dad87c1bf28ca6747393dbc7ddb72d57bb1ccfb19');
  };
  verifyNuveen(nuveenReviewed);
  try {
    const response = await request.get(nuveenUrl, { timeout: 40_000 });
    expect(response.ok()).toBe(true);
    verifyNuveen(await response.body());
  } catch (error) {
    test.info().annotations.push({ type: 'reviewed-original-fallback', description: `Nuveen official endpoint did not return the reviewed PDF: ${String(error)}` });
  }
});

// Worked independently from the reviewed issuer statements, not API recordings.
const cases: Array<[string, string, number]> = [
  ['altman_z', 'altman_z', 10.61901749051463],
  ['beneish_m', 'beneish_m', -2.294943021712222],
  ['book_dcf', 'dcf_value_per_share', 125.12238523309892],
  ['book_dividend_yield', 'dividend_yield', 1.02 / 269.05],
  ['book_earnings_yield', 'earnings_yield', 7.46 / 269.05],
  ['book_ebitda_margin', 'ebitda_margin', 144748 / 416161],
  ['book_ev', 'enterprise_value', 4037468.603],
  ['book_ev_ebit', 'ev_ebit', 4037468.603 / 133050],
  ['book_ev_sales', 'ev_sales', 4037468.603 / 416161],
  ['book_fcf_yield', 'fcf_yield', 98767 / 3974745.603],
  ['book_intangibles_ratio', 'intangibles_ratio', 4388 / 5158],
  ['book_interest_coverage', 'interest_coverage', 2318 / 259],
  ['book_nav_discount', 'nav_premium_discount', (14.91 - 16.30) / 16.30],
  ['book_net_debt_ebitda', 'net_debt_ebitda', 62723 / 144748],
  ['book_payout_ratio', 'payout_ratio', 15421 / 112010],
  ['book_price_cashflow', 'price_operating_cash_flow', 3974745.603 / 111482],
  ['book_ptbv', 'price_tangible_book', 87.58 * 471 / 770],
  ['book_qoq', 'qoq_growth', 102466 / 94036 - 1],
  ['ev_ebitda', 'ev_ebitda', 4037468.603 / 144748],
  ['goodwill_ratio', 'goodwill_intangibles_to_equity', 4388 / 5158],
  ['industry_pe_compare', 'apple_pe', 277.55 / 7.46],
  ['pb', 'pb', 3974745.603 / 73733],
  ['pe', 'pe', 269.05 / 7.46],
  ['peg', 'peg', (269.05 / 7.46) / ((7.46 / 6.08 - 1) * 100)],
  ['piotroski', 'piotroski_f_score', 7],
  ['ps', 'ps', 3974745.603 / 416161],
  ['roic', 'roic', 0.8314270092598283],
  ['book_cape', 'cape', 53.214146004905224],
];

test('valuation practices open from knowledge cards and preserve source identity, calculations and theme readability', async ({ page }) => {
  test.setTimeout(900_000);
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  for (const [id, key, expected] of cases) {
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto('/learn');
    await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill(id);
    const card = page.locator(`.ax-kb-card[data-concept-id="${id}"]`);
    await expect(card).toHaveCount(1);
    await card.getByRole('button', { name: '在数据探索中实践' }).click();
    await expect(page).toHaveURL(new RegExp(`/data\\?concept=${id}&source=us_stock$`));
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const received = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST', { timeout: 240_000 });
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await received;
    expect(response.ok(), `${id}: ${await response.text()}`).toBe(true);
    const result = await response.json();
    expect(result.values[key]).toBeCloseTo(expected, 8);
    expect(result.industry_facts.calculation.result_key).toBe(key);
    expect(result).toMatchObject({ concept_id: id, source: 'issuer_disclosure', status: 'computed', inputs: {}, bars: [] });
    const visual = panel.locator('.ax-industry-case');
    await expect(visual).toBeVisible();
    await expect(visual).not.toContainText('无法计算');
    await expect(visual).not.toContainText('PDF 第 0 页');
    await expect(visual.locator('[data-industry-fact]')).toHaveCount(result.industry_facts.reported_facts.length);
    expect(result.industry_facts.calculation.formula.trim()).not.toBe('');
    await expect(visual).toContainText(result.industry_facts.calculation.formula);
    for (const fact of result.industry_facts.reported_facts) {
      const row = visual.locator(`[data-industry-fact="${fact.key}"]`);
      await expect(row).toContainText(fact.label);
      if (fact.kind === 'assumption') {
        await expect(row).toContainText('模型假设');
        await expect(row.locator('a')).toHaveCount(0);
      } else {
        const url = `${fact.source_url}${fact.pdf_page > 0 ? `#page=${fact.pdf_page}` : ''}`;
        await expect(row.getByRole('link')).toHaveAttribute('href', url);
      }
    }
    if (['book_intangibles_ratio', 'book_interest_coverage', 'book_ptbv', 'goodwill_ratio'].includes(id)) {
      expect(result.industry_case.issuer.ticker).toBe('EBAY');
      await expect(visual.locator('h4')).toContainText('eBay');
      await expect(visual.locator('h4')).not.toContainText('Apple');
    }
    if (id === 'industry_pe_compare') expect(result.values.hp_pe).toBeCloseTo(23.98 / 2.65, 10);
    if (id === 'book_nav_discount') {
      const source = result.industry_case.sources.find((item: {id: string}) => item.id === 'nuveen_proxy_2025');
      expect(source.url).toBe('https://documents.nuveen.com/Documents/Nuveen/Viewer.aspx?download=1&uniqueId=0779f60a-86ee-4128-87a3-1db9363e171e');
      expect(source.verification.matched_bytes).toBe(1_438_050);
      expect(source.verification.matched_sha256).toBe('328251373d35c20d0450538dad87c1bf28ca6747393dbc7ddb72d57bb1ccfb19');
      if (source.verification.status === 'verified_archived_original') {
        await expect(visual).toContainText('已核验原文备份');
        await expect(visual).toContainText('Nuveen 原站请求未返回已核验 PDF');
      }
    }
    if (id === 'book_dcf') {
      await expect(visual).toContainText('零净举债');
      expect(result.industry_facts.reported_facts.filter((fact: {kind: string}) => fact.kind === 'assumption')).toHaveLength(5);
    }
    if (result.industry_facts.derived_metrics.length > 1) {
      await visual.locator('.ax-industry-derived summary').click();
      await expect(visual.locator('[data-derived-metric]')).toHaveCount(Object.keys(result.values).length - 1);
    }
    if (id === 'beneish_m') {
      await expect(visual.locator('[data-derived-metric="gmi"]')).toContainText('毛利');
      await expect(visual.locator('[data-derived-metric]')).toHaveCount(8);
    }
    if (id === 'piotroski') {
      await expect(visual.locator('[data-derived-metric]')).toHaveCount(9);
      await expect(visual.locator('[data-derived-metric="no_common_share_issuance"]')).toContainText('未满足（0 分）');
    }
    const colors: string[] = [];
    for (const theme of ['dark', 'light']) {
      await page.evaluate(value => { localStorage.setItem('axiom-theme', value); }, theme);
      const toggle = page.getByLabel(theme === 'dark' ? '切换到深色模式' : '切换到浅色模式');
      if (await toggle.count()) await toggle.click();
      expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
      colors.push(await visual.evaluate(node => getComputedStyle(node).backgroundColor));
      // A tall-element capture scrolls the page while stitching. Keep the
      // unrelated sticky navigation out of that artifact after interaction checks.
      const captureStyle = await page.addStyleTag({ content: '.ax-tabs { visibility: hidden !important; }' });
      await visual.screenshot({ path: test.info().outputPath(`${id}-${theme}.png`), animations: 'disabled' });
      await captureStyle.evaluate(node => node.parentNode?.removeChild(node));
    }
    expect(colors[0]).not.toBe(colors[1]);
    await page.setViewportSize({ width: 390, height: 844 });
    expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
    expect(await visual.evaluate(node => node.getBoundingClientRect().right <= window.innerWidth)).toBe(true);
    const captureStyle = await page.addStyleTag({ content: '.ax-tabs { visibility: hidden !important; }' });
    await visual.screenshot({ path: test.info().outputPath(`${id}-mobile.png`), animations: 'disabled' });
    await captureStyle.evaluate(node => node.parentNode?.removeChild(node));
  }
  expect(errors).toEqual([]);
});
