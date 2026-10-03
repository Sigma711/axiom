import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { expect, test } from './v8-coverage';
import type { Locator, Page } from '@playwright/test';

test('Apple split practice independently matches the issuer announcement and live provider event', async ({ page, request }) => {
  test.setTimeout(120_000);
  const issuerUrl = 'https://www.apple.com/newsroom/2020/07/apple-reports-third-quarter-results/';
  const issuerResponse = await request.get(issuerUrl, { timeout: 25_000 });
  expect(issuerResponse.ok()).toBe(true);
  const issuerText = await issuerResponse.text();
  expect(issuerText).toMatch(/four-for-one stock split/i);
  expect(issuerText).toMatch(/August 31, 2020/i);

  await page.goto('/learn');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('book_adjustment');
  const card = page.locator('.ax-kb-card[data-concept-id="book_adjustment"]');
  await expect(card).toHaveCount(1);
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=book_adjustment&source=us_stock$/);
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBe(true);
  const result = await response.json();
  expect(result).toMatchObject({ concept_id: 'book_adjustment', input_kind: 'stock_action_case', source: 'us_stock', symbol: 'AAPL', values: { new_shares_per_old_share: 4, split_only_price_multiplier: .25 } });
  const retrieval = result.adjustment_evidence.retrieval;
  expect(result.provenance).toBe(retrieval === 'restricted_server_relay'
    ? 'server_relayed_stock_corporate_action'
    : 'server_fetched_stock_corporate_action');
  expect(result.bar_origin).toBe(retrieval === 'restricted_server_relay'
    ? 'restricted_server_relay_us_stock_adjustment_evidence'
    : 'server_fetched_us_stock_adjustment_evidence');
  const sourceUrl = result.adjustment_evidence.endpoint as string;
  expect(new URL(sourceUrl).hostname).toMatch(/^query[12]\.finance\.yahoo\.com$/);
  let upstreamResponse = await request.get(sourceUrl, { headers: { 'user-agent': 'AXIOM educational market reader/1.0' }, timeout: 25_000 });
  let upstreamPayload: any;
  if (upstreamResponse.ok()) {
    upstreamPayload = await upstreamResponse.json();
  } else {
    const relayUrl = process.env.AXIOM_YAHOO_RELAY_URL || 'https://sigma711.top/axiom/api/provider/yahoo-chart';
    upstreamResponse = await request.get(relayUrl, {
      params: { symbol: 'AAPL', period1: '1595808000', period2: '1601510400' },
      timeout: 25_000,
    });
    expect(upstreamResponse.ok(), await upstreamResponse.text()).toBe(true);
    const envelope = await upstreamResponse.json();
    expect(envelope).toMatchObject({ provider: 'yahoo', retrieval: 'restricted_server_relay', source_url: sourceUrl });
    upstreamPayload = envelope.payload;
  }
  const upstream = upstreamPayload.chart.result[0];
  expect(upstream.meta.symbol).toBe('AAPL');
  expect(upstream.meta.currency).toBe('USD');
  expect(['EQUITY', 'ETF']).toContain(upstream.meta.instrumentType);
  const event = Object.values(upstream.events.splits).find((item: any) => item.date === 1598880600) as { date: number; numerator: number; denominator: number; splitRatio: string } | undefined;
  expect(event).toMatchObject({ numerator: 4, denominator: 1, splitRatio: '4:1' });
  expect(result.adjustment_evidence.event).toMatchObject({ effective_trading_date: '2020-08-31', numerator: event!.numerator, denominator: event!.denominator, split_ratio: event!.splitRatio });
  const first = result.adjustment_evidence.observations[0];
  const index = upstream.timestamp.indexOf(Date.parse(first.timestamp) / 1000);
  expect(index).toBeGreaterThanOrEqual(0);
  expect(first.close).toBeCloseTo(upstream.indicators.quote[0].close[index], 10);
  // Yahoo recalculates adjusted-close floats between otherwise identical live requests.
  expect(Math.abs(first.adjusted_close - upstream.indicators.adjclose[0].adjclose[index])).toBeLessThan(0.001);
  const visual = panel.getByRole('figure', { name: '苹果公司历史拆股示意' });
  await expect(visual).toContainText('一股变四股');
  await expect(visual).toContainText('不是收益');
  await visual.getByText('核对事件与行情来源').click();
  await expect(visual.getByRole('link', { name: 'Apple 官方拆股公告 ↗' })).toHaveAttribute('href', issuerUrl);
  await expect(visual.getByRole('link', { name: '查看行情接口 ↗' })).toHaveAttribute('href', sourceUrl);
  await visual.screenshot({ path: test.info().outputPath('stock-adjustment-real-dark.png') });
  await page.getByLabel('切换到浅色模式').click();
  await expect(visual).toBeVisible();
  await visual.screenshot({ path: test.info().outputPath('stock-adjustment-real-light.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
  await visual.screenshot({ path: test.info().outputPath('stock-adjustment-real-mobile.png') });
});

test('unadjusted-price pitfall opens the same source-backed Apple split lesson', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/learn');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('book_pitfall_adjustment');
  const card = page.locator('.ax-kb-card[data-concept-id="book_pitfall_adjustment"]');
  await expect(card).toHaveCount(1);
  await expect(card).not.toContainText('可编辑教学输入');
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=book_pitfall_adjustment&source=us_stock$/);
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBe(true);
  const result = await response.json();
  expect(result).toMatchObject({ concept_id: 'book_pitfall_adjustment', source: 'us_stock', symbol: 'AAPL', values: { new_shares_per_old_share: 4, split_only_price_multiplier: .25 } });
  expect(result.provenance).toBe(result.adjustment_evidence.retrieval === 'restricted_server_relay'
    ? 'server_relayed_stock_corporate_action'
    : 'server_fetched_stock_corporate_action');
  await expect(panel.getByRole('figure', { name: '苹果公司历史拆股示意' })).toContainText('不是收益');
  await expect(panel).toContainText('供应商');
});

async function assertExecutionDisclosure(page: Page, payload: any) {
  const disclosure = page.getByLabel('成交与价格口径', { exact: true });
  await expect(disclosure).toBeVisible();
  if (!await disclosure.evaluate(node => (node as HTMLDetailsElement).open)) await disclosure.locator('summary').click();
  for (const rule of payload.execution_assumptions || []) {
    const item = disclosure.locator('li').filter({ hasText: rule.description_zh });
    await expect(item).toContainText(rule.simulated ? '已模拟' : '未模拟');
    if (rule.limitation_zh) await expect(item).toContainText(rule.limitation_zh);
    if (rule.source_url) await expect(item.getByRole('link')).toHaveAttribute('href', rule.source_url);
  }
  if (payload.market_provenance) {
    if (payload.market_provenance.corporate_actions === 'not_applicable') {
      await expect(disclosure).toContainText('现货市场不适用股票拆股');
    } else {
      await expect(disclosure).toContainText('不代表含分红再投资的总回报');
    }
    if (payload.market_provenance.endpoint.startsWith('https://')) {
      await expect(disclosure.getByRole('link', { name: '行情接口来源 ↗' })).toHaveAttribute('href', payload.market_provenance.endpoint);
    }
  }
}

async function captureHistoricalCard(page: Page, visual: Locator, name: string) {
  await page.evaluate(() => document.fonts.ready);
  await visual.scrollIntoViewIfNeeded();
  await expect(visual).toBeVisible();
  const original = await visual.evaluate(node => {
    const rect = node.getBoundingClientRect();
    return { width: rect.width, height: rect.height, position: getComputedStyle(node).position, scrollY: window.scrollY };
  });
  expect(original.position).not.toBe('fixed');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
  await visual.locator('.ax-filing-fact a').first().click({ trial: true });
  // Verify natural layout first, then keep the same live card and dimensions
  // while isolating its screenshot origin from unrelated chart scrolling.
  const snapshotStyle = await page.addStyleTag({ content: `.ax-tabs { visibility: hidden !important; } .ax-filing-case { position: fixed !important; top: 0 !important; left: 0 !important; width: ${original.width}px !important; box-sizing: border-box; margin: 0 !important; transform: none !important; z-index: 10000; }` });
  try {
    const isolated = await visual.boundingBox();
    expect(isolated).not.toBeNull();
    expect(isolated!.x).toBe(0);
    expect(isolated!.y).toBe(0);
    expect(isolated!.width).toBeCloseTo(original.width, 1);
    expect(isolated!.height).toBeCloseTo(original.height, 1);
    await page.mouse.move(0, 0);
    await expect(visual).toHaveScreenshot(name, { maxDiffPixelRatio: .01 });
  } finally {
    await snapshotStyle.evaluate(node => node.parentNode?.removeChild(node));
    await page.evaluate(scrollY => window.scrollTo(0, scrollY), original.scrollY);
  }
  expect(await visual.evaluate(node => getComputedStyle(node).position)).toBe(original.position);
}

test('every historical industry case follows its verified original disclosure from the knowledge card to practice', async ({ page, request }) => {
  test.setTimeout(900_000);
  const sources = [
    { symbol: '2318.HK', url: 'https://pagroup.pingan.com/resource/pingan/IR-Docs/2025/pingan-ar24-report.pdf', bytes: 14_886_158, hash: '62a5bd793ef9a787cc95750d65e52803aa58fa424b01d94e754ea0120d5be8a3' },
    { symbol: 'SHOP', url: 'https://s27.q4cdn.com/572064924/files/doc_financials/2024/q4/Q4-2024-Press-Release-Final.pdf', bytes: 86_468, hash: '4bf71232697a2270b2dbc38fc9609c11c27d545d6f4301fce3356fa60c6ef6de' },
    { symbol: 'O', url: 'https://www.realtyincome.com/sites/realty-income/files/2025-02/realty-income-q4-2024-supplemental-information.pdf', bytes: 17_522_920, hash: 'a0b3bf067c7b19ebde01ceaac3ecb172ed6a4c7084eeabe276ad1d4599c62a3f' },
    { symbol: 'EBAY', url: 'https://ebay.q4cdn.com/610426115/files/doc_financials/2024/q4/eBay-10-K-2024.pdf', bytes: 1_004_020, hash: '10530b8314c4dc49f9737b938f28ead7a70212885c35919fb361d145401f37fb' },
    { symbol: 'DAL', url: 'https://s2.q4cdn.com/181345880/files/doc_financials/2024/q4/DAL-12-31-2024-10K-2-11-25-Filed.pdf', bytes: 897_733, hash: '61116b7fe79dd0c687d88c04ac376e4d09a6c3760163bbe9433f572bb2549afa' },
    { symbol: 'COST', url: 'https://s201.q4cdn.com/287523651/files/doc_news/Costco-Wholesale-Corporation-Reports-Fourth-Quarter-and-Fiscal-Year-2024-Operating-Results-2024.pdf', bytes: 138_152, hash: '590d2dc15e168ca52697a8d8b85b0f388cbea3eab8235b2ec8aa77ed4f173c57' },
    { symbol: 'MRNA', url: 'https://s29.q4cdn.com/435878511/files/doc_financials/2024/ar/MRNA010_AR_WEB_FULL.pdf', bytes: 3_098_566, hash: '2347835006ac22d5cd9b74683568893431149071740e81ff17603504ff70c1a5' },
    { symbol: 'PBR', url: 'https://transparencia.petrobras.com.br/documents/1357439/14971831/Relat%C3%B3rio%2Bde%2BGest%C3%A3o%2B-%2B2024.pdf/50685b26-3e9e-2ece-3035-33eebe338c73?download=true&t=1748554910000&version=1.0', bytes: 6_302_154, hash: '04372d526d67247b9ad66098a58d85ca2bf00b534478575ead5f650a7a463122' },
    { symbol: 'GOLD', url: 'https://www.barrick.com/files/doc_financial/annual_reports/2024/Barrick_Annual_Report_2024.pdf', bytes: 11_789_238, hash: '3cb6cf59458e8799650d1c219222f8c01e41b1fbda9351523ed6b602f3875b86' },
    { symbol: 'SIE.DE', url: 'https://assets.new.siemens.com/siemens/assets/api/uuid:344347ec-a1bd-44cb-aaaa-711d1b3ec1b8/Siemens-Annual-Report-2024.pdf', bytes: 4_671_939, hash: '75f568180a8d35287f970a4812817dcd2b5c690ec937bf80f17b6fe68f42521e' },
    { symbol: 'SPOT', url: 'https://investors.spotify.com/files/doc_financials/2020/q3/Shareholder-Letter-Q3-2020_FINAL.pdf', bytes: 1_172_171, hash: '82025cc49cce680c62ba9e5576881e6e84c867ba77f44a4f46d82f6c9ae81518' },
    { symbol: 'META', url: 'https://investor.fb.com/files/doc_earnings/2023/q3/presentation/Earnings-Presentation-Q3-2023.pdf', bytes: 172_720, hash: 'dfcaa1c855d2da261f0d392c4a603fddf8897934dc60b3397f272698bf071af4' },
    { symbol: 'SMWB', url: 'https://d1io3yog0oux5.cloudfront.net/_8f428cad86e9f7d21dc312829a41f817/similarweb/db/2008/19607/presentation/SMWB_Q3_2024_Investor_Presentation_.pdf', bytes: 9_680_955, hash: '3278ddfd096ebcc828579466b3e0af46b01f6fe38f72554f1d1304b0bfb53bf0' },
    { symbol: 'ZM', snapshot: 'zoom-q4-fy2024.pdf', url: 'https://investors.zoom.us/static-files/70629942-ff77-4bed-91d6-422766c47e6b', bytes: 118_862, hash: '79f0e6b5126a47869f196d07aa3ab3626c4a61cdbb46e17a79762ab264fbeaf4' },
    { symbol: 'SNOW', url: 'https://investors.snowflake.com/files/doc_financials/2024/q4/Q4-FY2024-Investor-Presentation-vF.pdf', bytes: 5_257_197, hash: '8da8efb70b65fc2c8928a1d6ccef32e530d447d510fa9da033dcb916ccf80a37' },
    { symbol: '0981.HK', url: 'https://www1.hkexnews.hk/listedco/listconews/sehk/2025/0211/2025021100441.pdf', bytes: 444_831, hash: '18e7cc96cc2405587fbb06e5da078fe4ce129833e6257009fe1c650a0d070a76' },
    { symbol: 'FRO', url: 'https://www.frontlineplc.cy/wp-content/uploads/2024/09/Presentation-Q2-2024.pdf', bytes: 870_579, hash: '394b72e6c229f9586a18a2b1348b8262fc11459afa7c30147df2d5f1ff3677fa' },
    { symbol: 'VZ', url: 'https://www.verizon.com/about/sites/default/files/2024-04/FS_VZ_1Q24_042224.pdf', bytes: 103_314, hash: 'c22a0161f9268b2d9799cbfa1ea78da0e44d16d1cb502896e5a0ac212bae4817' },
  ];
  const industryCache = resolve(process.cwd(), '..', 'target', 'e2e-data', 'industry-sources');
  await mkdir(industryCache, { recursive: true });
  for (const source of sources) {
    const portableSnapshot = 'snapshot' in source
      ? await readFile(resolve(process.cwd(), 'e2e', 'fixtures', 'original-sources', source.snapshot!))
      : undefined;
    if (portableSnapshot) {
      expect(portableSnapshot.subarray(0, 5).toString()).toBe('%PDF-');
      expect(portableSnapshot).toHaveLength(source.bytes);
      expect(createHash('sha256').update(portableSnapshot).digest('hex')).toBe(source.hash);
    }
    let bytes: Buffer;
    try {
      const response = await request.get(source.url, { timeout: 60_000 });
      const live = await response.body();
      if (!response.ok() || createHash('sha256').update(live).digest('hex') !== source.hash) throw new Error(`HTTP ${response.status()} or bytes changed`);
      bytes = live;
    } catch (error) {
      if (!portableSnapshot) throw error;
      test.info().annotations.push({
        type: 'reviewed-snapshot-fallback',
        description: `${source.symbol}: provider unavailable or changed; using version-controlled original ${source.snapshot}: ${String(error)}`,
      });
      bytes = portableSnapshot;
    }
    expect(bytes.subarray(0, 5).toString()).toBe('%PDF-');
    expect(bytes).toHaveLength(source.bytes);
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(source.hash);
    // This seeds the application's immutable reviewed-document cache. The
    // annotation above remains explicit when the provider is not live-reachable.
    await writeFile(resolve(industryCache, `${source.hash}.pdf`), bytes);
  }
  // Independent literals transcribed from Ping An pp57–58/336, Shopify p1,
  // Realty Income p25 and eBay p44. Ratios use actual business denominators.
  const cases = [
    { id: 'bank_nim', symbol: '2318.HK', key: 'net_interest_margin', expected: 93427 / 4994494, facts: { net_interest_income: 93427, average_earning_assets: 4994494 }, page: 57, audited: false },
    { id: 'book_bank_nim', symbol: '2318.HK', key: 'net_interest_margin', expected: 93427 / 4994494, facts: { net_interest_income: 93427, average_earning_assets: 4994494 }, page: 57, audited: false },
    { id: 'book_bank_cost_income', symbol: '2318.HK', key: 'cost_income_ratio', expected: 40582 / 146695, facts: { operating_expenses: 40582, operating_income: 146695 }, page: 57, audited: false },
    { id: 'book_bank_npl_ratio', symbol: '2318.HK', key: 'nonperforming_loan_ratio', expected: 35738 / 3374103, facts: { nonperforming_loans: 35738, gross_loans: 3374103 }, page: 58, audited: false },
    { id: 'book_insurance_solvency_ratio', symbol: '2318.HK', key: 'solvency_adequacy_ratio', expected: 138649 / 67536, facts: { available_capital: 138649, required_capital: 67536 }, page: 336, audited: true },
    { id: 'book_saas_arr', symbol: 'SHOP', key: 'annualized_recurring_revenue_run_rate', expected: 2136, facts: { monthly_recurring_revenue: 178 }, page: 1, audited: false },
    { id: 'book_saas_rule_of_40', symbol: 'SHOP', key: 'rule_of_40', expected: (8880 / 7060 - 1) + 1597 / 8880, facts: { revenue_2024: 8880, revenue_2023: 7060, free_cash_flow: 1597 }, page: 1, audited: false },
    { id: 'book_platform_gmv', symbol: 'SHOP', key: 'gross_merchandise_value', expected: 292275, facts: { gross_merchandise_value: 292275 }, page: 1, audited: false },
    { id: 'book_platform_take_rate', symbol: 'EBAY', key: 'platform_take_rate', expected: 10283 / 74667, facts: { platform_revenue: 10283, gross_merchandise_value: 74667 }, page: 44, audited: false },
    { id: 'book_reit_occupancy', symbol: 'O', key: 'occupied_area_ratio', expected: 335777818 / 339361416, facts: { leased_area: 335777818, lettable_area: 339361416 }, page: 25, audited: false },
    { id: 'book_airline_casm', symbol: 'DAL', key: 'cost_per_available_seat_mile', expected: 55648 / 288394 * 100, facts: { total_operating_expense: 55648, available_seat_miles: 288394, reported_casm: 19.3 }, page: 40, audited: false },
    { id: 'book_airline_load_factor', symbol: 'DAL', key: 'passenger_load_factor', expected: 246145 / 288394, facts: { revenue_passenger_miles: 246145, available_seat_miles: 288394, reported_load_factor: .85 }, page: 42, audited: false },
    { id: 'book_airline_rasm', symbol: 'DAL', key: 'total_revenue_per_available_seat_mile', expected: 61643 / 288394 * 100, facts: { total_operating_revenue: 61643, available_seat_miles: 288394, reported_trasm: 21.37 }, page: 38, audited: false },
    { id: 'book_bank_cet1_ratio', symbol: '2318.HK', key: 'cet1_capital_adequacy_ratio', expected: .0912, facts: { core_tier_1_capital_adequacy_ratio: .0912 }, page: 4, audited: false },
    { id: 'book_bank_provision_coverage', symbol: '2318.HK', key: 'provision_coverage_ratio', expected: 2.5071, facts: { provision_coverage_ratio: 2.5071 }, page: 4, audited: false },
    { id: 'book_insurance_combined_ratio', symbol: '2318.HK', key: 'combined_ratio', expected: .983, facts: { property_casualty_combined_ratio: .983 }, page: 4, audited: false },
    { id: 'book_insurance_nbv', symbol: '2318.HK', key: 'new_business_value', expected: 28534, facts: { life_health_new_business_value: 28534 }, page: 4, audited: false },
    { id: 'book_reit_affo', symbol: 'O', key: 'affo_available_to_common_stockholders', expected: 3621437, facts: { affo_available_to_common_stockholders: 3621437 }, page: 6, audited: false },
    { id: 'book_reit_cap_rate', symbol: 'O', key: 'net_cash_capitalization_rate', expected: .072, facts: { disposition_net_cash_cap_rate: .072 }, page: 15, audited: false },
    { id: 'book_reit_ffo', symbol: 'O', key: 'ffo_available_to_common_stockholders', expected: 3467659, facts: { ffo_available_to_common_stockholders: 3467659 }, page: 5, audited: false },
    { id: 'book_retail_same_store_sales_growth', symbol: 'COST', key: 'same_store_sales_growth', expected: .053, facts: { total_company_comparable_sales_growth: .053 }, page: 1, audited: false },
    { id: 'book_biopharma_cash_runway', symbol: 'MRNA', key: 'cash_runway_months', expected: 9519 / 3004 * 12, facts: { cash_and_investments: 9519, annual_operating_cash_burn: 3004 }, page: 122, audited: true },
    { id: 'book_energy_lifting_cost', symbol: 'PBR', key: 'lifting_cost', expected: 6.05, facts: { reported_lifting_cost: 6.05 }, page: 78, audited: false },
    { id: 'book_energy_reserve_life', symbol: 'PBR', key: 'reserve_life', expected: 13.2, facts: { reported_reserve_life: 13.2 }, page: 79, audited: false },
    { id: 'book_gold_aisc', symbol: 'GOLD', key: 'gold_aisc', expected: 1350, facts: { reported_gold_aisc: 1350 }, page: 30, audited: false },
    { id: 'book_industrial_backlog', symbol: 'SIE.DE', key: 'order_backlog', expected: 113, facts: { order_backlog: 113 }, page: 15, audited: false },
    { id: 'book_industrial_book_to_bill', symbol: 'SIE.DE', key: 'book_to_bill', expected: 84056 / 75930, facts: { orders: 84056, revenue: 75930 }, page: 15, audited: false },
    { id: 'book_internet_arpu', symbol: 'SPOT', key: 'premium_arpu', expected: 4.19, facts: { premium_arpu: 4.19 }, page: 4, audited: false },
    { id: 'book_internet_dau_mau', symbol: 'META', key: 'daily_monthly_active_ratio', expected: 3.14 / 3.96, facts: { family_dap: 3.14, family_map: 3.96 }, page: 10, audited: false },
    { id: 'book_saas_cac_payback', symbol: 'SMWB', key: 'cac_payback_lower_bound', expected: 21, facts: { cac_payback_lower_bound: 21, cac_payback_upper_bound: 22 }, page: 22, audited: false },
    { id: 'book_saas_churn', symbol: 'ZM', key: 'monthly_customer_churn', expected: .032, facts: { online_monthly_churn: .032 }, page: 5, audited: false },
    { id: 'book_saas_nrr', symbol: 'SNOW', key: 'net_revenue_retention', expected: 1.31, facts: { net_revenue_retention: 1.31 }, page: 21, audited: false },
    { id: 'book_semiconductor_asp', symbol: '0981.HK', key: 'implied_revenue_per_equivalent_wafer', expected: 2207281 * .925 * 1000 / 1991761, facts: { revenue: 2207281, wafer_revenue_share: .925, wafer_shipments: 1991761 }, page: 5, audited: false },
    { id: 'book_semiconductor_utilization', symbol: '0981.HK', key: 'capacity_utilization', expected: .855, facts: { utilization_rate: .855 }, page: 5, audited: false },
    { id: 'book_shipping_tce', symbol: 'FRO', key: 'vlcc_spot_tce', expected: 49600, facts: { vlcc_spot_tce: 49600 }, page: 3, audited: false },
    { id: 'book_telecom_arpu', symbol: 'VZ', key: 'prepaid_arpu', expected: 31.17, facts: { prepaid_arpu: 31.17 }, page: 7, audited: false },
    { id: 'book_telecom_churn', symbol: 'VZ', key: 'prepaid_monthly_churn', expected: .0426, facts: { prepaid_churn: .0426 }, page: 6, audited: false },
  ];
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  for (const item of cases) {
    await page.goto('/learn');
    await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill(item.id);
    const card = page.locator(`.ax-kb-card[data-concept-id="${item.id}"]`);
    await expect(card).toHaveCount(1);
    await card.getByRole('button', { name: '在数据探索中实践' }).click();
    await expect(page).toHaveURL(new RegExp(`/data\\?concept=${item.id}&source=issuer_disclosure$`));
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const sent = page.waitForRequest(r => r.url().includes('/api/practice') && r.method() === 'POST');
    const received = page.waitForResponse(r => r.url().includes('/api/practice') && r.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [outbound, response] = await Promise.all([sent, received]);
    expect(outbound.postDataJSON()).toEqual({ concept_id: item.id, module: 'data', source: 'issuer_disclosure', symbol: item.symbol, inputs: {} });
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result).toMatchObject({ context: 'historical_industry_disclosure', bar_origin: 'server_verified_issuer_filing_pdf', input_kind: 'industry_case', status: 'computed', bars: [], inputs: {} });
    expect(result.values[item.key]).toBeCloseTo(item.expected, 12);
    const source = sources.find(source => source.symbol === item.symbol)!;
    expect(result.industry_case).toMatchObject({ url: source.url, sha256: source.hash, bytes: source.bytes, audited: item.audited, issuer: { ticker: item.symbol } });
    expect(result.industry_case.verification).toMatchObject({ status: 'verified_immutable_cache', matched_sha256: source.hash, matched_bytes: source.bytes });
    expect(Object.fromEntries(result.industry_facts.reported_facts.map((fact: { key: string; value: number }) => [fact.key, fact.value]))).toEqual(item.facts);
    expect(result.units[item.key]).toBeTruthy();
    expect(result.industry_facts.calculation.formula.trim()).not.toBe('');
    expect(Object.keys(result.industry_facts.calculation.symbol_mapping).length).toBeGreaterThan(0);
    expect(result.industry_facts.reported_facts.every((fact: { pdf_page: number; unit: string }) => fact.pdf_page > 0 && fact.unit.trim() !== '')).toBe(true);
    const visual = panel.locator('.ax-industry-case');
    await expect(visual).toBeVisible();
    await expect(visual).toContainText('固定历史案例');
    await expect(visual.locator('.ax-industry-formula')).toBeVisible();
    await expect(visual.locator('.ax-industry-unit-group')).not.toHaveCount(0);
    await expect(visual).not.toContainText('PDF 第 0 页');
    await expect(visual.locator('[data-industry-fact]')).toHaveCount(Object.keys(item.facts).length);
    await expect(visual.locator('[data-industry-fact]').first().getByRole('link')).toHaveAttribute('href', `${source.url}#page=${item.page}`);
    await expect(visual.locator(`a[href="${source.url}"]`).first()).toHaveAttribute('href', source.url);
  }
  await page.goto('/data?concept=book_reit_occupancy&source=issuer_disclosure');
  await page.getByLabel('概念实践').getByRole('button', { name: '运行实践' }).click();
  const visual = page.getByLabel('概念实践').locator('.ax-industry-case');
  await captureHistoricalCard(page, visual, 'industry-occupancy-dark.png');
  await page.getByLabel('切换到浅色模式').click();
  await captureHistoricalCard(page, visual, 'industry-occupancy-light.png');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await visual.evaluate(node => node.getBoundingClientRect().right <= window.innerWidth)).toBe(true);
  await captureHistoricalCard(page, visual, 'industry-occupancy-mobile.png');
  expect(errors).toEqual([]);
});

test('every ownership and analyst disclosure follows its fixed original source from catalog to visible calculation', async ({ page, request }) => {
  test.setTimeout(600_000);
  const documents = [
    { url: 'https://static.cninfo.com.cn/finalpage/2026-04-17/1225114741.PDF', bytes: 1_082_847, hash: '474905deeaf0f875fc0a1b097a626c0c7852c427faadc5d7fc7816cbf45ea288' },
    { url: 'https://cdn.cboe.com/resources/us/equities/market-statistics/short-interest/Bats_Listed_Short_Interest-finra-20261002.csv', bytes: 127_080, hash: '564c843210b40a596568deb62da27c10e5627b00754b4372197b7ea90942763e' },
    { url: 'https://mediaassets.airbus.com/pm_38_787_787398-089y4vnyo3.pdf', bytes: 81_415, hash: '6152126d3b4b46b3ffad7c68dc76a6d6ec40908f28b2af85bf514a2ba62e6e9b' },
    { url: 'https://www.afm.nl/downloadregisterfile.aspx?type=openbaarmaking-voorwetenschap&enc=7Rpj0BBaMD5lzfwUlyQ9TIe+XbKYS1JE7+GVfXY2PxNqAT3NVNKp3QCQcI2WUb5gS/PYmhvg5EA+E7FpgiengZj0NLAMUDbP/oPnUtKH27VA0/CZGu7E4hLGdYYwwWpdTyp/cuqK9u2A7er37RJifDgVQS3W6HmCKGWO1CEeZNXO0g0IROVIr+z+5qosB2fxjW2qofVbttWzylRmw6K7ZNM1O+nSGFVNazbTDT6ekO3vaajCoWPbTc7g3zIvR0ilTPg8vjdE1aEQ10Qyj60J7wrJWgG+ROEOcLEujFwVnZc4hQg9b/oBAl+w6cUoq0F56qLbopyLMcJnwQKx89Mczg==', bytes: 296_359, hash: '21f849df5646b6b57e006827581768016e3b0995ec8d95d096f573a7886f8610' },

    { url: 'https://downloads.research-hub.de/2025%2002%2024%20Airbus%20Update___kh66mlcm.pdf', bytes: 991_184, hash: '2819c2970703eaf90c22f57db151915bbd8a03dd259eb8478848a6fe25643471' },
    { url: 'https://www.signify.com/static/2025/20260114-signify-analyst-consensus-pre-q4-2025.pdf', bytes: 231_176, hash: 'c2ebe6999e54928dc1f2041a0194e6a856e900b184147ad3974ac51e68842bb3' },
  ];
  const ownershipCache = resolve(process.cwd(), '..', 'target', 'e2e-data', 'ownership-sources');
  await mkdir(ownershipCache, { recursive: true });
  for (const document of documents) {
    const response = await request.get(document.url, { timeout: 60_000 });
    expect(response.ok(), document.url).toBe(true);
    const bytes = await response.body();
    if (document.url.endsWith('.csv')) {
      const text = Buffer.from(bytes).toString('utf8');
      expect(text).toContain('Cycle Settlement Date,BATS-Symbol');
      expect(text).toContain(',ARKW,');
    } else {
      expect(bytes.subarray(0, 5).toString()).toBe('%PDF-');
    }
    expect(bytes).toHaveLength(document.bytes);
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(document.hash);
    await writeFile(resolve(ownershipCache, `${document.hash}.source`), bytes);
  }
  const holders = [681282935, 56996777, 55048844, 27849688, 11573000, 10397104, 10324650, 8039447, 7377868, 5629234];
  const cases = [
    { id: 'buyback_rate', symbol: '600519', key: 'buyback_rate', expected: (3927585 + 87059) / 1256197800, facts: { first_program_repurchased_shares: 3927585, second_program_repurchased_shares: 87059, opening_total_shares: 1256197800 } },
    { id: 'holder_concentration', symbol: '600519', key: 'top_holder_fraction', expected: holders.reduce((sum, value) => sum + value, 0) / 1252270215, facts: Object.fromEntries([...holders.map((value, index) => [`holder_${index + 1}_shares`, value]), ['closing_total_shares', 1252270215]]) },
    { id: 'insider_trading', symbol: '600519', key: 'insider_trading_rate', expected: 2071359 / (681282935 - 2071359), facts: { controlling_holder_acquired_shares: 2071359, controlling_holder_shares: 681282935 } },
    { id: 'institution_holding', symbol: '600519', key: 'institution_holding', expected: (11573000 + 10324650 + 7377868 + 5629234) / 1252270215, facts: { holder_5_shares: 11573000, holder_7_shares: 10324650, holder_9_shares: 7377868, holder_10_shares: 5629234, closing_total_shares: 1252270215 } },
    { id: 'restricted_shares', symbol: '600519', key: 'restricted_share_fraction', expected: 0, facts: { closing_total_shares: 1252270215, unrestricted_shares: 1252270215 } },
    { id: 'share_pledge', symbol: '600519', key: 'share_pledge', expected: 0, facts: { controlling_holder_pledged_shares: 0, controlling_holder_shares: 681282935 } },
    { id: 'short_interest', symbol: 'ARKW', key: 'days_to_cover', expected: 378713 / 56692, status: 'partial', facts: { aggregate_short_shares: 378713, average_daily_share_volume: 56692, reported_days_to_cover: 6.68 } },
    { id: 'consensus', symbol: 'AIR.PA', key: 'mean_eps', expected: 2.82, facts: { q4_consensus_reported_eps: 2.82, q4_consensus_analyst_count: 20 } },
    { id: 'earnings_surprise', symbol: 'AIR.PA', key: 'earnings_surprise', expected: (3.27 - 2.82) / 2.82, facts: { q4_consensus_reported_eps: 2.82, q4_2025_reported_eps: 3.27 } },
    { id: 'forecast_dispersion', symbol: 'LIGHT.AS', key: 'forecast_dispersion', expected: (1579 - 1496) / 1533, facts: { q4_sales_high: 1579, q4_sales_low: 1496, q4_sales_average: 1533 } },
    { id: 'revision', symbol: 'AIR.PA', key: 'earnings_revision', expected: (6.01 - 5.85) / 5.85, facts: { mwb_2025_eps: 6.01, mwb_2025_previous_eps: 5.85 } },
    { id: 'target_upside', symbol: 'AIR.PA', key: 'target_upside', expected: (145 - 159.9) / 159.9, facts: { mwb_target_price: 145, mwb_current_price: 159.9 } },
  ];
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  for (const item of cases) {
    await page.goto('/learn');
    await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill(item.id);
    const card = page.locator(`.ax-kb-card[data-concept-id="${item.id}"]`);
    await expect(card).toHaveCount(1);
    await card.getByRole('button', { name: '在数据探索中实践' }).click();
    await expect(page).toHaveURL(new RegExp(`/data\\?concept=${item.id}&source=issuer_disclosure$`));
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const sent = page.waitForRequest(r => r.url().includes('/api/practice') && r.method() === 'POST');
    const received = page.waitForResponse(r => r.url().includes('/api/practice') && r.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [outbound, response] = await Promise.all([sent, received]);
    expect(outbound.postDataJSON()).toEqual({ concept_id: item.id, module: 'data', source: 'issuer_disclosure', symbol: item.symbol, inputs: {} });
    expect(response.ok(), `${item.id}: ${await response.text()}`).toBe(true);
    const result = await response.json();
    expect(result).toMatchObject({ concept_id: item.id, input_kind: 'industry_case', provenance: 'verified_original_public_source', source: 'issuer_disclosure', symbol: item.symbol, status: 'status' in item ? item.status : 'computed', bars: [], inputs: {} });
    expect(result.values[item.key]).toBeCloseTo(item.expected, 12);
    expect(result.units[item.key]).toBeTruthy();
    expect(Object.fromEntries(result.industry_facts.reported_facts.map((fact: { key: string; value: number }) => [fact.key, fact.value]))).toEqual(item.facts);
    expect(result.industry_facts.reported_facts.every((fact: { pdf_page: number | null; unit: string; source_url: string }) => (fact.pdf_page === null || fact.pdf_page > 0) && fact.unit.trim() !== '' && fact.source_url.startsWith('https://'))).toBe(true);
    expect(result.industry_facts.calculation.formula.trim()).not.toBe('');
    expect(result.industry_case.sources.every((source: { verification: { status: string } }) => source.verification.status === 'verified_immutable_cache')).toBe(true);
    const visual = panel.locator('.ax-industry-case');
    await expect(visual).toBeVisible();
    await expect(visual.locator('.ax-industry-formula')).toBeVisible();
    await expect(visual.locator('.ax-industry-unit-group')).not.toHaveCount(0);
    await expect(visual.locator('[data-industry-fact]')).toHaveCount(Object.keys(item.facts).length);
    await expect(visual).not.toContainText('PDF 第 0 页');
    await expect(visual.locator('[data-industry-fact] a')).toHaveCount(Object.keys(item.facts).length);
    await expect(visual.locator(`a[href="${result.industry_case.url}"]`).first()).toHaveAttribute('href', result.industry_case.url);
  }
  const visual = page.getByLabel('概念实践').locator('.ax-industry-case');
  await expect(visual).toBeVisible();
  await visual.screenshot({ path: test.info().outputPath('ownership-target-upside-dark.png') });
  await page.getByLabel('切换到浅色模式').click();
  await visual.screenshot({ path: test.info().outputPath('ownership-target-upside-light.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
  await visual.screenshot({ path: test.info().outputPath('ownership-target-upside-mobile.png') });
  expect(errors).toEqual([]);
});

test('Binance symbol picker reaches the complete tradable universe and searches beyond its startup seed', async ({ page, request }) => {
  test.setTimeout(90_000);
  const upstream = await request.get('https://data-api.binance.vision/api/v3/exchangeInfo?permissions=SPOT&showPermissionSets=false', { timeout: 30_000 });
  expect(upstream.ok()).toBe(true);
  const raw = await upstream.json();
  const expected = raw.symbols.filter((symbol: { symbol: string; status: string; quoteAsset: string; isSpotTradingAllowed: boolean }) => symbol.status === 'TRADING' && symbol.quoteAsset === 'USDT' && symbol.isSpotTradingAllowed).map((symbol: { symbol: string }) => symbol.symbol).sort() as string[];
  expect(expected.length).toBeGreaterThan(200);
  await expect.poll(async () => {
    const response = await request.get('/api/symbols?source=binance&limit=100');
    expect(response.ok()).toBe(true);
    return (await response.json()).complete;
  }, { timeout: 30_000, intervals: [250, 500, 1000] }).toBe(true);
  const listed: string[] = [];
  for (let offset = 0; ; offset += 100) {
    const response = await request.get(`/api/symbols?source=binance&limit=100&offset=${offset}`);
    expect(response.ok()).toBe(true);
    const catalog = await response.json();
    expect(catalog.complete).toBe(true);
    expect(catalog.universe_count).toBe(expected.length);
    listed.push(...catalog.symbols);
    if (!catalog.has_more) break;
  }
  expect(listed).toEqual(expected);
  await page.goto('/data');
  await page.getByRole('button', { name: '交易对', exact: true }).click();
  await expect(page.locator('.ax-symbol-status')).toContainText(`目录共 ${expected.length.toLocaleString('en-US')} 个`);
  const symbol = expected[expected.length - 1];
  await page.getByLabel('搜索交易对').fill(symbol);
  await expect(page.getByRole('option', { name: new RegExp(`^${symbol}\\b`) })).toBeVisible();
  await expect(page.locator('.ax-symbol-status')).toContainText(`目录共 ${expected.length.toLocaleString('en-US')} 个`);
  await page.getByRole('option', { name: new RegExp(`^${symbol}\\b`) }).click();
  await expect(page.getByRole('button', { name: '交易对', exact: true })).toContainText(symbol);
});

test('original book renders real pages and navigates its own bookmarks in both themes', async ({ page, request }) => {
  test.setTimeout(90_000);
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  const pdf = await request.get('/api/book/pdf');
  expect(pdf.ok()).toBe(true);
  expect((await pdf.body()).subarray(0, 5).toString()).toBe('%PDF-');
  await page.goto('/learn/book');
  const reader = page.locator('.ax-book-reader');
  await expect(reader).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  await expect(reader).toContainText('共 83 页');
  const toc = page.getByRole('navigation', { name: '原书目录' });
  await expect(toc.locator(':scope > ol > li')).toHaveCount(8);
  expect(await toc.evaluate(node => node.scrollHeight > node.clientHeight)).toBe(true);
  expect(await page.locator('.ax-book-toc').evaluate(node => node.clientHeight < window.innerHeight)).toBe(true);
  const chapter = toc.getByRole('button', { name: '第三部分 趋势、动量、波动与量价技术指标 第 33 页', exact: true });
  const windowScroll = await page.evaluate(() => window.scrollY);
  await chapter.click();
  const sheet = page.locator('#pdf-page-33');
  await expect.poll(() => sheet.evaluate(node => {
    const scroll = node.closest('.ax-book-scroll')!.getBoundingClientRect();
    const bounds = node.getBoundingClientRect();
    return Math.abs(bounds.top - scroll.top) < 40;
  })).toBe(true);
  expect(await page.evaluate(() => window.scrollY)).toBe(windowScroll);
  const canvas = sheet.locator('canvas');
  await expect(canvas).toBeVisible();
  // Check the actual PDF pixels, not merely a mounted canvas or a blank iframe.
  await expect.poll(() => canvas.evaluate((node: HTMLCanvasElement) => {
    if (!node.width || !node.height) return 0;
    const pixels = node.getContext('2d')!.getImageData(0, 0, node.width, node.height).data;
    let ink = 0;
    for (let i = 0; i < pixels.length; i += 16) if (pixels[i] < 220 || pixels[i + 1] < 220 || pixels[i + 2] < 220) ink++;
    return ink;
  })).toBeGreaterThan(300);
  await expect(chapter).toHaveClass(/active/);
  const dark = await toc.evaluate(node => ({ background: getComputedStyle(node.closest('aside')!).backgroundColor, text: getComputedStyle(node).color }));
  const tabs = page.locator('.ax-tabs');
  const snapshotStyle = await page.addStyleTag({ content: '.ax-tabs { visibility: hidden !important; }' });
  await expect(tabs).toHaveCSS('visibility', 'hidden');
  await expect(page.locator('.ax-book-toc')).toHaveScreenshot('book-toc-dark.png', { maxDiffPixelRatio: .01 });
  await page.getByLabel('切换到浅色模式').click();
  const light = await toc.evaluate(node => ({ background: getComputedStyle(node.closest('aside')!).backgroundColor, text: getComputedStyle(node).color }));
  expect(light.background).not.toBe(dark.background);
  expect(light.text).not.toBe(dark.text);
  await expect(page.locator('.ax-book-toc')).toHaveScreenshot('book-toc-light.png', { maxDiffPixelRatio: .01 });
  await page.getByLabel('收起原书目录').click();
  await expect(toc).toHaveCount(0);
  await page.getByLabel('展开原书目录').click();
  await expect(toc).toBeVisible();
  await page.getByLabel('当前页码').fill('82');
  await page.getByLabel('当前页码').press('Enter');
  await expect(page.locator('#pdf-page-82 canvas')).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await expect(page.locator('.ax-book-toc')).toHaveScreenshot('book-toc-mobile.png', { maxDiffPixelRatio: .01 });
  await snapshotStyle.evaluate(node => node.parentNode?.removeChild(node));
  await page.getByRole('button', { name: '指标大全', exact: true }).click();
  await expect(reader).toHaveCount(0);
  expect(errors).toEqual([]);
});

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
  await expect(page.locator('.ax-practice-result')).toContainText('所选标的的已收盘行情');
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
    await expect(page.getByLabel('成交与价格口径')).toHaveCount(1);
    await expect(page.getByLabel('形态来源与价格口径')).toHaveCount(1);
    await assertExecutionDisclosure(page, payload);
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
    await assertExecutionDisclosure(page, payload);
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
  const comparisonResponses: any[] = [];
  const onRequest = (request: import('@playwright/test').Request) => {
    const url = new URL(request.url());
    if (request.method() === 'POST' && url.pathname.endsWith('/api/backtest')) comparisonRequests.push(request.postDataJSON());
  };
  page.on('request', onRequest);
  const onResponse = async (response: import('@playwright/test').Response) => {
    if (response.url().includes('/api/backtest') && response.request().method() === 'POST' && response.ok()) comparisonResponses.push(await response.json());
  };
  page.on('response', onResponse);
  for (const market of markets) {
    comparisonRequests.length = 0;
    comparisonResponses.length = 0;
    await selectMarket(market);
    await page.getByRole('button', { name: /\u8dd1\u5bf9\u6bd4/ }).click();
    await expect.poll(() => comparisonRequests.length, { timeout: 45_000 }).toBe(2);
    for (const body of comparisonRequests) {
      expect(body.source).toBe(market.source);
      expect(body.symbol).toBe(market.symbol);
    }
    await expect.poll(() => page.locator('.ax-cmp-table tbody tr').count(), { timeout: 30_000 }).toBe(2);
    await expect(page.locator('.ax-chart svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
    await expect.poll(() => comparisonResponses.length).toBe(2);
    await assertExecutionDisclosure(page, comparisonResponses[0]);
  }
  page.off('request', onRequest);
  page.off('response', onResponse);

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
    await assertExecutionDisclosure(page, payload);
  }
});

test('backtest strategy charts show the returned equity path for each live market', async ({ page }, testInfo) => {
  test.setTimeout(180_000);
  await page.goto('/backtest');
  const markets = [
    { source: 'a_share', label: 'A 股 · 公开行情', symbol: '600519' },
    { source: 'us_stock', label: '美股 · 公开行情', symbol: 'AAPL' },
    { source: 'binance', label: '加密货币 · Binance', symbol: 'BTCUSDT' },
  ];
  for (const market of markets) {
    await page.getByRole('button', { name: '数据源', exact: true }).click();
    await page.getByRole('option', { name: market.label }).click();
    await expect(page.getByRole('button', { name: '交易对', exact: true })).toContainText(market.symbol);
    // 600519's 100-share minimum costs much more than the default 10,000.
    // Give each market enough cash to test strategy behaviour rather than
    // correctly rendering two flat, no-fill equity curves.
    if (market.source === 'a_share') await page.getByRole('spinbutton', { name: '资金' }).fill('1000000');
    const paths: string[] = [];
    const curves: number[][] = [];
    for (const strategy of ['买入持有 (基准)', '双均线交叉']) {
      await page.getByRole('button', { name: '策略', exact: true }).click();
      await page.getByRole('option', { name: strategy, exact: true }).click();
      const [response] = await Promise.all([
        page.waitForResponse(result => result.url().includes('/api/backtest') && result.request().method() === 'POST', { timeout: 45_000 }),
        page.getByRole('button', { name: '运行回测' }).click(),
      ]);
      expect(response.ok(), await response.text()).toBe(true);
      const result = await response.json();
      expect(result.source).toBe(market.source);
      expect(result.config.symbol).toBe(market.symbol);
      const curve = result.equity_curve.map((point: { equity: number }) => point.equity);
      expect(curve.length).toBeGreaterThan(40);
      expect(curve.every((equity: number) => Number.isFinite(equity) && equity > 0)).toBe(true);
      const chart = page.locator('.ax-chart').first();
      const line = chart.locator('g.scatterlayer path.js-line').first();
      await expect(chart.locator('svg.main-svg').first()).toBeVisible({ timeout: 20_000 });
      const expectedUnit = market.source === 'a_share' ? '元' : market.source === 'binance' ? 'USDT' : 'USD';
      await expect(chart.locator('.ytitle')).toContainText(`净值 (${expectedUnit})`);
      const netValue = page.locator('.ax-metric').filter({ has: page.locator('.ax-metric-label').getByText('净值', { exact: true }) }).locator('.ax-metric-value');
      await expect(netValue).toContainText(market.source === 'a_share' ? '¥' : market.source === 'binance' ? 'USDT' : '$');
      await expect(line).toHaveAttribute('d', /^M/);
      if (market.source !== 'binance') {
        const ticks = await chart.locator('.xtick text').allTextContents();
        expect(ticks.some(tick => /20\d{2}/.test(tick))).toBe(true);
        expect(ticks.every(tick => !tick.includes('00:00'))).toBe(true);
      }
      const path = await line.getAttribute('d');
      expect(path?.length).toBeGreaterThan(20);
      const bounds = await chart.boundingBox();
      expect(bounds).not.toBeNull();
      expect(bounds!.width).toBeGreaterThan(300);
      expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(1441);
      await testInfo.attach(`${market.source}-${result.config.strategy}-equity.png`, {
        body: await chart.screenshot(), contentType: 'image/png',
      });
      paths.push(path!);
      curves.push(curve);
    }
    expect(curves[0]).not.toEqual(curves[1]);
    expect(paths[0]).not.toEqual(paths[1]);
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
  await expect(roe).not.toContainText('待接入独立证据');
  await roe.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=roe&source=us_stock/);
  await expect(page.getByRole('button', { name: '数据源', exact: true })).toContainText('美股');
  await expect(page.getByRole('button', { name: '交易对' })).toContainText('AAPL');
  const roePractice = page.getByLabel('概念实践');
  await expect(roePractice).toContainText('ROE 净资产收益率');
  await expect(roePractice.locator('.ax-practice-inputs')).toHaveCount(0);
  await expect(roePractice).not.toContainText('教学示例');
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


test('formula convention practice server-fetches one real MACD series and draws both scales', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('数据供应商公式可能不同');
  const card = page.locator('.ax-kb-card').filter({ hasText: '数据供应商公式可能不同' });
  await expect(card).toHaveCount(1, { timeout: 30_000 });
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  const panel = page.getByLabel('概念实践');
  await expect(panel).toContainText('不是命名供应商的实测输出');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const [request, response] = await Promise.all([requestPromise, responsePromise]);
  const body = request.postDataJSON() as { concept_id: string; source: string; bars?: unknown; inputs: unknown };
  expect(body).toMatchObject({ concept_id: 'book_pitfall_formula_variant', source: 'binance', inputs: {} });
  expect(body.bars).toBeUndefined();
  expect(response.ok(), await response.text()).toBe(true);
  const payload = await response.json();
  expect(payload.context).toBe('selected_dataset');
  expect(payload.bar_origin).toBe('server_fetched_completed_source_bars');
  expect(payload.values.source_bar_seconds).toBe(3600);
  expect(payload.values.fast_period).toBe(12);
  expect(payload.values.slow_period).toBe(26);
  expect(payload.values.signal_period).toBe(9);
  const series = Object.fromEntries(payload.series.map((item: { name: string; values: Array<number | null> }) => [item.name, item.values]));
  expect(Object.keys(series)).toEqual(['macd_histogram_x1', 'macd_histogram_x2']);
  for (let index = 0; index < series.macd_histogram_x1.length; index += 1) {
    const one = series.macd_histogram_x1[index]; const two = series.macd_histogram_x2[index];
    if (one == null) expect(two).toBeNull();
    else expect(two).toBeCloseTo(2 * one, 12);
  }
  await expect(panel).toContainText('最新两种柱体约定之差');
  await expect(panel.locator('.ax-series-illustration svg')).toBeVisible();
});

test('open-candle practice shows an exchange-timed provisional snapshot without a fabricated final price', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('未收盘 K 线会变化');
  const card = page.locator('.ax-kb-card').filter({ hasText: '未收盘 K 线会变化' });
  await expect(card).toHaveCount(1, { timeout: 30_000 });
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const [request, response] = await Promise.all([requestPromise, responsePromise]);
  expect(request.postDataJSON()).toMatchObject({ concept_id: 'book_pitfall_open_candle', source: 'binance', inputs: {} });
  expect(request.postDataJSON().bars).toBeUndefined();
  expect(response.ok(), await response.text()).toBe(true);
  const payload = await response.json();
  expect(payload.bar_origin).toBe('server_fetched_binance_provisional_snapshot');
  expect(payload.bars).toHaveLength(1);
  expect(payload.provisional_snapshot.is_closed).toBe(false);
  expect(payload.provisional_snapshot.completion_evidence).toContain('timestamp-derived');
  expect(payload.values.is_current_candle_closed).toBe(0);
  expect(payload.values.final_close).toBeUndefined();
  expect(payload.values.last_completed_close).toBe(payload.bars[0].close);
  expect(payload.values.provisional_close).toBe(payload.provisional_snapshot.candle.close);
  expect(payload.values.current_candle_open_timestamp - payload.values.last_completed_timestamp).toBe(3600);
  expect(payload.values.as_of_timestamp).toBeGreaterThanOrEqual(payload.values.current_candle_open_timestamp);
  expect(payload.values.as_of_timestamp).toBeLessThan(payload.values.expected_close_timestamp);
  await expect(panel).toContainText('临时价尚未收盘');
  await expect(panel).toContainText('不能当作最终收盘价');
  await expect(panel.locator('.ax-series-illustration svg')).toBeVisible();
});

test('trade-volume practice keeps exact Binance base and quote volumes in separate units', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('成交量与成交额');
  const card = page.locator('.ax-kb-card').filter({ hasText: '成交量与成交额' });
  await expect(card).toHaveCount(1, { timeout: 30_000 });
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const [request, response] = await Promise.all([requestPromise, responsePromise]);
  expect(request.postDataJSON()).toMatchObject({ concept_id: 'book_trade_volume', source: 'binance', inputs: {} });
  expect(request.postDataJSON().bars).toBeUndefined();
  expect(response.ok(), await response.text()).toBe(true);
  const payload = await response.json();
  expect(payload.asset_units.base_asset).toBe('BTC');
  expect(payload.asset_units.quote_asset).toBe('USDT');
  expect(payload.bar_origin).toBe('server_fetched_completed_binance_usdt_spot_bars');
  expect(payload.bars).toHaveLength(24);
  expect(payload.values.base_volume).toBe(payload.bars.at(-1).volume);
  expect(payload.values.quote_volume).toBeGreaterThan(0);
  expect(payload.values.vwap).toBeCloseTo(payload.values.quote_volume / payload.values.base_volume, 8);
  expect(payload.series.map((item: { name: string }) => item.name)).toEqual(['base_volume_series', 'quote_volume_series']);
  await expect(panel).toContainText('成交额取自交易所汇总字段');
  await expect(panel).toContainText('成交数量（BTC）');
  await expect(panel).toContainText('实际成交额（USDT）');
  await expect(panel.locator('.ax-series-illustration svg')).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  const mobileChart = await panel.locator('.ax-trade-volume-chart').evaluate(element => ({ viewport: element.clientWidth, content: element.scrollWidth, svg: element.querySelector('svg')?.getBoundingClientRect().width || 0 }));
  expect(mobileChart.content).toBeGreaterThan(mobileChart.viewport);
  expect(mobileChart.svg).toBeGreaterThanOrEqual(700);
});

test('aggressor-side and CVD practices preserve exact Binance classifications and their 24-hour window', async ({ page }) => {
  test.setTimeout(180_000);
  for (const [name, conceptId] of [
    ['内盘 / 外盘', 'inside_outside'],
    ['订单流分类', 'book_order_flow'],
    ['CVD 累计成交量差', 'cvd'],
  ] as const) {
    await page.goto('/');
    await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill(name);
    const card = page.locator('.ax-kb-card').filter({ hasText: name });
    await expect(card).toHaveCount(1, { timeout: 30_000 });
    await card.getByRole('button', { name: '在数据探索中实践' }).click();
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    expect(request.postDataJSON()).toMatchObject({ concept_id: conceptId, source: 'binance', inputs: {} });
    expect(request.postDataJSON().bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.bar_origin).toBe('server_fetched_completed_binance_usdt_spot_1h_klines');
    expect(result.asset_units).toEqual({ base_asset: 'BTC', quote_asset: 'USDT' });
    expect(result.bars).toHaveLength(24);
    for (const bar of result.bars) {
      expect(bar.taker_buy_base_volume + bar.taker_sell_base_volume).toBeCloseTo(bar.total_base_volume, 8);
      expect(bar.taker_buy_quote_volume + bar.taker_sell_quote_volume).toBeCloseTo(bar.total_quote_volume, 6);
      expect(bar.total_base_volume).toBe(bar.volume);
    }
    await expect(panel.locator('.ax-flow-chart svg')).toBeVisible();
    if (conceptId !== 'cvd') await expect(panel).toContainText('这不是 A 股内外盘的等价数据');
    if (conceptId === 'cvd') {
      const cumulative = result.series.find((series: { name: string }) => series.name === 'cvd_base').values;
      let prefix = 0;
      result.bars.forEach((bar: { taker_buy_base_volume: number; taker_sell_base_volume: number }, index: number) => {
        prefix += bar.taker_buy_base_volume - bar.taker_sell_base_volume;
        expect(cumulative[index]).toBeCloseTo(prefix, 8);
      });
      await expect(panel).toContainText('不能把这里的数值称为全市场历史 CVD');
    }
  }
  await page.setViewportSize({ width: 390, height: 844 });
  const mobile = await page.locator('.ax-flow-chart').evaluate(element => ({ viewport: element.clientWidth, content: element.scrollWidth, svg: element.querySelector('svg')?.getBoundingClientRect().width || 0 }));
  expect(mobile.content).toBeGreaterThan(mobile.viewport);
  expect(mobile.svg).toBeGreaterThanOrEqual(700);
});

test('spot-depth practices read one live Binance order-book snapshot and keep quotes distinct from trades', async ({ page }) => {
  test.setTimeout(150_000);
  for (const [name, conceptId] of [
    ['买卖价差 Spread', 'bid_ask_spread'],
    ['委比与委差', 'book_order_imbalance'],
    ['委比高不代表必涨', 'book_pitfall_order_imbalance'],
  ] as const) {
    await page.goto('/');
    await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill(name);
    const card = page.locator('.ax-kb-card').filter({ hasText: name });
    await expect(card).toHaveCount(1, { timeout: 30_000 });
    await card.getByRole('button', { name: '在数据探索中实践' }).click();
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    expect(request.postDataJSON()).toMatchObject({ concept_id: conceptId, source: 'binance', limit: 5, inputs: {} });
    expect(request.postDataJSON().bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.provenance).toBe('server_fetched_binance_spot_order_book');
    expect(result.depth_snapshot.symbol).toBe('BTCUSDT');
    expect(result.depth_snapshot.timestamp).toBeNull();
    expect(result.depth_snapshot.update_id).toBeGreaterThan(0);
    expect(result.levels.bids.length).toBeGreaterThan(0);
    expect(result.levels.asks.length).toBeGreaterThan(0);
    expect(result.values.best_bid).toBe(result.levels.bids[0].price);
    expect(result.values.best_ask).toBe(result.levels.asks[0].price);
    expect(result.values.best_ask).toBeGreaterThan(result.values.best_bid);
    if (conceptId === 'bid_ask_spread') {
      expect(result.values.absolute_spread).toBeCloseTo(result.values.best_ask - result.values.best_bid, 8);
      expect(result.values.relative_spread).toBeCloseTo(result.values.absolute_spread / ((result.values.best_ask + result.values.best_bid) / 2), 10);
    } else {
      const bid = result.levels.bids.reduce((total: number, level: { quantity: number }) => total + level.quantity, 0);
      const ask = result.levels.asks.reduce((total: number, level: { quantity: number }) => total + level.quantity, 0);
      expect(result.values.top_n_bid_quantity).toBeCloseTo(bid, 8);
      expect(result.values.top_n_ask_quantity).toBeCloseTo(ask, 8);
      expect(result.values.order_imbalance).toBeCloseTo((bid - ask) / (bid + ask), 8);
    }
    await expect(panel.locator('.ax-depth-chart svg')).toBeVisible();
    await expect(panel).toContainText('没有交易所历史时间戳');
    if (conceptId === 'book_pitfall_order_imbalance') await expect(panel).toContainText('委比高不代表价格随后必涨');
  }
  await page.setViewportSize({ width: 390, height: 844 });
  const mobile = await page.locator('.ax-depth-chart').evaluate(element => ({ viewport: element.clientWidth, content: element.scrollWidth, svg: element.querySelector('svg')?.getBoundingClientRect().width || 0 }));
  expect(mobile.content).toBeGreaterThan(mobile.viewport);
  expect(mobile.svg).toBeGreaterThanOrEqual(650);
});

test('paired-market practices fetch and align real completed candles through the browser', async ({ page }) => {
  test.setTimeout(180_000);
  for (const conceptId of ['book_relative_strength_line', 'book_pair_spread', 'book_cointegration_diagnostic']) {
    await page.goto(`/data?concept=${conceptId}&source=binance`);
    const panel = page.getByLabel('概念实践');
    await expect(panel.getByRole('button', { name: '比较标的' })).toBeVisible();
    await expect(panel.locator('.ax-practice-inputs input')).toHaveCount(conceptId === 'book_pair_spread' ? 2 : 0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    const body = request.postDataJSON();
    expect(body).toMatchObject({ concept_id: conceptId, module: 'data', symbol: 'BTCUSDT', second_symbol: 'ETHUSDT', source: 'binance' });
    expect(body.bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.pair).toMatchObject({ first_symbol: 'BTCUSDT', second_symbol: 'ETHUSDT', interval: '1h', quote_asset: 'USDT' });
    expect(result.pair.matched_count).toBe(result.bars.length);
    expect(result.second_bars).toHaveLength(result.bars.length);
    expect(result.bars.length).toBeGreaterThanOrEqual(4);
    for (let index = 0; index < result.bars.length; index++) {
      expect(result.bars[index].timestamp).toBe(result.second_bars[index].timestamp);
      if (index) expect(new Date(result.bars[index].timestamp).getTime() - new Date(result.bars[index - 1].timestamp).getTime()).toBe(3_600_000);
    }
    const series = Object.fromEntries(result.series.map((item: { name: string; values: Array<number | null> }) => [item.name, item.values])) as Record<string, Array<number | null>>;
    if (conceptId === 'book_relative_strength_line') {
      const last = result.bars.length - 1;
      expect(series.relative_strength_line[last]).toBeCloseTo(result.bars[last].close / result.second_bars[last].close, 10);
    }
    if (conceptId === 'book_pair_spread') {
      const last = result.bars.length - 1;
      expect(series.spread[last]).toBeCloseTo(result.bars[last].close - body.inputs.hedge_ratio * result.second_bars[last].close, 8);
    }
    if (conceptId === 'book_cointegration_diagnostic') {
      expect(result.values.cointegration_p_value).toBeNull();
      await expect(panel).toContainText('不单凭该统计量认定协整');
    }
    await expect(panel.locator('.ax-series-illustration svg')).toBeVisible();
    await expect(panel).toContainText('两只 Binance USDT 现货的同时刻已收盘小时线');
  }
});

test('relative-strength knowledge card previews the real paired series in both themes', async ({ page }) => {
  test.setTimeout(60_000);
  await page.goto('/');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('相对强弱线 Relative Strength Line');
  const card = page.locator('.ax-kb-card').filter({ hasText: '相对强弱线 Relative Strength Line' });
  await card.locator('.ax-kb-details').click();
  await expect(card).toContainText('BTCUSDT 与 ETHUSDT 同时刻的真实已收盘小时线');
  const chart = card.locator('.ax-series-illustration svg');
  await expect(chart).toBeVisible();
  const darkStroke = await chart.locator('path[style*="stroke"]').first().evaluate(node => getComputedStyle(node).stroke);
  await page.getByLabel('切换到浅色模式').click();
  const lightStroke = await chart.locator('path[style*="stroke"]').first().evaluate(node => getComputedStyle(node).stroke);
  expect(lightStroke).not.toBe(darkStroke);
  await expect(chart).toBeVisible();
});

test('52-week stock practice derives its range from real daily highs and lows in the browser', async ({ page }) => {
  test.setTimeout(120_000);
  for (const [source, symbol] of [['a_share', '600519'], ['us_stock', 'AAPL']] as const) {
    await page.goto(`/data?concept=book_52w_range&source=${source}`);
    const panel = page.getByLabel('概念实践');
    await expect(panel).toContainText('52周区间');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    expect(request.postDataJSON()).toMatchObject({ concept_id: 'book_52w_range', module: 'data', source, symbol, inputs: {} });
    expect(request.postDataJSON().bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.year_range.source).toBe(source);
    expect(result.provenance).toBe('server_fetched_completed_stock_daily_bars');
    expect(result.year_range.price_basis).toBe('provider_ohlc_adjustment_unverified');
    expect(result.year_range.bar_count).toBe(result.bars.length);
    expect(result.year_range.bar_count).toBeGreaterThanOrEqual(180);
    expect(new Date(result.year_range.pre_window_observation).getTime()).toBeLessThanOrEqual(new Date(result.year_range.window_start).getTime());
    expect(result.values.high_52w).toBe(Math.max(...result.bars.map((bar: { high: number }) => bar.high)));
    expect(result.values.low_52w).toBe(Math.min(...result.bars.map((bar: { low: number }) => bar.low)));
    expect(result.values.latest_close).toBe(result.bars.at(-1).close);
    expect(result.values.distance_from_high).toBeCloseTo(result.values.latest_close / result.values.high_52w - 1, 10);
    await expect(panel.locator('.ax-year-range')).toBeVisible();
    await expect(panel).toContainText('复权口径未经统一核验');
  }
  await page.setViewportSize({ width: 390, height: 844 });
  const visual = page.locator('.ax-year-range');
  expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
  expect(await visual.locator('.ax-year-range-prices span').evaluateAll(labels => labels.every(label => {
    const box = label.getBoundingClientRect(), figure = label.closest('figure')!.getBoundingClientRect();
    return box.left >= figure.left && box.right <= figure.right;
  }))).toBe(true);
  const dark = await visual.evaluate(node => getComputedStyle(node).backgroundColor);
  await page.getByLabel('切换到浅色模式').click();
  expect(await visual.evaluate(node => getComputedStyle(node).backgroundColor)).not.toBe(dark);
});

test('recent-trade concepts derive tick net volume and price distribution from exchange executions', async ({ page }) => {
  test.setTimeout(120_000);
  for (const conceptId of ['book_net_volume', 'volume_profile']) {
    await page.goto(`/data?concept=${conceptId}&source=binance`);
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    expect(request.postDataJSON()).toMatchObject({ concept_id: conceptId, module: 'data', source: 'binance', symbol: 'BTCUSDT', inputs: {} });
    expect(request.postDataJSON().bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.provenance).toBe('server_fetched_binance_recent_trades');
    expect(result.recent_trades.window_kind).toBe('recent_observed_trades');
    expect(result.trades.length).toBe(result.recent_trades.trade_count);
    expect(result.trades.length).toBeGreaterThanOrEqual(2);
    for (let index = 1; index < result.trades.length; index++) {
      expect(result.trades[index].id).toBe(result.trades[index - 1].id + 1);
      expect(new Date(result.trades[index].timestamp).getTime()).toBeGreaterThanOrEqual(new Date(result.trades[index - 1].timestamp).getTime());
    }
    if (conceptId === 'book_net_volume') {
      let up = 0, down = 0, neutral = 0;
      for (let index = 1; index < result.trades.length; index++) {
        const trade = result.trades[index], previous = result.trades[index - 1];
        if (trade.price > previous.price) up += trade.quantity;
        else if (trade.price < previous.price) down += trade.quantity;
        else neutral += trade.quantity;
      }
      expect(result.values.uptick_volume).toBeCloseTo(up, 8);
      expect(result.values.downtick_volume).toBeCloseTo(down, 8);
      expect(result.values.neutral_volume).toBeCloseTo(neutral, 8);
      expect(result.values.net_volume).toBeCloseTo(up - down, 8);
      expect(result.recent_trades.analyzed_trade_count).toBe(result.trades.length - 1);
      await expect(panel.locator('.ax-trade-directions')).toBeVisible();
      await expect(panel).toContainText('首笔只作前价锚点');
    } else {
      const groups = new Map<number, number>();
      for (const trade of result.trades) groups.set(trade.price, (groups.get(trade.price) || 0) + trade.quantity);
      expect(result.profile_levels).toHaveLength(groups.size);
      for (const level of result.profile_levels) expect(level.volume).toBeCloseTo(groups.get(level.price)!, 8);
      const included = result.profile_levels.filter((level: { in_value_area: boolean }) => level.in_value_area).reduce((sum: number, level: { volume: number }) => sum + level.volume, 0);
      expect(result.values.included_fraction).toBeCloseTo(included / result.values.total_volume, 8);
      expect(result.values.poc).toBe(result.profile_levels.find((level: { is_poc: boolean }) => level.is_poc).price);
      await expect(panel.locator('.ax-trade-profile svg')).toBeVisible();
      await expect(panel.locator('.ax-trade-profile rect[data-price-level]')).toHaveCount(groups.size);
      await expect(panel).toContainText('不是全天成交分布');
    }
  }
  await page.setViewportSize({ width: 390, height: 844 });
  const profile = page.locator('.ax-trade-profile');
  expect(await profile.evaluate(node => node.scrollWidth > node.clientWidth)).toBe(true);
});

test('Bitcoin block practices read linked mainnet blocks and independently recompute heights and bytes', async ({ page }) => {
  test.setTimeout(120_000);
  for (const conceptId of ['book_block_height', 'book_block_size']) {
    await page.goto(`/data?concept=${conceptId}&source=binance`);
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    expect(request.postDataJSON()).toMatchObject({ concept_id: conceptId, module: 'data', source: 'binance', symbol: 'BTCUSDT', limit: 10, inputs: {} });
    expect(request.postDataJSON().bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.provenance).toBe('server_fetched_bitcoin_block_snapshot');
    expect(result.block_snapshot.network).toBe('bitcoin_mainnet');
    expect(result.block_snapshot.endpoint).toMatch(/^https:\/\/(blockstream\.info|mempool\.space)\/api\/blocks$/);
    expect(result.blocks).toHaveLength(10);
    expect(result.blocks[0].height).toBe(result.block_snapshot.first_height);
    expect(result.blocks[9].height).toBe(result.block_snapshot.last_height);
    for (let index = 1; index < result.blocks.length; index++) {
      expect(result.blocks[index].height).toBe(result.blocks[index - 1].height + 1);
      expect(result.blocks[index].previous_hash).toBe(result.blocks[index - 1].hash);
    }
    if (conceptId === 'book_block_height') {
      expect(result.values.latest_height - result.values.reference_height).toBe(9);
      expect(result.values.blocks_since_reference).toBe(9);
      await expect(panel).toContainText('十个区块只有九段相邻关系');
    } else {
      const total = result.blocks.reduce((sum: number, block: { size_bytes: number }) => sum + block.size_bytes, 0);
      expect(result.values.total_size_bytes).toBe(total);
      expect(result.values.mean_size_bytes).toBeCloseTo(total / 10, 9);
      await expect(panel.locator('.ax-block-size-bar')).toHaveCount(10);
    }
    await expect(panel.locator('.ax-block-snapshot')).toBeVisible();
  }
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.locator('.ax-block-scroll').evaluate(node => node.scrollWidth > node.clientWidth)).toBe(true);
});

test('Bitcoin transaction practices read a pinned block page and recompute fees and bytes in the browser', async ({ page }) => {
  test.setTimeout(180_000);
  for (const conceptId of ['book_transaction_fees', 'book_transaction_bytes']) {
    await page.goto(`/data?concept=${conceptId}&source=binance`);
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    const body = request.postDataJSON();
    expect(body).toMatchObject({ concept_id: conceptId, module: 'data', source: 'binance', symbol: 'BTCUSDT', limit: 25, inputs: {} });
    expect(body.bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.provenance).toBe('server_fetched_bitcoin_transaction_sample');
    expect(result.transaction_sample.network).toBe('bitcoin_mainnet');
    expect(result.transaction_sample.endpoint).toMatch(/^https:\/\/(blockstream\.info|mempool\.space)\/api\/block\/[0-9a-f]{64}\/txs\/0$/);
    expect(result.transaction_sample.block_hash).toMatch(/^[0-9a-f]{64}$/);
    expect(result.transaction_sample.block_height).toBeGreaterThan(0);
    expect(result.transaction_sample.page_start).toBe(0);
    expect(result.transaction_sample.returned_count).toBe(result.transaction_sample.analyzed_count + result.transaction_sample.excluded_coinbase_count);
    expect(result.transaction_sample.excluded_coinbase_count).toBe(1);
    expect(result.transaction_sample.scope).toBe('first_page_non_coinbase_transactions');
    expect(result.transaction_sample.observed_newer_blocks).toBe(6);
    expect(result.transactions.length).toBeGreaterThan(0);
    expect(result.transactions.length).toBeLessThanOrEqual(24);
    expect(new Set(result.transactions.map((tx: { txid: string }) => tx.txid)).size).toBe(result.transactions.length);
    expect(result.transactions.every((tx: { txid: string; fee_sats: number; size_bytes: number }) => /^[0-9a-f]{64}$/.test(tx.txid) && Number.isInteger(tx.fee_sats) && tx.fee_sats >= 0 && Number.isInteger(tx.size_bytes) && tx.size_bytes > 0)).toBe(true);
    const values = result.transactions.map((tx: { fee_sats: number; size_bytes: number }) => conceptId === 'book_transaction_fees' ? tx.fee_sats : tx.size_bytes);
    const total = values.reduce((sum: number, value: number) => sum + value, 0);
    const sorted = [...values].sort((a, b) => a - b);
    const median = sorted.length % 2 ? sorted[Math.floor(sorted.length / 2)] : (sorted[sorted.length / 2 - 1] + sorted[sorted.length / 2]) / 2;
    const prefix = conceptId === 'book_transaction_fees' ? 'fee' : 'size';
    expect(result.values[`total_${prefix === 'fee' ? 'fee_sats' : 'size_bytes'}`]).toBe(total);
    expect(result.values[`mean_${prefix === 'fee' ? 'fee_sats' : 'size_bytes'}`]).toBeCloseTo(total / values.length, 9);
    expect(result.values[`median_${prefix === 'fee' ? 'fee_sats' : 'size_bytes'}`]).toBe(median);
    await expect(panel.locator('.ax-transaction-sample')).toBeVisible();
    await expect(panel.locator('svg[role="img"]')).toBeVisible();
    await expect(panel.locator('[data-txid]')).toHaveCount(values.length);
    await expect(panel.locator('a[href*="blockstream.info/block/"]')).toHaveAttribute('href', `https://blockstream.info/block/${result.transaction_sample.block_hash}`);
    await expect(panel).toContainText('固定区块首页');
    await expect(panel).toContainText('不是完整区块');
  }
});

test('Bitcoin UTXO practices independently recompute observed first-page inputs and outputs', async ({ page, request }) => {
  test.setTimeout(180_000);
  for (const conceptId of ['book_utxo_value_stats', 'book_utxo_counts', 'book_utxo_totals']) {
    await page.goto(`/data?concept=${conceptId}&source=binance`);
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const response = await responsePromise;
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.provenance).toBe('server_fetched_bitcoin_transaction_first_page');
    expect(result.utxo_sample.network).toBe('bitcoin_mainnet');
    expect(result.utxo_sample.endpoint).toMatch(/^https:\/\/(blockstream\.info|mempool\.space)\/api\/block\/[0-9a-f]{64}\/txs\/0$/);
    expect(result.utxo_sample.scope).toBe('confirmed_pinned_block_first_page_noncoinbase_transactions');
    expect(result.utxo_sample.total_utxo_scope).toBe('undefined_not_derived_from_first_page_sample');
    expect(result.utxo_transactions.length).toBe(result.utxo_sample.sampled_noncoinbase_transaction_count);
    const provider = await request.get(result.utxo_sample.endpoint, { timeout: 30_000 });
    expect(provider.ok()).toBe(true);
    const raw = await provider.json();
    expect(raw.length).toBe(result.utxo_sample.returned_count);
    expect(raw[0].vin[0].is_coinbase).toBe(true);
    const ordinary = raw.slice(1);
    expect(ordinary.length).toBe(result.utxo_transactions.length);
    const spent: number[] = [];
    const created: number[] = [];
    for (const [index, tx] of result.utxo_transactions.entries()) {
      const original = ordinary[index];
      expect(tx.txid).toBe(original.txid);
      expect(original.status.confirmed).toBe(true);
      expect(original.status.block_hash).toBe(result.utxo_sample.block_hash);
      const inputValues = original.vin.map((input: { prevout: { value: number } }) => input.prevout.value);
      const outputValues = original.vout.filter((output: { scriptpubkey_type: string }) => output.scriptpubkey_type !== 'op_return').map((output: { value: number }) => output.value);
      expect(tx.input_prevout_values_sats).toEqual(inputValues);
      expect(tx.non_op_return_output_values_sats).toEqual(outputValues);
      expect(tx.spent_prevout_value_sats).toBe(inputValues.reduce((total: number, value: number) => total + value, 0));
      expect(tx.created_non_op_return_value_sats).toBe(outputValues.reduce((total: number, value: number) => total + value, 0));
      expect(inputValues.reduce((total: number, value: number) => total + value, 0)).toBe(original.vout.reduce((total: number, output: { value: number }) => total + output.value, original.fee));
      spent.push(...inputValues);
      created.push(...outputValues);
    }
    expect(result.values.spent_prevout_count).toBe(spent.length);
    expect(result.values.created_non_op_return_output_count).toBe(created.length);
    expect(result.values.spent_prevout_value_sats).toBe(spent.reduce((sum, value) => sum + value, 0));
    expect(result.values.created_non_op_return_value_sats).toBe(created.reduce((sum, value) => sum + value, 0));
    if (conceptId === 'book_utxo_value_stats') {
      const median = (values: number[]) => { const sorted = [...values].sort((a, b) => a - b); return sorted.length ? sorted.length % 2 ? sorted[Math.floor(sorted.length / 2)] : (sorted[sorted.length / 2 - 1] + sorted[sorted.length / 2]) / 2 : null; };
      expect(result.values.spent_median_value_sats).toBe(median(spent));
      expect(result.values.created_median_value_sats).toBe(median(created));
      expect(result.values.spent_mean_value_sats).toBeCloseTo(spent.reduce((a, b) => a + b, 0) / spent.length, 8);
      expect(result.values.created_mean_value_sats).toBeCloseTo(created.reduce((a, b) => a + b, 0) / created.length, 8);
    } else {
      expect(result.status).toBe('partial');
      expect(result.values[conceptId === 'book_utxo_counts' ? 'total_utxo_count' : 'total_utxo_value_sats']).toBeNull();
      await expect(panel).toContainText('全网当前 UTXO');
    }
    await expect(panel.locator('.ax-utxo-sample')).toBeVisible();
    await expect(panel.locator('[data-utxo-txid]')).toHaveCount(ordinary.length);
    await expect(panel.locator(`a[href="https://blockstream.info/block/${result.utxo_sample.block_hash}"]`)).toBeVisible();
  }
});

test('Bitcoin script address sets match the original confirmed block page rather than people or transfers', async ({ page, request }) => {
  test.setTimeout(120_000);
  await page.goto('/data?concept=book_sending_receiving&source=binance');
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const [sent, response] = await Promise.all([requestPromise, responsePromise]);
  expect(sent.postDataJSON()).toMatchObject({ concept_id: 'book_sending_receiving', module: 'data', source: 'binance', inputs: {} });
  expect(sent.postDataJSON().bars).toBeUndefined();
  expect(response.ok(), await response.text()).toBe(true);
  const result = await response.json();
  expect(result.provenance).toBe('server_fetched_bitcoin_transaction_first_page');
  expect(result.address_sample.network).toBe('bitcoin_mainnet');
  expect(result.address_sample.endpoint).toMatch(/^https:\/\/(blockstream\.info|mempool\.space)\/api\/block\/[0-9a-f]{64}\/txs\/0$/);
  const provider = await request.get(result.address_sample.endpoint, { timeout: 30_000 });
  expect(provider.ok(), await provider.text()).toBe(true);
  const raw = await provider.json();
  expect(raw[0].vin[0].is_coinbase).toBe(true);
  const ordinary = raw.slice(1);
  expect(ordinary.length).toBe(result.address_transactions.length);
  const sending = new Set<string>(), receiving = new Set<string>();
  let missingInputs = 0, missingOutputs = 0, excluded = 0;
  type Script = { scriptpubkey_address?: string | null; scriptpubkey_type: string };
  for (const [index, tx] of result.address_transactions.entries()) {
    const original = ordinary[index];
    expect(tx.txid).toBe(original.txid);
    expect(original.status.confirmed).toBe(true);
    expect(original.status.block_hash).toBe(result.address_sample.block_hash);
    const inputs: Script[] = original.vin.map((input: { prevout: Script }) => input.prevout);
    const outputs: Script[] = original.vout.filter((output: Script) => output.scriptpubkey_type !== 'op_return');
    const decoded = (scripts: Script[]) => [...new Set(scripts.flatMap(script => typeof script.scriptpubkey_address === 'string' ? [script.scriptpubkey_address] : []))].sort();
    expect(tx.input_addresses).toEqual(decoded(inputs));
    expect(tx.output_addresses).toEqual(decoded(outputs));
    const inputMissing = inputs.filter(script => !script.scriptpubkey_address).length;
    const outputMissing = outputs.filter(script => !script.scriptpubkey_address).length;
    const opReturn = original.vout.length - outputs.length;
    expect(tx.missing_input_address_slots).toBe(inputMissing);
    expect(tx.missing_output_address_slots).toBe(outputMissing);
    expect(tx.excluded_op_return_output_count).toBe(opReturn);
    decoded(inputs).forEach(address => sending.add(address));
    decoded(outputs).forEach(address => receiving.add(address));
    missingInputs += inputMissing; missingOutputs += outputMissing; excluded += opReturn;
  }
  const shared = [...sending].filter(address => receiving.has(address)).length;
  expect(result.values.unique_sending_script_address_count).toBe(sending.size);
  expect(result.values.unique_receiving_script_address_count).toBe(receiving.size);
  expect(result.values.shared_script_address_count).toBe(shared);
  expect(result.values.union_script_address_count).toBe(new Set([...sending, ...receiving]).size);
  expect(result.values.missing_input_address_count).toBe(missingInputs);
  expect(result.values.missing_output_address_count).toBe(missingOutputs);
  expect(result.values.excluded_op_return_output_count).toBe(excluded);
  const visual = panel.locator('.ax-address-sample');
  await expect(visual).toBeVisible();
  await expect(panel.locator('[data-address-txid]')).toHaveCount(ordinary.length);
  await expect(panel.locator(`a[href="${new URL(result.address_sample.endpoint).origin}/block/${result.address_sample.block_hash}"]`)).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await visual.evaluate(node => node.getBoundingClientRect().right <= window.innerWidth)).toBe(true);
  const dark = await visual.evaluate(node => getComputedStyle(node).backgroundColor);
  await page.getByLabel('切换到浅色模式').click();
  expect(await visual.evaluate(node => getComputedStyle(node).backgroundColor)).not.toBe(dark);
});

test('Bitcoin block timing and transaction rate use exactly nine linked header-time intervals', async ({ page }) => {
  test.setTimeout(120_000);
  for (const conceptId of ['book_block_interval', 'book_transaction_rate']) {
    await page.goto(`/data?concept=${conceptId}&source=binance`);
    const panel = page.getByLabel('概念实践');
    await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
    const requestPromise = page.waitForRequest(request => request.url().includes('/api/practice') && request.method() === 'POST');
    const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
    await panel.getByRole('button', { name: '运行实践' }).click();
    const [request, response] = await Promise.all([requestPromise, responsePromise]);
    expect(request.postDataJSON()).toMatchObject({ concept_id: conceptId, module: 'data', source: 'binance', symbol: 'BTCUSDT', limit: 10, inputs: {} });
    expect(request.postDataJSON().bars).toBeUndefined();
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.provenance).toBe('server_fetched_bitcoin_block_snapshot');
    expect(result.block_snapshot.network).toBe('bitcoin_mainnet');
    expect(result.block_snapshot.endpoint).toMatch(/^https:\/\/(blockstream\.info|mempool\.space)\/api\/blocks$/);
    expect(result.blocks).toHaveLength(10);
    expect(result.intervals).toHaveLength(9);
    const deltas: number[] = [];
    for (let index = 1; index < result.blocks.length; index++) {
      const older = result.blocks[index - 1], newer = result.blocks[index];
      expect(newer.height).toBe(older.height + 1);
      expect(newer.previous_hash).toBe(older.hash);
      expect(Number.isInteger(newer.tx_count) && newer.tx_count > 0).toBe(true);
      const seconds = (Date.parse(newer.timestamp) - Date.parse(older.timestamp)) / 1000;
      deltas.push(seconds);
      expect(result.intervals[index - 1]).toMatchObject({ from_height: older.height, to_height: newer.height, seconds });
    }
    expect(Number.isInteger(result.blocks[0].tx_count) && result.blocks[0].tx_count > 0).toBe(true);
    const total = deltas.reduce((sum, value) => sum + value, 0);
    const ordered = [...deltas].sort((a, b) => a - b);
    const median = ordered[4];
    expect(result.values.nonpositive_interval_count).toBe(deltas.filter(value => value <= 0).length);
    await expect(panel.locator('.ax-block-timing')).toBeVisible();
    await expect(panel.locator('.ax-block-snapshot')).toHaveCount(0);
    await expect(panel.locator('.ax-block-timing [data-value]')).toHaveCount(9);
    if (conceptId === 'book_block_interval') {
      expect(result.values.interval_count).toBe(9);
      expect(result.values.total_declared_span_seconds).toBe(total);
      expect(result.values.mean_block_interval_seconds).toBeCloseTo(total / 9, 9);
      expect(result.values.median_block_interval_seconds).toBe(median);
      for (let index = 0; index < 9; index++) await expect(panel.locator(`.ax-block-timing [data-height="${result.blocks[index + 1].height}"]`)).toHaveAttribute('data-value', String(deltas[index]));
      await expect(panel).toContainText('不是实测出块耗时');
    } else {
      const count = result.blocks.slice(1).reduce((sum: number, block: { tx_count: number }) => sum + block.tx_count, 0);
      expect(result.values.confirmed_transaction_count).toBe(count);
      expect(result.values.elapsed_seconds).toBe(total);
      expect(result.values.included_block_count).toBe(9);
      expect(result.anchor_block_excluded).toBe(true);
      expect(panel.locator(`.ax-block-timing [data-height="${result.blocks[0].height}"]`)).toHaveCount(0);
      for (let index = 1; index < 10; index++) await expect(panel.locator(`.ax-block-timing [data-height="${result.blocks[index].height}"]`)).toHaveAttribute('data-value', String(result.blocks[index].tx_count));
      if (total > 0) {
        expect(result.status).toBe('computed');
        expect(result.values.transaction_rate).toBeCloseTo(count / total, 9);
      } else {
        expect(result.status).toBe('undefined');
        expect(result.values.transaction_rate).toBeNull();
        await expect(panel).toContainText('本次速率无法定义');
      }
      await expect(panel).toContainText('交易数包含每块的 coinbase');
    }
  }
});

test('Apple filing case verifies the issuer PDF and computes every supported metric from one historical statement', async ({ page, request }) => {
  test.setTimeout(180_000);
  const officialUrl = 'https://www.apple.com/newsroom/pdfs/fy2025-q4/FY25_Q4_Consolidated_Financial_Statements.pdf';
  const officialPdf = await request.get(officialUrl, { timeout: 30_000 });
  expect(officialPdf.ok(), await officialPdf.text()).toBe(true);
  const officialBytes = await officialPdf.body();
  expect(officialBytes).toHaveLength(4_919_649);
  expect(createHash('sha256').update(officialBytes).digest('hex')).toBe('43e7f0730b3cce0fc37301a2f43c29712bbde6ab299d97c6df345fd0c754508a');

  await page.goto('/');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('book_fcf');
  const card = page.locator('.ax-kb-card').filter({ has: page.getByRole('heading', { name: '自由现金流', exact: true }) });
  await expect(card).toHaveCount(1, { timeout: 30_000 });
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=book_fcf&source=us_stock$/);
  const panel = page.getByLabel('概念实践');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  const requestPromise = page.waitForRequest(sent => sent.url().includes('/api/practice') && sent.method() === 'POST');
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const [sent, response] = await Promise.all([requestPromise, responsePromise]);
  expect(sent.postDataJSON()).toEqual({ concept_id: 'book_fcf', module: 'data', symbol: 'AAPL', source: 'us_stock', inputs: {} });
  expect(response.ok(), await response.text()).toBe(true);
  const result = await response.json();
  expect(result).toMatchObject({
    concept_id: 'book_fcf', input_kind: 'filing_case', provenance: 'verified_issuer_filing_case', status: 'computed',
    module: 'data', source: 'us_stock', symbol: 'AAPL', context: 'historical_filing_case', bar_origin: 'server_verified_issuer_filing_pdf',
    values: { free_cash_flow: 98_767 }, units: { free_cash_flow: 'USD millions' }, bars: [], inputs: {}, series: [],
    filing_case: {
      case_id: 'apple_fy2025_q4_annual_gaap_press_release', issuer: 'Apple Inc.', ticker: 'AAPL',
      period: { start: '2024-09-29', end: '2025-09-27', fiscal_year: 2025 },
      comparison_period: { start: '2023-10-01', end: '2024-09-28', fiscal_year: 2024 },
      published: '2025-10-30', audited: false, source_kind: 'issuer_official_earnings_release_pdf',
      url: officialUrl, sha256: '43e7f0730b3cce0fc37301a2f43c29712bbde6ab299d97c6df345fd0c754508a', bytes: 4_919_649,
      pages: { income_statement: 1, balance_sheet: 2, cash_flow: 3 },
    },
  });
  expect(result.filing_case.status).toContain('Unaudited');
  expect(result.filing_case.status).toContain('not an annual report');
  expect(result.filing_case.verification.requested_url).toBe(officialUrl);
  expect(result.filing_case.verification.matched_bytes).toBe(4_919_649);
  expect(result.filing_case.verification.matched_sha256).toBe('43e7f0730b3cce0fc37301a2f43c29712bbde6ab299d97c6df345fd0c754508a');
  expect(result.facts.calculation_boundaries).toEqual({
    eps: 'Reported EPS is authoritative. Net income in millions divided by weighted shares in thousands is only an approximate cross-check because both inputs are rounded in the release.',
    cash: 'Cash-flow ending cash agrees to the balance-sheet cash and cash equivalents in each reported year.',
    capex: 'PPE capex is the absolute value of the reported cash-flow line Payments for acquisition of property, plant and equipment.',
  });

  const visual = panel.locator('.ax-filing-case');
  await expect(visual).toBeVisible();
  await expect(visual).toContainText('历史财务案例 · 原文已核验');
  await expect(visual).toContainText('Apple Inc. · FY2025');
  await expect(visual).toContainText('2024-09-29 至 2025-09-27 · 公告发布 2025-10-30');
  await expect(visual).toContainText('未经审计');
  await expect(visual).toContainText('不是年度报告');
  await expect(visual).toContainText('不是当前行情或当前所选股票的财务数据');
  await expect(visual.locator('.ax-filing-results')).toContainText('98,767 百万美元');
  await expect(visual.locator('figure')).toContainText('经营现金流 − 购置固定资产的现金支出');
  await expect(visual.locator('[data-filing-fact="cfo"]')).toContainText('111,482 百万美元');
  await expect(visual.locator('[data-filing-fact="ppe_capex_cash_outflow"]')).toContainText('12,715 百万美元');
  await expect(visual.locator('.ax-filing-fact')).toHaveCount(2);
  await expect(visual.locator('.ax-filing-bar')).toHaveCount(4);
  await expect(visual.locator(`a[href="${officialUrl}#page=3"]`)).toHaveCount(2);
  const captureCase = async (name: string) => {
    await page.evaluate(() => document.fonts.ready);
    await visual.scrollIntoViewIfNeeded();
    await expect(visual).toBeVisible();
    const original = await visual.evaluate(node => {
      const rect = node.getBoundingClientRect();
      return { width: rect.width, height: rect.height, position: getComputedStyle(node).position, scrollY: window.scrollY };
    });
    expect(original.position).not.toBe('fixed');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await visual.evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
    await visual.getByRole('link', { name: '第 3 页 ↗' }).first().click({ trial: true });
    // Keep the live component and its natural dimensions; isolate only the
    // screenshot origin from asynchronous charts and page scrolling outside it.
    const snapshotStyle = await page.addStyleTag({ content: `.ax-tabs { visibility: hidden !important; } .ax-filing-case { position: fixed !important; top: 0 !important; left: 0 !important; width: ${original.width}px !important; box-sizing: border-box; margin: 0 !important; transform: none !important; z-index: 10000; }` });
    try {
      const isolated = await visual.boundingBox();
      expect(isolated).not.toBeNull();
      expect(isolated!.x).toBe(0);
      expect(isolated!.y).toBe(0);
      expect(isolated!.width).toBeCloseTo(original.width, 1);
      expect(isolated!.height).toBeCloseTo(original.height, 1);
      await page.mouse.move(0, 0);
      await expect(visual).toHaveScreenshot(name, { maxDiffPixelRatio: .01 });
    } finally {
      await snapshotStyle.evaluate(node => node.parentNode?.removeChild(node));
      await page.evaluate(scrollY => window.scrollTo(0, scrollY), original.scrollY);
    }
    expect(await visual.evaluate(node => getComputedStyle(node).position)).toBe(original.position);
  };
  await captureCase('book-fcf-dark.png');
  await page.getByLabel('切换到浅色模式').click();
  await captureCase('book-fcf-light.png');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await visual.evaluate(node => node.getBoundingClientRect().right <= window.innerWidth)).toBe(true);
  await captureCase('book-fcf-mobile.png');

  // Literal inputs transcribed from pages 1–3 of the checked Apple announcement.
  // Expected values are recomputed here without importing application code or its embedded case JSON.
  const i = { sales: 416_161, cogs: 220_960, gross: 195_201, operating: 133_050, net: 112_010, basicEps: 7.49, dilutedEps: 7.46, basicShares: 14_948_500, dilutedShares: 15_004_697 };
  const b = { cash: 35_934, securities: 18_763, receivables: 39_777, inventory: 5_718, currentAssets: 147_957, payables: 69_860, commercialPaper: 7_979, currentDebt: 12_350, currentLiabilities: 165_631, noncurrentDebt: 78_328, assets: 359_241, liabilities: 285_508, equity: 73_733 };
  const prior = { sales: 391_035, receivables: 33_410, inventory: 7_286, payables: 68_960, assets: 364_980, equity: 56_950 };
  const cashFlow = { cfo: 111_482, capex: 12_715 };
  const averageAssets = (b.assets + prior.assets) / 2;
  const averageEquity = (b.equity + prior.equity) / 2;
  const expected: Record<string, Record<string, number>> = {
    eps: { reported_basic_eps: i.basicEps, reported_diluted_eps: i.dilutedEps, approx_basic_eps_cross_check: i.net * 1_000 / i.basicShares, approx_diluted_eps_cross_check: i.net * 1_000 / i.dilutedShares },
    book_diluted_shares: { weighted_diluted_shares: i.dilutedShares, reported_diluted_eps: i.dilutedEps, approx_diluted_eps_cross_check: i.net * 1_000 / i.dilutedShares },
    book_revenue: { revenue: i.sales },
    book_gross_margin: { gross_margin: i.gross / i.sales },
    book_ebit_margin: { ebit_margin: i.operating / i.sales },
    book_net_margin: { net_margin: i.net / i.sales },
    book_current_ratio: { current_ratio: b.currentAssets / b.currentLiabilities },
    book_quick_ratio: { quick_ratio: (b.cash + b.securities + b.receivables) / b.currentLiabilities },
    book_cash_ratio: { cash_ratio: b.cash / b.currentLiabilities },
    book_debt_ratio: { debt_ratio: b.liabilities / b.assets },
    book_de_ratio: { debt_to_equity: b.liabilities / b.equity },
    book_net_debt: { net_debt: b.commercialPaper + b.currentDebt + b.noncurrentDebt - b.cash },
    book_cfo: { operating_cash_flow: cashFlow.cfo },
    book_capex: { capital_expenditure: cashFlow.capex },
    book_fcf: { free_cash_flow: cashFlow.cfo - cashFlow.capex },
    fcf: { free_cash_flow: cashFlow.cfo - cashFlow.capex },
    book_cfo_income: { cfo_to_net_income: cashFlow.cfo / i.net },
    book_asset_turnover: { asset_turnover: i.sales / averageAssets },
    book_roa: { return_on_average_assets: i.net / averageAssets },
    book_inventory_turnover: { inventory_turnover: i.cogs / ((b.inventory + prior.inventory) / 2) },
    book_receivable_turnover: { receivable_turnover_proxy: i.sales / ((b.receivables + prior.receivables) / 2) },
    book_dpo: { days_payable_outstanding: ((b.payables + prior.payables) / 2) / i.cogs * 364 },
    accrual_ratio: { accrual_ratio: (i.net - cashFlow.cfo) / averageAssets },
    book_yoy: { revenue_year_over_year: (i.sales - prior.sales) / prior.sales },
    book_ccc: {
      cash_conversion_cycle: ((b.inventory + prior.inventory) / 2) / i.cogs * 364 + ((b.receivables + prior.receivables) / 2) / i.sales * 364 - ((b.payables + prior.payables) / 2) / i.cogs * 364,
      days_sales_outstanding_proxy: ((b.receivables + prior.receivables) / 2) / i.sales * 364,
      days_inventory_outstanding: ((b.inventory + prior.inventory) / 2) / i.cogs * 364,
      days_payable_outstanding: ((b.payables + prior.payables) / 2) / i.cogs * 364,
    },
    roe: { return_on_average_equity: i.net / averageEquity },
    dupont: { net_margin: i.net / i.sales, asset_turnover: i.sales / averageAssets, equity_multiplier: averageAssets / averageEquity, dupont_roe: i.net / averageEquity },
    book_roce: { return_on_capital_employed: i.operating / (b.assets - b.currentLiabilities) },
  };
  expect(Object.keys(expected)).toHaveLength(28);
  for (const [conceptId, expectedValues] of Object.entries(expected)) {
    const apiResponse = await request.post('/api/practice', { data: { concept_id: conceptId, module: 'data', symbol: 'AAPL', source: 'us_stock', inputs: {} } });
    expect(apiResponse.ok(), `${conceptId}: ${await apiResponse.text()}`).toBe(true);
    const value = await apiResponse.json();
    expect(value.filing_case.case_id).toBe('apple_fy2025_q4_annual_gaap_press_release');
    expect(value.context).toBe('historical_filing_case');
    expect(value.filing_case.url).toBe(officialUrl);
    expect(value.filing_case.audited).toBe(false);
    expect(value.facts.annual_income_statement['2025']).toMatchObject({ page: 1, sales: i.sales, cogs: i.cogs, grossprofit: i.gross, operatingincome: i.operating, netincome: i.net, basic_eps: i.basicEps, diluted_eps: i.dilutedEps, weighted_basic_shares: i.basicShares, weighted_diluted_shares: i.dilutedShares });
    expect(value.facts.balance_sheets['2025-09-27']).toMatchObject({ page: 2, cash: b.cash, current_securities: b.securities, accounts_receivable: b.receivables, inventory: b.inventory, current_assets: b.currentAssets, accounts_payable: b.payables, commercial_paper: b.commercialPaper, current_term_debt: b.currentDebt, current_liabilities: b.currentLiabilities, noncurrent_term_debt: b.noncurrentDebt, assets: b.assets, liabilities: b.liabilities, equity: b.equity });
    expect(value.facts.annual_cash_flows['2025']).toMatchObject({ page: 3, cfo: cashFlow.cfo, ppe_capex_cash_outflow: cashFlow.capex, ending_cash: b.cash });
    expect(Object.keys(value.values).sort()).toEqual(Object.keys(expectedValues).sort());
    for (const [key, expectedValue] of Object.entries(expectedValues)) expect(value.values[key], `${conceptId}.${key}`).toBeCloseTo(expectedValue, 10);
    if (conceptId === 'accrual_ratio') expect(value.notes.join(' ')).toContain('接近零只描述该历史期间');
    if (conceptId === 'book_yoy') expect(value.notes.join(' ')).toContain('不是 EPS、季度收入或当前增长率');
    if (conceptId === 'book_ccc') expect(value.notes.join(' ')).toContain('负CCC是公式结果，不按零截断');
  }
});


test('Moutai share structure independently verifies annual-report page 47 and public practice', async ({ page, request }) => {
  test.setTimeout(120_000);
  const url = 'https://static.cninfo.com.cn/finalpage/2026-04-17/1225114741.PDF';
  const upstream = await request.get(url, { timeout: 60_000 });
  expect(upstream.ok()).toBe(true);
  const bytes = await upstream.body();
  expect(bytes).toHaveLength(1082847);
  expect(createHash('sha256').update(bytes).digest('hex')).toBe('474905deeaf0f875fc0a1b097a626c0c7852c427faadc5d7fc7816cbf45ea288');
  const { getDocument } = await import('pdfjs-dist/legacy/build/pdf.mjs');
  const loadingTask = getDocument({ data: new Uint8Array(bytes), useSystemFonts: true });
  const pdf = await loadingTask.promise;
  const text = (await (await pdf.getPage(47)).getTextContent()).items.map(item => 'str' in item ? item.str : '').join('').replace(/\s/g, '');
  for (const literal of ['单位：股', '无限售条件流通股份', '股份总数', '1,256,197,800', '3,927,585', '1,252,270,215', '2025年8月30日']) expect(text).toContain(literal);
  await loadingTask.destroy();
  await page.goto('/learn');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('book_share_counts');
  const card = page.locator('.ax-kb-card[data-concept-id="book_share_counts"]');
  await expect(card).not.toContainText('使用透明教学输入');
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=book_share_counts&source=issuer_disclosure$/);
  const panel = page.getByLabel('概念实践');
  const received = page.waitForResponse(r => r.url().includes('/api/practice') && r.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const response = await received;
  expect(response.ok(), await response.text()).toBe(true);
  expect(response.request().postDataJSON()).toEqual({ concept_id: 'book_share_counts', module: 'data', source: 'issuer_disclosure', symbol: '600519', inputs: {} });
  const result = await response.json();
  // Independently read page 47 above; these are raw count literals, not copied API output.
  expect(Object.fromEntries(result.industry_facts.reported_facts.map((f: {key: string; value: number}) => [f.key, f.value]))).toEqual({ opening_total_shares: 1256197800, cancelled_shares: 3927585, closing_total_shares: 1252270215, unrestricted_shares: 1252270215 });
  expect(1256197800 - 3927585).toBe(1252270215);
  expect(result.values).toEqual({ restricted_shares_residual: 1252270215 - 1252270215 });
  expect(result.industry_case).toMatchObject({ published: '2026-04-17', period: { end: '2025-12-31' }, pdf_pages: [47], url, bytes: 1082847 });
  const visual = panel.getByRole('region', { name: '股本结构 股本披露案例' });
  await expect(visual).toContainText('无限售条件流通股份不等于自由流通股');
  await expect(visual).toContainText('公告日期不冒充注销生效日');
  await expect(visual.locator('[data-industry-fact]')).toHaveCount(4);
  await expect(visual.locator('[data-industry-fact] a').first()).toHaveAttribute('href', `${url}#page=47`);
  await captureHistoricalCard(page, visual, 'moutai-shares-real-dark.png');
  await page.getByLabel('切换到浅色模式').click();
  await captureHistoricalCard(page, visual, 'moutai-shares-real-light.png');
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await visual.evaluate(node => node.getBoundingClientRect().right <= window.innerWidth)).toBe(true);
  await captureHistoricalCard(page, visual, 'moutai-shares-real-mobile.png');
});

test('Moutai free float independently reconciles issuer holders, CSI methodology and same-day price', async ({ page, request }) => {
  test.setTimeout(180_000);
  const documents = [
    { url: 'https://static.cninfo.com.cn/finalpage/2025-12-30/1224906220.PDF', bytes: 92857, sha: 'ae056c65f53fcacd0b2cffd3562af0f2696d32593e25543ba279b59b07e57ee6', pages: [1, 2, 3], facts: ['681,282,935', '27,849,688', '709,132,623'] },
    { url: 'https://oss-ch.csindex.com.cn/static/html/csindex/public/uploads/indices/detail/files/zh_CN/000300_Index_Methodology_cn.pdf', bytes: 1184619, sha: 'de6491e03e5d57ecf1aca104b1412543643a59e387a5343c9bd21ecbbdeba5b6', pages: [3, 4, 5, 6], facts: ['自由流通量', '5%'] },
  ];
  const { getDocument } = await import('pdfjs-dist/legacy/build/pdf.mjs');
  for (const document of documents) {
    let bytes: Buffer;
    try {
      const response = await request.get(document.url, { timeout: 60_000 });
      expect(response.ok()).toBe(true);
      bytes = await response.body();
    } catch (error) {
      if (!/ECONNRESET|ETIMEDOUT|EAI_AGAIN|ENOTFOUND|socket hang up/i.test(String(error))) throw error;
      // CI runners can be refused by the official host. The pinned original
      // still has to pass the byte count, SHA-256 and PDF fact checks below.
      bytes = await readFile(resolve(process.cwd(), '..', 'data', 'verified-sources', `${document.sha}.pdf`));
    }
    expect(bytes).toHaveLength(document.bytes);
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(document.sha);
    const task = getDocument({ data: new Uint8Array(bytes), useSystemFonts: true });
    const pdf = await task.promise;
    const texts = await Promise.all(document.pages.map(async pageNumber => (await (await pdf.getPage(pageNumber)).getTextContent()).items.map(item => 'str' in item ? item.str : '').join('').replace(/\s/g, '')));
    for (const fact of document.facts) expect(texts.join('')).toContain(fact);
    await task.destroy();
  }

  const priceUrl = 'https://web.ifzq.gtimg.cn/appstock/app/kline/kline?param=sh600519,day,2025-12-31,2025-12-31,1';
  const priceResponse = await request.get(priceUrl, { timeout: 30_000 });
  expect(priceResponse.ok()).toBe(true);
  const price = (await priceResponse.json()).data.sh600519;
  expect(price.day).toHaveLength(1);
  const [date, , close] = price.day[0];
  expect(date).toBe('2025-12-31');
  expect(Number(close)).toBe(1377.18);

  const unrestricted = 1_252_270_215;
  const independentlyDerivedFree = unrestricted - 681_282_935 - 27_849_688;
  const independentlyDerivedCap = unrestricted * Number(close);
  expect(independentlyDerivedFree).toBe(543_137_592);
  expect(independentlyDerivedCap).toBeCloseTo(1_724_601_494_693.70, 2);
  for (const [conceptId, expected] of [['book_free_float', independentlyDerivedFree / unrestricted], ['book_float_market_cap', independentlyDerivedCap]] as const) {
    const response = await request.post('/api/practice', { data: { concept_id: conceptId, module: 'data', source: 'issuer_disclosure', symbol: '600519', inputs: {} } });
    expect(response.ok(), await response.text()).toBe(true);
    const result = await response.json();
    expect(result.provenance).toBe('verified_independent_a_share_float_case');
    if (conceptId === 'book_float_market_cap') expect(Math.abs(result.values[conceptId] - expected)).toBeLessThan(0.01);
    else expect(result.values[conceptId]).toBeCloseTo(expected, 10);
    expect(result.values.free_float_shares).toBe(independentlyDerivedFree);
    expect(result.values.unrestricted_shares).toBe(unrestricted);
    expect(result.float_case.price).toMatchObject({ trading_date: date, close: Number(close), basis: 'unadjusted_daily_close' });
    expect(result.float_case.sources.slice(0, 3).map((source: { sha256: string }) => source.sha256)).toEqual(['474905deeaf0f875fc0a1b097a626c0c7852c427faadc5d7fc7816cbf45ea288', ...documents.map(document => document.sha)]);
  }

  await page.goto('/learn');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('book_free_float');
  const card = page.locator('.ax-kb-card[data-concept-id="book_free_float"]');
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=book_free_float&source=issuer_disclosure$/);
  const panel = page.getByLabel('概念实践');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const visual = panel.getByRole('region', { name: /A股自由流通口径案例/ });
  await expect(visual).toContainText('543,137,592');
  await expect(visual).toContainText('43.3722%');
  await expect(visual).toContainText('1,724,601,494,693.70');
  await visual.locator('.ax-filing-sources > summary').click();
  await expect(visual.getByRole('link', { name: '发行人一致行动关系公告 ↗' })).toHaveAttribute('href', documents[0].url);
  await visual.locator('.ax-filing-sources > summary').click();
  await captureHistoricalCard(page, visual, 'moutai-float-real-dark.png');
  await page.getByLabel('切换到浅色模式').click();
  await captureHistoricalCard(page, visual, 'moutai-float-real-light.png');
  await page.setViewportSize({ width: 390, height: 844 });
  await captureHistoricalCard(page, visual, 'moutai-float-real-mobile.png');
});
