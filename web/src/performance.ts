import type { Bar, EquityPoint } from './types';
/** Keep the same observation frequency and risk-free convention as Rust metrics. */
export function performanceInputs(points: EquityPoint[] | undefined, initialCapital?: number, bars?: Bar[]): Record<string, unknown> {
  if (!points?.length || points.some(point => !Number.isFinite(point.equity) || !Number.isFinite(Date.parse(point.timestamp)))) return {};
  const observed = points.map(point => point.equity);
  const initial = initialCapital != null && Number.isFinite(initialCapital) ? initialCapital : observed[0];
  const equity = initial === observed[0] ? observed : [initial, ...observed];
  const returns = observed.slice(1).map((value, index) => observed[index] > 0 ? value / observed[index] - 1 : NaN);
  const elapsedDays = (Date.parse(points[points.length - 1].timestamp) - Date.parse(points[0].timestamp)) / 86_400_000;
  const context: Record<string, unknown> = {
    equity, returns, strategy_returns: returns, initial_capital: initial, elapsed_days: elapsedDays,
    periods_per_year: elapsedDays > 0 ? (points.length - 1) * 365 / elapsedDays : 0,
    risk_free_annual: 0,
  };
  if (bars && points.length > 1) {
    const closeByTime = new Map(bars.filter(bar => Number.isFinite(bar.close) && bar.close > 0).map(bar => [Date.parse(bar.timestamp), bar.close]));
    const closes = points.map(point => closeByTime.get(Date.parse(point.timestamp)));
    if (closes.every((close): close is number => close != null && Number.isFinite(close) && close > 0)) {
      context.benchmark_returns = closes.slice(1).map((close, index) => close / closes[index]! - 1);
    }
  }
  return context;
}
