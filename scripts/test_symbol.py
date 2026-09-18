import sys, json
d = json.load(open(sys.argv[1]))
m = d['metrics']
print(f"  交易对: {d['config']['symbol']}")
print(f"  K线数: {d['config']['n_bars']}")
print(f"  总收益: {m['总收益率']*100:.2f}%")
print(f"  交易笔数: {m['交易笔数']}")