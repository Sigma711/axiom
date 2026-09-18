import json
d = json.load(open('/tmp/s.json'))
print(f"总交易对: {d['count']}")
print(f"数据源: {d['source']}")
print(f"Top 15: {d['symbols'][:15]}")