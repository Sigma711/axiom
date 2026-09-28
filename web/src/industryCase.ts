import type { PracticeConcept } from './types';

export function industryCaseRequest(concept: Pick<PracticeConcept, 'id' | 'input_kind' | 'plan'>) {
  const source = concept.plan?.fixed_source;
  const symbol = concept.plan?.fixed_symbol;
  if (concept.input_kind !== 'industry_case' || source !== 'issuer_disclosure' || !symbol) {
    throw new Error(`${concept.id} 缺少固定发行人历史披露契约`);
  }
  return {
    concept_id: concept.id,
    module: 'data' as const,
    source,
    symbol,
    inputs: {},
  };
}
