#!/usr/bin/env python3
import json
import subprocess
import sys
import time
import os

os.chdir('/home/sigma711/projects/quant-demo-rs')
proc = subprocess.Popen(
    ['./target/debug/quant_demo'],
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)
time.sleep(2)
try:
    import urllib.request
    req = urllib.request.Request(
        'http://localhost:8080/api/backtest',
        data=json.dumps({
            'strategy': 'buy_and_hold',
            'source': 'real',
            'limit': 200,
            'initial_capital': 10000,
        }).encode('utf-8'),
        headers={'Content-Type': 'application/json'},
    )
    resp = urllib.request.urlopen(req, timeout=15)
    d = json.loads(resp.read())
    print('✅ 真实 BTC 数据回测成功')
    m = d['metrics']
    print(f"  K线数: {d['config']['n_bars']}")
    print(f"  策略: {d['config']['strategy']}")
    print(f"  初始资金: ${m['初始资金']:.2f}")
    print(f"  最终净值: ${m['最终净值']:.2f}")
    print(f"  总收益: {m['总收益率']*100:.2f}%")
    print(f"  最大回撤: {m['最大回撤_pct']*100:.2f}%")
    print(f"  交易笔数: {m['交易笔数']}")
except Exception as e:
    print(f'❌ 失败: {e}')
finally:
    proc.terminate()
    proc.wait(timeout=5)