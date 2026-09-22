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

markets = (("binance", "BTCUSDT", 200), ("a_share", "600519", 1000), ("us_stock", "AAPL", 1000))
for source, symbol, minimum in markets:
    bars = read_json("/api/data", {"source": source, "symbol": symbol, "limit": 5})["bars"]
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
