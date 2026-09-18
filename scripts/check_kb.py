import json, sys, urllib.request
d = json.loads(urllib.request.urlopen('http://localhost:8080/api/knowledge').read())
print(f"知识库 API: 总条目={d['total']}, 分类={len(d['categories'])}")