import json, urllib.request
d = json.loads(urllib.request.urlopen('http://localhost:8080/api/knowledge').read())
print(f'共 {d["total"]} 条')
# 看 MACD
for cat, items in d['categories'].items():
    for it in items:
        if it['id'] == 'macd':
            print()
            print('MACD entry:')
            for k, v in it.items():
                if isinstance(v, str) and len(v) > 100:
                    print(f'  {k}: {v[:100]}...')
                else:
                    print(f'  {k}: {v}')
            break