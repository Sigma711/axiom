import json, sys, urllib.request
req = {
    'strategy': 'donchian_breakout',
    'params': {'entry_period': 20, 'exit_period': 10},
    'source': 'real', 'limit': 500, 'initial_capital': 10000,
}
req_bytes = json.dumps(req).encode('utf-8')
r = urllib.request.Request('http://localhost:8080/api/backtest',
    data=req_bytes, headers={'Content-Type': 'application/json'})
d = json.loads(urllib.request.urlopen(r).read())
m = d['metrics']
print(f"交易: {m['交易笔数']}, 最终净值: {m['最终净值']}")
print(f"信号总数: {len(d['signals'])}")
buys = [s for s in d['signals'] if s['side'] == 'BUY']
sells = [s for s in d['signals'] if s['side'] == 'SELL']
holds = [s for s in d['signals'] if s['side'] == 'HOLD']
print(f"BUY: {len(buys)}, SELL: {len(sells)}, HOLD: {len(holds)}")
print(f"equity_curve 长度: {len(d['equity_curve'])}")