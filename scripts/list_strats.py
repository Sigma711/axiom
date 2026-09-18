import json
d = json.load(open('/tmp/strats.json'))
print(f"策略总数: {len(d['strategies'])}")
for s in d['strategies']:
    params = ", ".join([p['key'] for p in s.get('params', [])])
    print(f"  {s['name']:<22} {s['display_name']:<25} [{params}]")