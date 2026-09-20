export type IndicatorPanel = 'price' | 'oscillator' | 'momentum' | 'volume' | 'volatility';

const INDICATOR_PANELS: Array<[RegExp, IndicatorPanel]> = [
  [/^(sma|ema|wma|vwma|vwap|bbands|bollinger|donchian|keltner|psar|supertrend|ichimoku)/i, 'price'],
  [/^(rsi|stoch|kdj|williams|cci|mfi|fisher)/i, 'oscillator'],
  [/^(macd|ppo|roc|mom|adx|dmi|trix|tsi|ao)/i, 'momentum'],
  [/^(obv|cmf|adl|volume)/i, 'volume'],
  [/^(atr|natr|stddev|variance|volatility)/i, 'volatility'],
];

export function indicatorPanel(name: string): IndicatorPanel {
  return INDICATOR_PANELS.find(([pattern]) => pattern.test(name))?.[1] ?? 'momentum';
}

export function validSeries(values: Array<{ x: string; y: number } | null>) {
  return values.map(value => {
    if (!value || !Number.isFinite(value.y) || Number.isNaN(Date.parse(value.x))) return null;
    return value;
  });
}
