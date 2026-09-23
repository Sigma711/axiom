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
    expect(data.equity_points).toEqual([
      { timestamp: '2024-01-01T00:00:00Z', equity: 100 },
      { timestamp: '2024-01-01T01:00:00Z', equity: 101 },
      { timestamp: '2024-01-01T02:00:00Z', equity: 99 },
    ]);
  });
  it('includes initial trading costs in the drawdown and total-return path', () => {
    const data = performanceInputs([point('2024-01-01T00:00:00Z', 98), point('2024-01-02T00:00:00Z', 99)], 100);
    expect(data.equity).toEqual([100,98,99]);
    expect(data.periods_per_year).toBe(365);
  });

  it('uses the same completed bars as a timestamp-aligned buy-and-hold benchmark', () => {
    const points = [point('2024-01-01T00:00:00Z', 100), point('2024-01-02T00:00:00Z', 105), point('2024-01-03T00:00:00Z', 103)];
    const bars = [100, 110, 99].map((close, index) => ({
      timestamp: points[index].timestamp, open: close, high: close, low: close, close, volume: 1,
    }));
    const data = performanceInputs(points, 100, bars);
    expect((data.strategy_returns as number[])[0]).toBeCloseTo(0.05);
    expect((data.strategy_returns as number[])[1]).toBeCloseTo(103 / 105 - 1);
    expect((data.benchmark_returns as number[])[0]).toBeCloseTo(0.1);
    expect((data.benchmark_returns as number[])[1]).toBeCloseTo(-0.1);
  });
  it('does not invent a benchmark when bars and equity observations do not align', () => {
    const points = [point('2024-01-01T00:00:00Z', 100), point('2024-01-02T00:00:00Z', 101)];
    const bars = [{ timestamp: points[0].timestamp, open: 100, high: 100, low: 100, close: 100, volume: 1 }];
    expect(performanceInputs(points, 100, bars)).not.toHaveProperty('benchmark_returns');
    expect(performanceInputs(points, 100, [{ ...bars[0], timestamp: points[1].timestamp, close: 0 }])).not.toHaveProperty('benchmark_returns');
  });
  it('does not override teaching inputs with absent or malformed snapshots', () => {
    expect(performanceInputs(undefined, 100)).toEqual({});
    expect(performanceInputs([], 100)).toEqual({});
    expect(performanceInputs([point('invalid', NaN)], 100)).toEqual({});
  });
});
