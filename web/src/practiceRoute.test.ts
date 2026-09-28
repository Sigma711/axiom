import { describe, expect, it } from 'vitest';
import { marketSourceFromPracticeRoute, practiceRouteSource } from './practiceRoute';
import type { PracticeConcept } from './types';

type Plan = NonNullable<PracticeConcept['plan']>;
const plan = (market: Plan['markets'][number]) => ({ markets: [market] } as Plan);

describe('practice route source', () => {
  it('preserves issuer disclosure in the knowledge-card route', () => {
    expect(practiceRouteSource(plan('issuer_disclosure'))).toBe('issuer_disclosure');
    expect(marketSourceFromPracticeRoute('issuer_disclosure')).toBeUndefined();
  });

  it.each([
    ['crypto', 'binance'],
    ['cn_equity', 'a_share'],
    ['us_equity', 'us_stock'],
  ] as const)('maps %s to %s', (market, source) => {
    expect(practiceRouteSource(plan(market))).toBe(source);
    expect(marketSourceFromPracticeRoute(source)).toBe(source);
  });
});
