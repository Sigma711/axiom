import { appPath } from './api';
import type { AlphaCatalog, AlphaResearchRequest, AlphaResearchResult } from './types';

async function request<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<T> {
  let response: Response;
  try {
    response = await fetch(appPath(path), {
      method,
      headers: { 'Content-Type': 'application/json' },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
  } catch (error) {
    throw new Error(`Network error: ${(error as Error).message}`);
  }
  if (!response.ok) throw new Error(`HTTP ${response.status}: ${(await response.text()).slice(0, 300)}`);
  return response.json() as Promise<T>;
}

export const alphaApi = {
  catalog: () => request<AlphaCatalog>('GET', '/api/alphas/catalog'),
  research: (input: AlphaResearchRequest) => request<AlphaResearchResult>('POST', '/api/alphas/research', input),
};
