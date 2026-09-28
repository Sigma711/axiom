import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { ExecutionAssumptionsVisual } from './ExecutionAssumptionsVisual';

describe('ExecutionAssumptionsVisual', () => {
  it('keeps missing execution metadata unknown rather than claiming exchange-compliant execution', () => {
    expect(renderToStaticMarkup(<ExecutionAssumptionsVisual />)).toBe('');
  });
  it('separates enforced inventory rules from omitted trading constraints and cites the original rules', () => {
    const html = renderToStaticMarkup(<ExecutionAssumptionsVisual assumptions={[
      { id: 't_plus_one', description_zh: '当日新增普通股票不可卖出。', simulated: true, limitation_zh: '早先库存仍可卖。', source_url: 'https://www.sse.com.cn/rules' },
      { id: 'halts', description_zh: '未模拟涨跌停和停牌。', simulated: false, limitation_zh: null, source_url: null },
    ]} provenance={{ provider: 'tencent', endpoint: 'https://web.ifzq.gtimg.cn/kline', price_basis: 'unadjusted_requested', corporate_actions: 'not_simulated' }} />);
    expect(html).toContain('已模拟');
    expect(html).toContain('未模拟');
    expect(html).toContain('当日新增普通股票不可卖出');
    expect(html).toContain('早先库存仍可卖');
    expect(html).toContain('https://www.sse.com.cn/rules');
    expect(html).toContain('腾讯行情');
    expect(html).toContain('不复权请求');
    expect(html).toContain('分红、拆股');
  });
  it('labels cached, caller-provided and provider-adjustment uncertainty without disguising them as total return', () => {
    for (const [provider, price_basis, expected] of [
      ['local_csv_cache', 'cache_price_basis_unverified', '历史 CSV 缓存'],
      ['caller_provided_unverified', 'caller_provided_unverified', '调用方提供'],
      ['yahoo', 'provider_adjustment_unverified', '复权口径未核验'],
      ['nasdaq', 'provider_adjustment_unverified', 'Nasdaq'],
      ['binance_spot', 'spot_trade_prices', '现货成交价'],
      ['new_provider', 'new_basis', 'new_basis'],
    ]) {
      const html = renderToStaticMarkup(<ExecutionAssumptionsVisual provenance={{ provider, endpoint: 'local historical CSV cache', price_basis, corporate_actions: 'not_simulated' }} />);
      expect(html).toContain(expected);
      expect(html).not.toContain('href="local');
      expect(html).toContain('不代表含分红再投资的总回报');
    }
  });
});
