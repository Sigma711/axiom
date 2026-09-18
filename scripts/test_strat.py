import json, sys
strategy = sys.argv[1] if len(sys.argv) > 1 else 'macd'
d = json.load(open(f'/tmp/{strategy[0]}.json'))
m = d['metrics']
print(f"策略: {d['config']['strategy']}")
print(f"交易: {m['交易笔数']} 笔, 胜率 {m['胜率']*100:.1f}%")
print(f"总收益: {m['总收益率']*100:.2f}%, 夏普 {m['夏普比率']:.2f}")