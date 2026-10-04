import type { PracticeConcept, PracticeRouteSource, SourceType } from './types';

export function practiceRouteSource(plan?: PracticeConcept['plan']): PracticeRouteSource {
  if (plan?.fixed_source === 'market_breadth') return 'market_breadth';
  const market = plan?.markets[0];
  if (market === 'market_breadth') return 'market_breadth';
  if (market === 'issuer_disclosure') return 'issuer_disclosure';
  if (market === 'cn_equity') return 'a_share';
  if (market === 'us_equity') return 'us_stock';
  return 'binance';
}

export function marketSourceFromPracticeRoute(source?: PracticeRouteSource): SourceType | undefined {
  return source === 'issuer_disclosure' || source === 'market_breadth' ? undefined : source;
}
