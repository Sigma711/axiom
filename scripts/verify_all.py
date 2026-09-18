"""验证所有新策略和指标"""
import json, subprocess, os

os.chdir('/home/sigma711/projects/quant-demo-rs')

# 1. 指标 API
subprocess.run(['curl', '-s',
    'http://localhost:8080/api/indicators?symbol=BTCUSDT&limit=10&indicators=sma_20,rsi_14,macd,bbands_20,vwap,atr_14',
    '-o', '/tmp/i.json'], check=True)
d = json.load(open('/tmp/i.json'))
print(f'✅ 指标 API: K线={len(d["bars"])}, 返回 {len(d["indicators"])} 组指标')
print('   ', ', '.join(d['indicators'].keys()))

# 2. 各策略快测
strategies = ['macd', 'bollinger', 'supertrend', 'donchian_breakout', 'vwap_reversion', 'kdj']
for s in strategies:
    subprocess.run([
        'curl', '-s', '-X', 'POST', 'http://localhost:8080/api/backtest',
        '-H', 'Content-Type: application/json',
        '-d', json.dumps({
            'strategy': s, 'source': 'real', 'limit': 200,
            'initial_capital': 10000
        }),
        '-o', f'/tmp/{s}.json'
    ], check=True)

print('\n新策略回测结果(BTC 200 根 K 线, 真实数据):')
for s in strategies:
    d = json.load(open(f'/tmp/{s}.json'))
    m = d['metrics']
    ret = m['总收益率'] * 100
    trades = m['交易笔数']
    sharpe = m['夏普比率']
    sign = '+' if ret >= 0 else ''
    print(f'  {s:<22} {trades:>3} 笔  收益 {sign}{ret:>6.2f}%  夏普 {sharpe:>6.2f}')