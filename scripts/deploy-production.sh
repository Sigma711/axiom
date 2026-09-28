#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
test "$(git branch --show-current)" = main
git fetch --quiet origin main
test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
test -z "$(git status --porcelain)"
make ci
make verify-published

revision="$(git rev-parse --short=12 HEAD)"
release="/srv/axiom/releases/$revision"
bundle="target/deploy-web"
VITE_BASE=/axiom/ VITE_APP_BASE=/axiom VITE_OUT_DIR=../target/deploy-web make build-web
make build-rust
mkdir -p target/deploy-release/static
rsync -a --delete static/ target/deploy-release/static/
rsync -a "$bundle/" target/deploy-release/static/
# Existing live directories survive release switches. On a first install,
# seed them from the locally verified runtime cache before starting the service.
ssh root@sigma711.top "install -d -o axiom -g axiom -m 755 /srv/axiom/data/symbols"
for source in a_share us_stock; do
    if ! ssh root@sigma711.top "test -s /srv/axiom/data/symbols/$source.json"; then
        test -s "data/symbols/$source.json"
        rsync -az "data/symbols/$source.json" "root@sigma711.top:/srv/axiom/data/symbols/.$source.deploy.tmp"
        ssh root@sigma711.top "chown axiom:axiom /srv/axiom/data/symbols/.$source.deploy.tmp && mv -f /srv/axiom/data/symbols/.$source.deploy.tmp /srv/axiom/data/symbols/$source.json"
    fi
done
ssh root@sigma711.top "mkdir -p '$release/static'"
rsync -az target/release/axiom "root@sigma711.top:$release/axiom"
rsync -az --delete target/deploy-release/static/ "root@sigma711.top:$release/static/"
previous="$(ssh root@sigma711.top "readlink -f /srv/axiom/current")"
test -n "$previous"
activated=0
rollback() {
    rc=$?
    trap - ERR
    if [[ "$activated" = 1 ]]; then
        printf 'Deployment verification failed; restoring %s\n' "$previous" >&2
        ssh root@sigma711.top "ln -sfn '$previous' /srv/axiom/current.next && mv -Tf /srv/axiom/current.next /srv/axiom/current && systemctl restart axiom" || true
    fi
    exit "$rc"
}
trap rollback ERR
activated=1
ssh root@sigma711.top "chmod 755 '$release/axiom' && ln -sfn '$release' /srv/axiom/current.next && mv -Tf /srv/axiom/current.next /srv/axiom/current && systemctl restart axiom && systemctl is-active --quiet axiom"

python3 - <<'PYVERIFY'
import json
import math
import subprocess
from datetime import datetime
from pathlib import Path
import time
import urllib.parse
import urllib.request

base = "https://sigma711.top/axiom"
def read_json(path, params):
    query = urllib.parse.urlencode(params)
    with urllib.request.urlopen(base + path + "?" + query, timeout=30) as response:
        assert response.status == 200
        return json.load(response)

with urllib.request.urlopen(base + "/", timeout=20) as response:
    html = response.read().decode()
    assert response.status == 200 and "/axiom/assets/" in html

with urllib.request.urlopen(base + "/api/book/pdf", timeout=30) as response:
    assert response.status == 200 and response.read(5) == b"%PDF-"
workers = list(Path("target/deploy-web/assets").glob("pdf.worker.min-*.mjs"))
assert len(workers) == 1
with urllib.request.urlopen(base + "/assets/" + workers[0].name, timeout=30) as response:
    assert response.status == 200 and "javascript" in response.headers["Content-Type"]
print("book PDF and module worker", "ok", flush=True)

filing_request = urllib.request.Request(base + "/api/practice", data=json.dumps({
    "concept_id": "book_fcf", "module": "data", "source": "us_stock", "symbol": "AAPL", "inputs": {}
}).encode(), headers={"Content-Type": "application/json"}, method="POST")
with urllib.request.urlopen(filing_request, timeout=90) as response:
    filing = json.load(response)
assert filing["provenance"] == "verified_issuer_filing_case"
assert filing["values"]["free_cash_flow"] == 98767
assert filing["filing_case"]["sha256"] == "43e7f0730b3cce0fc37301a2f43c29712bbde6ab299d97c6df345fd0c754508a"
assert filing["filing_case"]["verification"]["matched_bytes"] == 4919649
print("Apple official historical filing", "verified", flush=True)

# Verify each issuer can actually be retrieved by the production host. These
# expected values are transcribed from the original disclosure pages.
industry_cases = (
    ("bank_nim", "2318.HK", "net_interest_margin", 93427 / 4994494, "62a5bd793ef9a787cc95750d65e52803aa58fa424b01d94e754ea0120d5be8a3", 14886158),
    ("book_saas_arr", "SHOP", "annualized_recurring_revenue_run_rate", 2136, "4bf71232697a2270b2dbc38fc9609c11c27d545d6f4301fce3356fa60c6ef6de", 86468),
    ("book_platform_take_rate", "EBAY", "platform_take_rate", 10283 / 74667, "10530b8314c4dc49f9737b938f28ead7a70212885c35919fb361d145401f37fb", 1004020),
    ("book_reit_occupancy", "O", "occupied_area_ratio", 335777818 / 339361416, "a0b3bf067c7b19ebde01ceaac3ecb172ed6a4c7084eeabe276ad1d4599c62a3f", 17522920),
)
for concept, symbol, key, expected, fingerprint, size in industry_cases:
    body = {"concept_id": concept, "module": "data", "source": "issuer_disclosure", "symbol": symbol, "inputs": {}}
    req = urllib.request.Request(base + "/api/practice", data=json.dumps(body).encode(), headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=90) as response:
        result = json.load(response)
    assert result["bar_origin"] == "server_verified_issuer_filing_pdf"
    assert result["industry_case"]["issuer"]["ticker"] == symbol
    assert result["industry_case"]["sha256"] == fingerprint
    assert result["industry_case"]["verification"]["matched_sha256"] == fingerprint
    assert result["industry_case"]["verification"]["matched_bytes"] == size
    assert math.isclose(result["values"][key], expected, rel_tol=1e-12, abs_tol=1e-12)
    print(symbol, "official historical disclosure", "verified", flush=True)

markets = (("binance", "BTCUSDT", 200), ("a_share", "600519", 1000), ("us_stock", "AAPL", 1000))
for source, symbol, minimum in markets:
    snapshot = read_json("/api/data", {"source": source, "symbol": symbol, "limit": 5})
    provenance = snapshot["market_provenance"]
    allowed = {"binance": {"binance_spot"}, "a_share": {"eastmoney", "tencent"}, "us_stock": {"yahoo", "nasdaq"}}
    bars = snapshot["bars"]
    if source == "binance" and provenance["provider"] == "local_csv_cache":
        # A warm cache is not connectivity evidence. Pull official candles on
        # the production host and independently compare the returned OHLCV.
        probe = '''import json, time, urllib.request
errors = []
for host in ("https://data-api.binance.vision", "https://api.binance.com"):
    try:
        with urllib.request.urlopen(host + "/api/v3/klines?symbol=BTCUSDT&interval=1h&limit=10", timeout=30) as response:
            rows = json.load(response)
        print(json.dumps([row for row in rows if row[6] < time.time() * 1000]))
        break
    except Exception as error:
        errors.append(str(error))
else:
    raise RuntimeError("Official Binance candle pull failed: " + "; ".join(errors))
'''
        rows = json.loads(subprocess.check_output(["ssh", "root@sigma711.top", "python3", "-"], input=probe.encode(), timeout=90))
        by_time = {row[0]: row for row in rows}
        for bar in bars:
            timestamp = round(datetime.fromisoformat(bar["timestamp"].replace("Z", "+00:00")).timestamp() * 1000)
            raw = by_time[timestamp]
            for field, index in (("open", 1), ("high", 2), ("low", 3), ("close", 4), ("volume", 5)):
                assert math.isclose(bar[field], float(raw[index]), rel_tol=1e-10, abs_tol=1e-10), (field, bar, raw)
        print("Binance production-host official pull and cached OHLCV match", flush=True)
    else:
        assert provenance["provider"] in allowed[source], (source, provenance)
        assert provenance["endpoint"].startswith("https://"), (source, provenance)
    assert len(bars) == 5, (source, bars)
    assert all(bar["low"] <= min(bar["open"], bar["close"]) <= max(bar["open"], bar["close"]) <= bar["high"] for bar in bars)
    print(source, symbol, bars[-1]["timestamp"], flush=True)

for source, symbol, minimum in markets:
    deadline = time.monotonic() + 120
    while True:
        catalog = read_json("/api/symbols", {"source": source, "limit": 5})
        if catalog["complete"] and catalog["status"] in ("cached", "stale") and catalog["universe_count"] >= minimum:
            break
        if time.monotonic() >= deadline:
            raise AssertionError(f"{source} catalog incomplete: {catalog}")
        time.sleep(3)
    match = read_json("/api/symbols", {"source": source, "q": symbol, "limit": 5})
    assert any(item["symbol"] == symbol for item in match["items"]), (source, symbol, match)
    print(source, "catalog", catalog["universe_count"], catalog["status"], flush=True)
PYVERIFY
trap - ERR
