import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, appBase, appPath, fmtMoney, fmtNum, fmtPct, fmtPctSigned, preferredCodeLocationUrl } from './api';

const fetchMock = vi.fn();

afterEach(() => {
  fetchMock.mockReset();
  vi.unstubAllGlobals();
});

function response(body: unknown, init: { ok?: boolean; status?: number } = {}) {
  return {
    ok: init.ok ?? true,
    status: init.status ?? 200,
    json: vi.fn().mockResolvedValue(body),
    text: vi.fn().mockResolvedValue(typeof body === 'string' ? body : JSON.stringify(body)),
  } as unknown as Response;
}

function useFetch(body: unknown = { ok: true }) {
  fetchMock.mockResolvedValue(response(body));
  vi.stubGlobal('fetch', fetchMock);
}

it('keeps application requests and routes under Vite deployment base', () => {
  expect(appBase === '' || appBase.startsWith('/')).toBe(true);
  expect(appPath('/api/knowledge')).toBe(`${appBase}/api/knowledge`);
  expect(appPath('/backtest')).toBe(`${appBase}/backtest`);
});

describe('API client requests', () => {
  it('encodes symbol catalog and data query parameters', async () => {
    useFetch({ symbols: [], items: [], source: 'a_share' });

    await api.listSymbols('a_share', '中 1', 50, 25);
    expect(fetchMock).toHaveBeenLastCalledWith(
      `${appBase}/api/symbols?source=a_share&q=%E4%B8%AD%201&offset=50&limit=25`,
      expect.objectContaining({ method: 'GET', headers: { 'Content-Type': 'application/json' } }),
    );

    await api.getData('BTC/USDT', 120, 'binance');
    expect(fetchMock).toHaveBeenLastCalledWith(
      `${appBase}/api/data?symbol=BTC%2FUSDT&limit=120&source=binance`,
      expect.objectContaining({ method: 'GET' }),
    );

    await api.getIndicators('A B', 'sma_20,rsi_14', 30, 'us_stock');
    expect(fetchMock).toHaveBeenLastCalledWith(
      `${appBase}/api/indicators?symbol=A%20B&indicators=sma_20%2Crsi_14&limit=30&source=us_stock`,
      expect.objectContaining({ method: 'GET' }),
    );

    await api.getPatterns('ETH', 10, 'binance');
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/patterns?symbol=ETH&limit=10&source=binance`, expect.anything());
    await api.getHeikinAshi('ETH', 10, 'binance');
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/heikin_ashi?symbol=ETH&limit=10&source=binance`, expect.anything());
    await api.getCodeLocation('src/main.rs#L10');
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/code_loc?ref=src%2Fmain.rs%23L10`, expect.anything());
  });

  it('uses GET endpoints and returns decoded payloads', async () => {
    const payload = { value: 'decoded' };
    useFetch(payload);

    await expect(api.listStrategies()).resolves.toEqual(payload);
    await expect(api.getConfig()).resolves.toEqual(payload);
    await expect(api.listKnowledge()).resolves.toEqual(payload);
    await expect(api.paperSnapshot()).resolves.toEqual(payload);
    await expect(api.listPractice()).resolves.toEqual(payload);

    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/practice`, expect.objectContaining({ method: 'GET' }));
  });

  it('sends backtest, paper, and practice payloads as JSON', async () => {
    useFetch({ status: 'ok' });
    const backtest = { strategy: 'sma_cross', symbol: 'BTCUSDT', source: 'binance' as const, limit: 100, initial_capital: 1000, params: { fast: 5 } };
    await api.runBacktest(backtest);
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/backtest`, expect.objectContaining({ method: 'POST', body: JSON.stringify(backtest) }));

    await api.paperStart();
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/paper/start`, expect.objectContaining({ method: 'POST' }));
    await api.paperStop();
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/paper/stop`, expect.objectContaining({ method: 'POST' }));
    await api.paperStrategy('rsi');
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/paper/strategy`, expect.objectContaining({ method: 'POST', body: JSON.stringify({ strategy: 'rsi' }) }));
    await api.paperConfigure('us_stock', 'AAPL', 'rsi');
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/paper/config`, expect.objectContaining({ method: 'POST', body: JSON.stringify({ source: 'us_stock', symbol: 'AAPL', strategy: 'rsi' }) }));

    const practice = { concept_id: 'rsi', module: 'data' as const, symbol: 'BTCUSDT', source: 'binance' as const, limit: 50, inputs: { period: 14 } };
    await api.runPractice(practice);
    expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/practice`, expect.objectContaining({ method: 'POST', body: JSON.stringify(practice) }));
  });

  it('normalizes network and HTTP failures into useful errors', async () => {
    fetchMock.mockRejectedValueOnce(new Error('offline'));
    vi.stubGlobal('fetch', fetchMock);
    await expect(api.getConfig()).rejects.toThrow('Network error: offline');

    fetchMock.mockResolvedValueOnce(response('server details', { ok: false, status: 503 }));
    await expect(api.getConfig()).rejects.toThrow('HTTP 503: server details');

    fetchMock.mockResolvedValueOnce({ ok: false, status: 502, text: vi.fn().mockRejectedValue(new Error('body unavailable')) });
    await expect(api.getConfig()).rejects.toThrow('HTTP 502: ');
  });
});

describe('API display helpers', () => {
  it('formats nullish values and signed percentages', () => {
    expect(fmtPct(null)).toBe('—');
    expect(fmtPct(0.1234, 1)).toBe('12.3%');
    expect(fmtNum(undefined)).toBe('—');
    expect(fmtNum(1.234, 1)).toBe('1.2');
    expect(fmtMoney(null)).toBe('—');
    expect(fmtMoney(1234.5)).toMatch(/^\$1,234\.5?$/);
    expect(fmtPctSigned(0.125, 1)).toBe('+12.5%');
    expect(fmtPctSigned(-0.125, 1)).toBe('-12.5%');
  });

  it('prefers the most specific available source URL', () => {
    expect(preferredCodeLocationUrl({ path: 'src/a.ts', line: 1, github_url: 'https://github/a', url: 'https://url', source_url: 'https://source' })).toBe('https://github/a');
    expect(preferredCodeLocationUrl({ path: 'src/a.ts', line: 1, url: 'https://url', source_url: 'https://source' })).toBe('https://url');
    expect(preferredCodeLocationUrl({ path: 'src/a.ts', line: 1, source_url: 'https://source' })).toBe('https://source');
    expect(preferredCodeLocationUrl({ path: 'src/a.ts', line: 1 })).toBe('');
  });
});
