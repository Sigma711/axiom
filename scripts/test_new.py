import json
import subprocess
import os

os.chdir('/home/sigma711/projects/quant-demo-rs')

for s in ['ichimoku', 'ppo', 'vortex', 'elder_ray']:
    subprocess.run([
        'curl', '-s', '-X', 'POST', 'http://localhost:8080/api/backtest',
        '-H', 'Content-Type: application/json',
        '-d', json.dumps({
            'strategy': s, 'source': 'real', 'limit': 100,
            'initial_capital': 10000
        }),
        '-o', f'/tmp/{s}.json'
    ], check=True)
    try:
        d = json.load(open(f'/tmp/{s}.json'))
        m = d['metrics']
        print(f"  {s}: 交易 {m['交易笔数']} 笔, 收益 {m['总收益率']*100:+.2f}%, 夏普 {m['夏普比率']:.2f}")
    except Exception as e:
        print(f"  {s}: 失败 - {e}")

print()
print('=== Ichimoku 指标 ===')
subprocess.run([
    'curl', '-s',
    'http://localhost:8080/api/indicators?symbol=BTCUSDT&limit=20&indicators=ichimoku',
    '-o', '/tmp/ind.json'
], check=True)
d = json.load(open('/tmp/ind.json'))
print(f"bars: {len(d['bars'])}")
print(f"indicators: {list(d['indicators'].keys())}")
for k, v in d['indicators'].items():
    valid = sum(1 for x in v if x is not None)
    print(f"  {k}: {valid} 有效值")

print()
print('=== 全部策略列表 ===')
import urllib.request
d = json.loads(urllib.request.urlopen('http://localhost:8080/api/strategies').read())
print(f"共 {len(d['strategies'])} 个策略:")
for s in d['strategies']:
    print(f"  - {s['name']:<22} {s['display_name']}")