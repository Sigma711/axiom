import { expect, test } from './v8-coverage';
import type { APIRequestContext } from '@playwright/test';

type RawBar = { date: string; close: number; volume: number };
async function rawMemberBars(request: APIRequestContext, member: { provider: string; endpoint: string }): Promise<RawBar[]> {
  const response = await request.get(member.endpoint, { headers: { 'user-agent': 'AXIOM independent breadth verifier/1.0', accept: 'application/json' }, timeout: 30_000 });
  expect(response.ok(), `${member.provider} ${member.endpoint}: ${await response.text()}`).toBe(true);
  const raw = await response.json();
  if (/yahoo/i.test(member.provider)) {
    const result = raw.chart.result[0], timestamps = result.timestamp as number[], quote = result.indicators.quote[0];
    return timestamps.flatMap((timestamp, index) => Number.isFinite(quote.close[index]) && Number.isFinite(quote.volume[index]) ? [{ date: new Date(timestamp * 1000).toISOString().slice(0, 10), close: quote.close[index], volume: quote.volume[index] }] : []);
  }
  if (/nasdaq/i.test(member.provider)) {
    return raw.data.tradesTable.rows.flatMap((row: Record<string, string>) => {
      const [month, day, year] = row.date.split('/');
      const close = Number(row.close.replace(/[$,]/g, '')), volume = Number(row.volume.replace(/[$,]/g, ''));
      return Number.isFinite(close) && Number.isFinite(volume) ? [{ date: `${year}-${month}-${day}`, close, volume }] : [];
    }).sort((a: RawBar, b: RawBar) => a.date.localeCompare(b.date));
  }
  throw new Error(`No independent parser for provider ${member.provider}`);
}

test('market breadth knowledge uses the live fixed basket and independently recomputes returned breadth', async ({ page, request }) => {
  test.setTimeout(240_000);
  await page.goto('/learn');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('ad_line');
  const card = page.locator('.ax-kb-card[data-concept-id="ad_line"]');
  await expect(card).toHaveCount(1);
  const received = page.waitForResponse(response => response.url().includes('/api/market-breadth/snapshot'));
  await card.locator('summary').click();
  const response = await received;
  expect(response.ok(), await response.text()).toBe(true);
  const snapshot = await response.json();

  expect(snapshot.schema_version).toBe(1);
  expect(snapshot.universe).toMatchObject({ id: 'dow_30_2024_11_08', member_count: 30, constituents_as_of: '2024-11-08' });
  expect(snapshot.universe.symbols).toHaveLength(30);
  expect(new Set(snapshot.universe.symbols).size).toBe(30);
  expect(snapshot.source.members).toHaveLength(30);
  expect(snapshot.concepts).toHaveLength(8);
  expect(new Set(snapshot.concepts.map((concept: { id: string }) => concept.id))).toEqual(new Set(['ad_line', 'trin', 'mcclellan', 'new_high_low', 'tick', 'breadth_thrust', 'bullish_percent', 'up_down_volume']));
  expect(snapshot.observations.length).toBeGreaterThan(39);
  for (const observation of snapshot.observations) {
    expect(observation.eligible_members).toBeGreaterThanOrEqual(snapshot.coverage.minimum_required);
    expect(observation.advances + observation.declines + observation.unchanged).toBe(observation.eligible_members);
  }

  const concept = (id: string) => snapshot.concepts.find((item: { id: string }) => item.id === id);
  const latest = snapshot.observations.at(-1);
  const rawBars: RawBar[][] = [];
  for (let index = 0; index < snapshot.source.members.length; index += 6) {
    rawBars.push(...await Promise.all(snapshot.source.members.slice(index, index + 6).map((member: { provider: string; endpoint: string }) => rawMemberBars(request, member))));
  }
  const rawDaily = new Map<string, { eligible: number; advances: number; declines: number }>();
  for (const bars of rawBars) for (let index = 1; index < bars.length; index += 1) {
    const current = bars[index], previous = bars[index - 1];
    const row = rawDaily.get(current.date) || { eligible: 0, advances: 0, declines: 0 };
    row.eligible += 1;
    if (current.close > previous.close) row.advances += 1;
    else if (current.close < previous.close) row.declines += 1;
    rawDaily.set(current.date, row);
  }
  const responseWindowStart = snapshot.observations[0].date;
  const independentlyAccumulatedAd = [...rawDaily.entries()].filter(([date, row]) => date >= responseWindowStart && date <= snapshot.as_of && row.eligible >= snapshot.coverage.minimum_required).sort(([left], [right]) => left.localeCompare(right)).reduce((sum, [, row]) => sum + row.advances - row.declines, 0);
  expect(concept('ad_line').latest).toBe(independentlyAccumulatedAd);
  const independentlyEligible = rawBars.flatMap(bars => {
    const currentIndex = bars.findIndex(bar => bar.date === latest.date);
    return currentIndex > 0 ? [{ previous: bars[currentIndex - 1], current: bars[currentIndex] }] : [];
  });
  const independent = independentlyEligible.reduce((total, member) => {
    if (member.current.close > member.previous.close) { total.advances += 1; total.up_volume += member.current.volume; }
    else if (member.current.close < member.previous.close) { total.declines += 1; total.down_volume += member.current.volume; }
    else total.unchanged += 1;
    return total;
  }, { advances: 0, declines: 0, unchanged: 0, up_volume: 0, down_volume: 0 });
  expect({ eligible_members: independentlyEligible.length, ...independent }).toEqual({ eligible_members: latest.eligible_members, advances: latest.advances, declines: latest.declines, unchanged: latest.unchanged, up_volume: latest.up_volume, down_volume: latest.down_volume });
  expect(concept('trin').latest).toBeCloseTo((independent.advances / independent.declines) / (independent.up_volume / independent.down_volume), 12);
  const bpi = concept('bullish_percent');
  if (bpi.status === 'available') expect(bpi.latest).toBeCloseTo(bpi.inputs.buy_signals / bpi.inputs.eligible_signals * 100, 12);
  else expect(bpi.reason).toMatch(/Point & Figure/i);
  const tick = concept('tick');
  if (tick.status === 'available') {
    expect(tick.inputs.members).toHaveLength(3);
    const rawDirections = await Promise.all(tick.inputs.members.map(async (member: { source_url: string }) => {
      const rawResponse = await request.get(member.source_url, { timeout: 30_000 });
      expect(rawResponse.ok(), await rawResponse.text()).toBe(true);
      const trades = await rawResponse.json();
      const previous = Number(trades.at(-2).p), current = Number(trades.at(-1).p);
      return current > previous ? 1 : current < previous ? -1 : 0;
    }));
    expect(tick.latest).toBe(rawDirections.reduce((sum, direction) => sum + direction, 0));
  } else {
    expect(tick.reason).toMatch(/intraday|Binance|TICK/i);
  }

  const visual = card.getByLabel('Advance–Decline Line 腾落线真实市场宽度实践');
  await expect(visual).toBeVisible();
  await expect(visual.locator('[data-breadth-concept]')).toHaveCount(8);
  await expect(visual).toContainText(`${snapshot.coverage.latest_eligible_members} / 30`);
  await visual.locator('summary').click();
  await expect(visual.getByRole('link', { name: /S&P DJI 成分变更公告/ })).toHaveAttribute('href', snapshot.universe.constituents_source);
  await expect(visual.getByRole('link', { name: new RegExp(`${snapshot.source.members[0].symbol} · ${snapshot.source.members[0].provider}`) })).toHaveAttribute('href', snapshot.source.members[0].endpoint);
  await visual.locator('summary').click();
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=ad_line&source=market_breadth$/);
  const panel = page.getByLabel('概念实践');
  await expect(panel).toContainText('不采用本页单一标的或手动输入');
  await expect(panel.locator('.ax-practice-inputs')).toHaveCount(0);
  await expect(page.locator('.ax-controls')).toHaveCount(0);
  const postPromise = page.waitForResponse(candidate => candidate.url().endsWith('/api/practice') && candidate.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const post = await postPromise;
  expect(post.request().postDataJSON()).toEqual({ concept_id: 'ad_line', module: 'data', source: 'market_breadth', inputs: {} });
  expect(post.ok(), await post.text()).toBe(true);
  const practice = await post.json();
  expect(practice).toMatchObject({ concept_id: 'ad_line', source: 'market_breadth', symbol: null, source_markets: { daily: 'us_equity' } });
  expect(practice.market_breadth.schema_version).toBe(1);
  await expect(panel.getByLabel('Advance–Decline Line 腾落线真实市场宽度实践')).toBeVisible();

  await page.goto('/data?concept=tick&source=market_breadth');
  const tickPanel = page.getByLabel('概念实践');
  await expect(tickPanel.locator('.ax-practice-inputs')).toHaveCount(0);
  const tickPostPromise = page.waitForResponse(candidate => candidate.url().endsWith('/api/practice') && candidate.request().method() === 'POST');
  await tickPanel.getByRole('button', { name: '运行实践' }).click();
  const tickResponse = await tickPostPromise;
  expect(tickResponse.request().postDataJSON()).toEqual({ concept_id: 'tick', module: 'data', source: 'market_breadth', inputs: {} });
  expect(tickResponse.ok(), await tickResponse.text()).toBe(true);
  const tickPractice = await tickResponse.json();
  expect(tickPractice).toMatchObject({ concept_id: 'tick', source: 'market_breadth', symbol: null, source_markets: { tick: 'crypto_spot' } });
  expect(Object.keys(tickPractice.source_markets)).toEqual(['tick']);
  expect(tickPractice.bars).toEqual([]);
  expect(tickPractice.market_tick.members).toHaveLength(3);
  expect(tickPractice.market_breadth).toBeUndefined();
  await expect(tickPanel.getByLabel('TICK 指数真实市场宽度实践')).toContainText('真实逐笔快照');
});

test('the original-book McClellan summation opens and draws its own real practice', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/learn');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('book_mcclellan_sum');
  const card = page.locator('.ax-kb-card[data-concept-id="book_mcclellan_sum"]');
  await expect(card).toHaveCount(1);
  await card.locator('summary').click();
  await expect(card.getByLabel('McClellan Summation Index 麦克莱伦累积指数真实市场宽度实践')).toBeVisible();
  await card.getByRole('button', { name: '在数据探索中实践' }).click();
  await expect(page).toHaveURL(/\/data\?concept=book_mcclellan_sum&source=market_breadth$/);
  const panel = page.getByLabel('概念实践');
  const postPromise = page.waitForResponse(response => response.url().endsWith('/api/practice') && response.request().method() === 'POST');
  await panel.getByRole('button', { name: '运行实践' }).click();
  const post = await postPromise;
  expect(post.request().postDataJSON()).toEqual({ concept_id: 'book_mcclellan_sum', module: 'data', source: 'market_breadth', inputs: {} });
  expect(post.ok(), await post.text()).toBe(true);
  const result = await post.json();
  const oscillator = result.market_breadth.concepts.find((item: { id: string }) => item.id === 'mcclellan').series;
  const independentSum = oscillator.reduce((sum: number, item: { value: number }) => sum + item.value, 0);
  expect(oscillator.length).toBeGreaterThan(0);
  expect(result.values.book_mcclellan_sum).toBeCloseTo(independentSum, 10);
  expect(result.source_markets).toEqual({ daily: 'us_equity' });
  await expect(panel.getByLabel('McClellan Summation Index 麦克莱伦累积指数真实市场宽度实践')).toBeVisible();
  await expect(panel.locator('[data-breadth-selected="book_mcclellan_sum"]')).toContainText('McClellan Summation Index');
});

test('TICK knowledge preview stays available without the US daily breadth endpoint', async ({ page }) => {
  test.setTimeout(90_000);
  let dailyRequests = 0;
  await page.route('**/api/market-breadth/snapshot', async route => {
    dailyRequests += 1;
    await route.fulfill({ status: 503, body: 'US daily source unavailable' });
  });
  await page.goto('/learn');
  await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill('tick');
  const card = page.locator('.ax-kb-card[data-concept-id="tick"]');
  await expect(card).toHaveCount(1);
  const postPromise = page.waitForResponse(response => response.url().endsWith('/api/practice') && response.request().method() === 'POST');
  await card.locator('summary').click();
  const post = await postPromise;
  expect(post.request().postDataJSON()).toEqual({ concept_id: 'tick', module: 'data', source: 'market_breadth', inputs: {} });
  expect(post.ok(), await post.text()).toBe(true);
  expect(dailyRequests).toBe(0);
  const visual = card.getByLabel('TICK 指数真实市场宽度实践');
  await expect(visual).toContainText('真实逐笔快照');
  await expect(visual.locator('[data-breadth-concept]')).toHaveCount(0);
});

test('every remaining daily breadth concept opens a real practice from its knowledge card', async ({ page }) => {
  test.setTimeout(300_000);
  for (const conceptId of ['trin', 'mcclellan', 'new_high_low', 'breadth_thrust', 'bullish_percent', 'up_down_volume']) {
    await test.step(conceptId, async () => {
      await page.goto('/learn');
      await page.getByRole('textbox', { name: '搜索 概念 / 公式 / 关键词' }).fill(conceptId);
      const card = page.locator(`.ax-kb-card[data-concept-id="${conceptId}"]`);
      await expect(card).toHaveCount(1);
      await card.locator('summary').click();
      await expect(card.locator('.ax-breadth-shell')).toBeVisible();
      await card.getByRole('button', { name: '在数据探索中实践' }).click();
      await expect(page).toHaveURL(new RegExp(`/data\\?concept=${conceptId}&source=market_breadth$`));
      const panel = page.getByLabel('概念实践');
      const postPromise = page.waitForResponse(response => response.url().endsWith('/api/practice') && response.request().method() === 'POST');
      await panel.getByRole('button', { name: '运行实践' }).click();
      const post = await postPromise;
      expect(post.ok(), `${conceptId}: ${await post.text()}`).toBe(true);
      const result = await post.json();
      expect(result).toMatchObject({ concept_id: conceptId, source_markets: { daily: 'us_equity' } });
      expect(result.market_breadth.concepts.find((item: { id: string }) => item.id === conceptId).status).toBe('available');
      await expect(panel.locator(`[data-breadth-selected="${conceptId}"]`)).toBeVisible();
    });
  }
});
