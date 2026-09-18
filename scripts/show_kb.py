import json
d = json.load(open('/tmp/kb.json'))
print(f"📚 总条目: {d['total']}")
print(f"📂 分类: {len(d['categories'])}")
print()
print("各分类条目数:")
for cat, items in d['categories'].items():
    print(f"  • {cat:<14} {len(items):>3} 条")