#!/usr/bin/env python3
"""测试 SMA 交叉策略在真实数据上的表现"""
import json
import subprocess
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
            'strategy': 'sma_cross',
            'params': {'fast': 5, 'slow': 20},
            'source': 'real',
            'limit': 500,
            'initial_capital': 10000,
        }).encode('utf-8'),
        headers={'Content-Type': 'application/json'},
    )
    resp = urllib.request.urlopen(req, timeout=30)
    d = json.loads(resp.read())
    print('✅ SMA 策略回测成功 (真实 BTC 数据, 500 根 K 线)')
    m = d['metrics']
    print(f"  策略参数: {d['config']['params']}")
    print(f"  总收益: {m['总收益率']*100:.2f}%")
    print(f"  年化收益: {m['年化收益率']*100:.2f}%")
    print(f"  最大回撤: {m['最大回撤_pct']*100:.2f}%")
    print(f"  夏普比率: {m['夏普比率']:.2f}")
    print(f"  交易笔数: {m['交易笔数']}")
    print(f"  胜率: {m['胜率']*100:.1f}%")
    print(f"  盈亏比: {m['盈亏比']:.2f}")
    print(f"  最终净值: ${m['最终净值']:.2f}")
except Exception as e:
    print(f'❌ 失败: {e}')
finally:
    proc.terminate()
    proc.wait(timeout=5)