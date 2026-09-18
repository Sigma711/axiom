"""模拟浏览器行为,看是否有 CORS 或其他问题"""
import urllib.request
import urllib.error
import json

URLS = [
    "http://localhost:8080/",
    "http://localhost:8080/api/indicators?symbol=BTCUSDT&limit=50&indicators=sma_20,rsi_14,bbands_20",
    "http://localhost:8080/api/symbols",
]

for url in URLS:
    try:
        req = urllib.request.Request(url, headers={
            'Accept': 'application/json',
            'Origin': 'http://localhost:8080',
            'Referer': 'http://localhost:8080/',
        })
        with urllib.request.urlopen(req, timeout=10) as resp:
            print(f"✅ {url[:80]}")
            print(f"   status={resp.status}, len={len(resp.read())}")
    except urllib.error.HTTPError as e:
        print(f"❌ HTTP {e.code}: {url[:80]}")
        print(f"   body: {e.read()[:200]}")
    except Exception as e:
        print(f"❌ {type(e).__name__}: {url[:80]}")
        print(f"   msg: {e}")