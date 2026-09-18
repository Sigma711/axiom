import json, urllib.request
d = json.loads(urllib.request.urlopen('http://localhost:8080/api/knowledge').read())
print(f'总: {d["total"]}')
for cat, items in d['categories'].items():
    for it in items:
        if it['id'] == 'macd':
            print()
            print('MACD 详情:')
            for k, v in it.items():
                v_str = str(v)[:90] if isinstance(v, str) else str(v)[:90]
                print(f'  {k}: {v_str}')
            break