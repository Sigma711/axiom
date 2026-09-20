import type { EquityPoint } from './types';
/** Keep the same observation frequency and risk-free convention as Rust metrics. */
export function performanceInputs(points: EquityPoint[] | undefined, initialCapital?: number): Record<string, unknown> {
  if (!points?.length || points.some(point => !Number.isFinite(point.equity) || !Number.isFinite(Date.parse(point.timestamp)))) return {};
  const observed = points.map(point => point.equity);
  const initial = initialCapital != null && Number.isFinite(initialCapital) ? initialCapital : observed[0];
  const equity = initial === observed[0] ? observed : [initial, ...observed];
  const returns = observed.slice(1).map((value, index) => observed[index] > 0 ? value / observed[index] - 1 : NaN);
  const elapsedDays = (Date.parse(points[points.length - 1].timestamp) - Date.parse(points[0].timestamp)) / 86_400_000;
  return {
    equity, returns, initial_capital: initial, elapsed_days: elapsedDays,
    periods_per_year: elapsedDays > 0 ? (points.length - 1) * 365 / elapsedDays : 0,
    risk_free_annual: 0,
  };
}
