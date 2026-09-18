import json, sys
d = json.load(open('/tmp/kb.json'))
print(f"📚 总条目: {d['total']}")
print(f"📂 总分类: {len(d['categories'])}")
print()
print("完整分类列表:")
for cat, items in d['categories'].items():
    print(f"  • {cat:<20} {len(items):>3} 条")