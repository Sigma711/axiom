import { describe, expect, it } from 'vitest';
import { industryCaseRequest } from './industryCase';
import type { PracticeConcept } from './types';

describe('industryCaseRequest', () => {
  it.each([
    ['bank_nim', '2318.HK'],
    ['book_bank_nim', '2318.HK'],
    ['book_bank_cost_income', '2318.HK'],
    ['book_bank_npl_ratio', '2318.HK'],
    ['book_insurance_solvency_ratio', '2318.HK'],
    ['book_saas_arr', 'SHOP'],
    ['book_saas_rule_of_40', 'SHOP'],
    ['book_platform_gmv', 'SHOP'],
    ['book_platform_take_rate', 'EBAY'],
    ['book_reit_occupancy', 'O'],
  ])('pins %s to its historical issuer rather than the selected market symbol', (conceptId, symbol) => {
    const concept = { id: conceptId, input_kind: 'industry_case', plan: { fixed_source: 'issuer_disclosure', fixed_symbol: symbol } } as PracticeConcept;
    expect(industryCaseRequest(concept)).toEqual({ concept_id: conceptId, module: 'data', source: 'issuer_disclosure', symbol, inputs: {} });
  });

  it('fails closed when the catalog omits the fixed issuer contract', () => {
    expect(() => industryCaseRequest({ id: 'book_fcf', input_kind: 'independent_inputs' } as PracticeConcept)).toThrow('缺少固定发行人');
  });
});
