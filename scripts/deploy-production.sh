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
import csv
import subprocess
from datetime import datetime, timezone
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

def post_json(path, payload, timeout=180):
    request = urllib.request.Request(
        base + path,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
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

adjustment_request = urllib.request.Request(base + "/api/practice", data=json.dumps({
    "concept_id": "book_adjustment", "module": "data", "source": "us_stock", "symbol": "AAPL", "inputs": {}
}).encode(), headers={"Content-Type": "application/json"}, method="POST")
with urllib.request.urlopen(adjustment_request, timeout=90) as response:
    adjustment = json.load(response)
assert adjustment["provenance"] == "server_fetched_stock_corporate_action"
assert adjustment["values"]["new_shares_per_old_share"] == 4
assert adjustment["values"]["split_only_price_multiplier"] == 0.25
assert adjustment["adjustment_evidence"]["provider"] == "yahoo"
assert adjustment["adjustment_evidence"]["event"]["effective_trading_date"] == "2020-08-31"
assert len(adjustment["adjustment_evidence"]["observations"]) >= 2
print("AAPL historical split event and provider observations", "verified", flush=True)

# Verify each issuer can actually be retrieved by the production host. These
# expected values are transcribed from the original disclosure pages.
industry_cases = (
    ("bank_nim", "2318.HK", "net_interest_margin", 93427 / 4994494, "62a5bd793ef9a787cc95750d65e52803aa58fa424b01d94e754ea0120d5be8a3", 14886158),
    ("book_saas_arr", "SHOP", "annualized_recurring_revenue_run_rate", 2136, "4bf71232697a2270b2dbc38fc9609c11c27d545d6f4301fce3356fa60c6ef6de", 86468),
    ("book_platform_take_rate", "EBAY", "platform_take_rate", 10283 / 74667, "10530b8314c4dc49f9737b938f28ead7a70212885c35919fb361d145401f37fb", 1004020),
    ("book_share_counts", "600519", "restricted_shares_residual", 0, "474905deeaf0f875fc0a1b097a626c0c7852c427faadc5d7fc7816cbf45ea288", 1082847),
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

for concept, expected in (("book_free_float", 543137592 / 1252270215), ("book_float_market_cap", 1252270215 * 1377.18)):
    body = {"concept_id": concept, "module": "data", "source": "issuer_disclosure", "symbol": "600519", "inputs": {}}
    req = urllib.request.Request(base + "/api/practice", data=json.dumps(body).encode(), headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=90) as response:
        result = json.load(response)
    assert result["provenance"] == "verified_independent_a_share_float_case"
    assert math.isclose(result["values"][concept], expected, rel_tol=1e-12, abs_tol=1e-6)
    assert result["float_case"]["price"]["trading_date"] == "2025-12-31"
    assert result["float_case"]["price"]["basis"] == "unadjusted_daily_close"
    assert all(source["verification"]["matched_sha256"] == source["sha256"] for source in result["float_case"]["sources"][:3])
    print("Moutai fixed float case", concept, "verified", flush=True)

markets = (("binance", "BTCUSDT", 200), ("a_share", "600519", 1000), ("us_stock", "AAPL", 1000))
allowed = {"binance": {"binance_spot"}, "a_share": {"eastmoney", "tencent"}, "us_stock": {"yahoo", "yahoo_via_restricted_relay", "nasdaq"}}
for source, symbol, minimum in markets:
    snapshot = read_json("/api/data", {"source": source, "symbol": symbol, "limit": 5})
    provenance = snapshot["market_provenance"]
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

for source, symbol, _ in markets:
    body = {"strategy": "buy_and_hold", "source": source, "symbol": symbol, "limit": 120, "initial_capital": 100000}
    request = urllib.request.Request(base + "/api/backtest", data=json.dumps(body).encode(), headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(request, timeout=90) as response:
        result = json.load(response)
        assert response.status == 200
    assert len(result["equity_curve"]) == len(result["bars"]) == 120
    provider = result["market_provenance"]["provider"]
    assert provider in allowed[source] or (source == "binance" and provider == "local_csv_cache")
    if source in ("a_share", "us_stock"):
        assert result["market_provenance"]["stock_split_coverage"]["status"] == "verified_no_split_in_window"
    print(source, symbol, "production backtest", "verified", flush=True)

# Stage 3 market breadth must be real cross-sectional data. Validate the
# aggregate contract, exercise every practice route, then independently pull
# the member URLs and recompute the latest critical values without trusting
# the API's own observations.
breadth = read_json("/api/market-breadth/snapshot", {})
breadth_ids = {
    "ad_line", "breadth_thrust", "bullish_percent", "mcclellan",
    "new_high_low", "tick", "trin", "up_down_volume",
}
assert breadth["schema_version"] == 1
assert breadth["universe"]["id"] == "dow_30_2024_11_08"
assert breadth["universe"]["constituents_as_of"] == "2024-11-08"
assert breadth["universe"]["member_count"] == len(breadth["universe"]["symbols"]) == 30
assert len(set(breadth["universe"]["symbols"])) == 30
assert breadth["coverage"]["minimum_required"] == 27
assert 27 <= breadth["coverage"]["latest_eligible_members"] <= 30
assert len(breadth["source"]["members"]) == 30
concepts = {item["id"]: item for item in breadth["concepts"]}
assert set(concepts) == breadth_ids
for concept_id, item in concepts.items():
    assert item["status"] == "available", (concept_id, item)
    assert item["unit"] and item["definition"] and item["inputs"] is not None
    assert isinstance(item["latest"], (int, float)) and math.isfinite(item["latest"])

for concept_id in sorted(breadth_ids | {"book_mcclellan_sum"}):
    result = post_json("/api/practice", {
        "concept_id": concept_id,
        "module": "data",
        "source": "market_breadth",
        "inputs": {},
    })
    assert result["concept_id"] == concept_id and result["status"] == "computed"
    assert result["symbol"] is None and result["source"] == "market_breadth"
    assert result["input_kind"] == "market_breadth_case"
    assert result["notes"] and result["units"]
    value = result["values"][concept_id]
    assert isinstance(value, (int, float)) and math.isfinite(value), (concept_id, result)
    if concept_id == "tick":
        assert result["source_markets"] == {"tick": "crypto_spot"}
        assert result["provenance"] == "server_fetched_fixed_crypto_tick_snapshot"
        assert "market_tick" in result and "market_breadth" not in result
    else:
        assert result["source_markets"] == {"daily": "us_equity"}
        assert result["provenance"] == "server_fetched_fixed_market_breadth_snapshot"
        assert result["market_breadth"]["universe"]["id"] == "dow_30_2024_11_08"

members = breadth["source"]["members"]
# Re-fetch the raw provider documents from the production network, where the
# app itself runs. One bounded SSH batch avoids a second burst from the local
# WSL IP being mistaken for a provider outage (Yahoo may return HTTP 429).
provider_probe = """import json, time, urllib.error, urllib.request
urls = """ + repr([member["endpoint"] for member in members]) + """
documents = []
for url in urls:
    for attempt in range(5):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": "AXIOM independent release verifier/1.0", "Accept": "application/json"})
            with urllib.request.urlopen(request, timeout=45) as response:
                documents.append(json.load(response))
            break
        except urllib.error.HTTPError as error:
            if error.code != 429 or attempt == 4:
                raise
            time.sleep(min(2 ** attempt, 12))
    time.sleep(0.4)
print(json.dumps(documents))
"""
raw_documents = json.loads(subprocess.check_output(
    ["ssh", "root@sigma711.top", "python3", "-"],
    input=provider_probe.encode(), timeout=600,
))
assert len(raw_documents) == len(members) == 30
member_rows = {}
for member, raw in zip(members, raw_documents):
    assert member["symbol"] in breadth["universe"]["symbols"]
    assert member["provider"] in {"yahoo", "yahoo_via_restricted_relay"}, member
    assert member["endpoint"].startswith("https://")
    assert member["price_basis"] and member["corporate_actions"] is not None
    chart = raw["chart"]["result"][0]
    assert chart["meta"]["symbol"] == member["symbol"]
    quote = chart["indicators"]["quote"][0]
    rows = []
    for timestamp, close, volume in zip(chart["timestamp"], quote["close"], quote["volume"]):
        if close is None or volume is None:
            continue
        day = datetime.fromtimestamp(timestamp, timezone.utc).date().isoformat()
        if day <= breadth["as_of"]:
            rows.append((day, float(close), float(volume)))
    assert len(rows) >= 2, (member["symbol"], len(rows))
    rows = rows[-600:]
    assert len(rows) <= 600 and rows[-1][0] <= breadth["as_of"]
    assert all(rows[i][0] < rows[i + 1][0] for i in range(len(rows) - 1))
    member_rows[member["symbol"]] = rows

by_day = {}
for symbol, rows in member_rows.items():
    for previous, current in zip(rows, rows[1:]):
        day, close, volume = current
        direction = 1 if close > previous[1] else -1 if close < previous[1] else 0
        by_day.setdefault(day, []).append((symbol, direction, volume))
eligible = [(day, rows) for day, rows in sorted(by_day.items()) if len(rows) >= 27]
assert eligible and eligible[-1][0] == breadth["as_of"]
ad_line = sum(sum(direction for _, direction, _ in rows) for _, rows in eligible)
latest_day, latest_rows = eligible[-1]
advances = sum(direction > 0 for _, direction, _ in latest_rows)
declines = sum(direction < 0 for _, direction, _ in latest_rows)
unchanged = sum(direction == 0 for _, direction, _ in latest_rows)
up_volume = sum(volume for _, direction, volume in latest_rows if direction > 0)
down_volume = sum(volume for _, direction, volume in latest_rows if direction < 0)
trin = (advances / declines) / (up_volume / down_volume)
volume_ratio = up_volume / down_volume
latest_observation = breadth["observations"][-1]
assert latest_observation["date"] == latest_day
assert (latest_observation["advances"], latest_observation["declines"], latest_observation["unchanged"]) == (advances, declines, unchanged)
assert math.isclose(latest_observation["up_volume"], up_volume, rel_tol=1e-12)
assert math.isclose(latest_observation["down_volume"], down_volume, rel_tol=1e-12)
assert math.isclose(concepts["ad_line"]["latest"], ad_line, rel_tol=1e-12)
assert math.isclose(concepts["trin"]["latest"], trin, rel_tol=1e-12)
assert math.isclose(concepts["up_down_volume"]["latest"], volume_ratio, rel_tol=1e-12)

tick_inputs = concepts["tick"]["inputs"]
tick_net = 0
for member in tick_inputs["members"]:
    with urllib.request.urlopen(member["source_url"], timeout=30) as response:
        trades = json.load(response)
    eligible_trades = [trade for trade in trades if trade["T"] <= round(datetime.fromisoformat(tick_inputs["sample_at"].replace("Z", "+00:00")).timestamp() * 1000)]
    assert len(eligible_trades) >= 2
    previous, latest = eligible_trades[-2:]
    direction = 1 if float(latest["p"]) > float(previous["p"]) else -1 if float(latest["p"]) < float(previous["p"]) else 0
    assert member["direction"] == ("up" if direction > 0 else "down" if direction < 0 else "unchanged")
    assert math.isclose(member["previous_price"], float(previous["p"]), rel_tol=1e-12)
    assert math.isclose(member["latest_price"], float(latest["p"]), rel_tol=1e-12)
    tick_net += direction
assert tick_net == tick_inputs["net_tick"] == concepts["tick"]["latest"]
print("8 market-breadth concepts, 9 practices, fixed universes and independent provider recomputation verified", flush=True)

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

# The release must exercise every company-disclosure entry, not just the old
# four-issuer smoke sample. Numeric truth is independently checked in make ci.
catalog = read_json("/api/practice", {})
plans = {entry["id"]: entry["plan"] for entry in catalog["concepts"]}
with Path("docs/concept-audit.csv").open() as stream:
    disclosure_ids = [row["id"] for row in csv.DictReader(stream)
                      if row["dependency_group"] == "2_company_disclosure"]
assert len(disclosure_ids) == len(set(disclosure_ids)) == 105
golden = {
    "beneish_m": ("beneish_m", -2.294943021712222),
    "piotroski": ("piotroski_f_score", 7),
    "book_dcf": ("dcf_value_per_share", 125.12238523309892),
    "book_cape": ("cape", 53.214146004905224),
    "book_nav_discount": ("nav_premium_discount", -0.0852760736196319),
    "earnings_surprise": ("earnings_surprise", 0.15957446808510645),
    "short_interest": ("days_to_cover", 6.680184152966909),
}
disclosure_evidence = []
for concept in disclosure_ids:
    plan = plans[concept]
    assert plan["source_policy"] == "real_required" and plan["modules"] == ["data"]
    symbol = plan.get("fixed_symbol", "AAPL")
    source = plan.get("fixed_source", "us_stock")
    body = {"concept_id": concept, "module": "data", "source": source, "symbol": symbol, "inputs": {}}
    request = urllib.request.Request(base + "/api/practice", data=json.dumps(body).encode(),
                                     headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(request, timeout=180) as response:
        result = json.load(response)
    assert result["concept_id"] == concept and result["symbol"] == symbol
    assert result["source"] == source and result["bars"] == [] and result["inputs"] == {}
    assert result["status"] == ("partial" if concept == "short_interest" else "computed"), (concept, result)
    assert result["values"] and all(isinstance(value, (int, float)) and math.isfinite(value)
                                     for value in result["values"].values()), (concept, result["values"])
    case = result.get("industry_case", result.get("filing_case"))
    assert case and case["verification"]["matched_sha256"] == case["sha256"]
    assert case["verification"]["matched_bytes"] == case["bytes"]
    assert case["published"] and case["period"]["start"] and case["period"]["end"], (concept, case)
    assert case["period"]["start"] <= case["period"]["end"], (concept, case["period"])
    assert all(result["units"].get(key) for key in result["values"]), (concept, result["units"])
    assert result["notes"] and result["provenance"]
    if "industry_facts" in result:
        issuer = case["issuer"]
        assert issuer["reporting_entity"] and issuer["metric_entity"] and issuer["ticker"] == symbol
        facts = result["industry_facts"]
        calculation = facts["calculation"]
        assert calculation["formula"] and calculation["result_key"] in result["values"]
        assert facts["currency"] and facts["scale"] and facts["definitions"]
        fields = {fact["key"]: fact for fact in facts["reported_facts"]}
        assert calculation["operands"] and all(key in fields and key in facts["field_provenance"]
                                               for key in calculation["operands"])
        for fact in fields.values():
            assert fact["label"] and fact["unit"] and math.isfinite(fact["value"])
            if fact.get("kind") == "assumption":
                assert fact.get("source_url") is None and fact.get("pdf_page") is None
            else:
                assert (fact.get("source_url") or case["url"]).startswith("https://")
                assert fact.get("as_of") or case["period"]["end"]
        if concept == "short_interest":
            assert result["reason"] and "short_interest_ratio" not in result["values"]
    else:
        assert case["issuer"] and case["scope"] and case["ticker"] == symbol
        assert result["facts"]["units"] and result["facts"]["calculation_boundaries"]
    for document in case.get("sources", []):
        assert document["verification"]["matched_sha256"] == document["sha256"]
        assert document["verification"]["matched_bytes"] == document["bytes"]
    if concept in golden:
        key, expected = golden[concept]
        assert math.isclose(result["values"][key], expected, rel_tol=1e-10, abs_tol=1e-10), (concept, result["values"])
    disclosure_evidence.append({"concept_id": concept, "status": result["status"],
                                "values": result["values"], "case": case})
Path("target/deploy-disclosure-evidence.json").write_text(json.dumps({
    "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
    "verified_at": datetime.now().isoformat(), "cases": disclosure_evidence,
}, ensure_ascii=False, indent=2))
print("105 historical company-disclosure practices, source fingerprints and representative independent results verified", flush=True)
PYVERIFY
trap - ERR
