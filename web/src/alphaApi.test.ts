import { afterEach, expect, it, vi } from 'vitest';
import { appBase } from './api';
import { alphaApi } from './alphaApi';

const fetchMock = vi.fn();

afterEach(() => {
  fetchMock.mockReset();
  vi.unstubAllGlobals();
});

it('loads the catalog and submits the selected research request', async () => {
  fetchMock.mockResolvedValue({ ok: true, json: vi.fn().mockResolvedValue({ collections: [] }) });
  vi.stubGlobal('fetch', fetchMock);

  await alphaApi.catalog();
  expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/alphas/catalog`, expect.objectContaining({ method: 'GET' }));

  const request = { collection: 'worldquant101' as const, id: 'alpha101', source: 'us_stock' as const, symbol: 'AAPL', limit: 500 };
  await alphaApi.research(request);
  expect(fetchMock).toHaveBeenLastCalledWith(`${appBase}/api/alphas/research`, expect.objectContaining({ method: 'POST', body: JSON.stringify(request) }));
});

it('reports transport and HTTP failures', async () => {
  fetchMock.mockRejectedValueOnce(new Error('offline'));
  vi.stubGlobal('fetch', fetchMock);
  await expect(alphaApi.catalog()).rejects.toThrow('Network error: offline');

  fetchMock.mockResolvedValueOnce({ ok: false, status: 422, text: vi.fn().mockResolvedValue('factor unavailable') });
  await expect(alphaApi.research({ collection: 'qlib_alpha158', id: 'vwap0', source: 'a_share', symbol: '600519', limit: 300 })).rejects.toThrow('HTTP 422: factor unavailable');
});
