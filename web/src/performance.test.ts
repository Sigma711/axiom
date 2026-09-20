import { describe, expect, it } from 'vitest';
import { performanceInputs } from './performance';
const point = (timestamp: string, equity: number) => ({timestamp, equity, cash: equity, position_value: 0});
describe('module performance context', () => {
  it('uses the actual hourly frequency and fractional elapsed days', () => {
    const data = performanceInputs([point('2024-01-01T00:00:00Z', 100), point('2024-01-01T01:00:00Z', 101), point('2024-01-01T02:00:00Z', 99)], 100);
    expect(data.elapsed_days).toBeCloseTo(1/12);
    expect(data.periods_per_year).toBe(8760);
    expect(data.risk_free_annual).toBe(0);
    expect(data.returns).toEqual([101/100-1, 99/101-1]);
    expect(data.equity).toEqual([100,101,99]);
  });
  it('includes initial trading costs in the drawdown and total-return path', () => {
    const data = performanceInputs([point('2024-01-01T00:00:00Z', 98), point('2024-01-02T00:00:00Z', 99)], 100);
    expect(data.equity).toEqual([100,98,99]);
    expect(data.periods_per_year).toBe(365);
  });
  it('does not override teaching inputs with absent or malformed snapshots', () => {
    expect(performanceInputs(undefined, 100)).toEqual({});
    expect(performanceInputs([], 100)).toEqual({});
    expect(performanceInputs([point('invalid', NaN)], 100)).toEqual({});
  });
});
