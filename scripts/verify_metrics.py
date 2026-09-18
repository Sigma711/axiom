import json
with open('/tmp/r.json') as f:
    d = json.load(f)
m = d['metrics']
print('新指标验证:')
for k in ['索提诺比率', 'Calmar比率', 'VaR_95', 'CVaR_95', '偏度', '峰度', '夏普比率', '总收益率']:
    v = m.get(k, 'N/A')
    if isinstance(v, float):
        if abs(v) < 1:
            print(f'  {k}: {v:.4f}')
        else:
            print(f'  {k}: {v:.2f}')
    else:
        print(f'  {k}: {v}')