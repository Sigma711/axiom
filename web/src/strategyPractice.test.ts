import { describe, expect, it } from 'vitest';
import { STRATEGY_EXERCISES, calculatePractice, validatePracticeValues } from './strategyPractice';

const cases: readonly [string, number[]][] = [
  ['trend', [1, 5.46037735849056]],
  ['momentum', [1, -5]],
  ['reversion', [-2, 5.55555555555556]],
  ['factor', [1, 2]],
  ['stat-arb', [20, 3]],
  ['event', [10, 1.61818181818182]],
  ['arbitrage', [-0.1, -1]],
  ['market-making', [1.6, -5.2]],
  ['volatility', [2000, 1100]],
  ['carry', [2, -3]],
  ['allocation', [12, 64.2857142857143, -4.2857142857143]],
];

describe('11 strategy teaching calculations', () => {
  it('has one distinct practice for every family with finite defaults and meaningful outputs', () => {
    expect(STRATEGY_EXERCISES.map(item => item.id)).toEqual(cases.map(([id]) => id));
    for (const [id, expected] of cases) {
      const exercise = STRATEGY_EXERCISES.find(item => item.id === id)!;
      const values = Object.fromEntries(exercise.fields.map(item => [item.key, item.initial]));
      expect(validatePracticeValues(exercise, values)).toBeNull();
      const result = calculatePractice(exercise, values);
      expect(result.metrics.map(item => item.value)).toHaveLength(expected.length);
      result.metrics.forEach((metric, index) => {
        expect(metric.value).toBeCloseTo(expected[index], 7);
        expect(metric.meaning.length).toBeGreaterThan(12);
      });
      expect(result.reading.length).toBeGreaterThan(12);
    }
  });

  it('rejects missing, nonfinite and out-of-range inputs before computing', () => {
    for (const exercise of STRATEGY_EXERCISES) {
      const values = Object.fromEntries(exercise.fields.map(item => [item.key, item.initial]));
      const first = exercise.fields[0];
      for (const invalid of [Number.NaN, Number.POSITIVE_INFINITY, first.min - 1, first.max + 1]) {
        const broken = { ...values, [first.key]: invalid };
        expect(validatePracticeValues(exercise, broken)).toContain(first.label);
        expect(() => calculatePractice(exercise, broken)).toThrow();
      }
    }
  });

  it('never lets future momentum or factor returns change the prior selection', () => {
    for (const id of ['momentum', 'factor']) {
      const exercise = STRATEGY_EXERCISES.find(item => item.id === id)!;
      const values = Object.fromEntries(exercise.fields.map(item => [item.key, item.initial]));
      const winner = calculatePractice(exercise, values).metrics[0].value;
      const future = exercise.fields.slice(-2);
      expect(calculatePractice(exercise, { ...values, [future[0].key]: -50, [future[1].key]: 50 }).metrics[0].value).toBe(winner);
    }
  });

  it('calculates maker adverse selection, straddle expiry loss and weight drift at boundaries', () => {
    const find = (id: string) => STRATEGY_EXERCISES.find(item => item.id === id)!;
    const defaults = (id: string) => Object.fromEntries(find(id).fields.map(item => [item.key, item.initial]));
    expect(calculatePractice(find('market-making'), { ...defaults('market-making'), later: 98 }).metrics[1].value).toBeCloseTo(-19.2);
    expect(calculatePractice(find('volatility'), { ...defaults('volatility'), settlement: 100 }).metrics.map(item => item.value)).toEqual([0, -900]);
    expect(calculatePractice(find('allocation'), { ...defaults('allocation'), returnA: 0, returnB: 0 }).metrics.map(item => item.value)).toEqual([0, 60, 0]);
  });
});
