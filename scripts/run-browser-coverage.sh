#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"
# WSL non-interactive shells normally skip nvm; use the pinned project runtime.
if [[ -d "$HOME/.nvm/versions/node" ]]; then
  node_bin="$(find "$HOME/.nvm/versions/node" -mindepth 2 -maxdepth 2 -type d -name bin | sort -V | tail -n 1)"
  [[ -z "$node_bin" ]] || export PATH="$node_bin:$PATH"
fi
# The browser suite owns this dedicated test port; a stale prior run must not
# make Vite silently choose a different one.
pgrep -af "[v]ite.*18181" | awk '{print $1}' | xargs -r kill || true
rm -rf web/test-results coverage/web coverage/web-e2e
mkdir -p coverage
# Unit-level cases exercise pure API/chart/performance behavior; Playwright below
# remains the release denominator because it observes the real browser bundle.
(
  cd web
  npm run test:coverage
)

(
  cd web
  VITE_BASE=/ npm run dev -- --host 127.0.0.1 --port 18181 > ../coverage/vite-coverage.log 2>&1 &
  echo $! > ../coverage/vite-coverage.pid
)
pid="$(cat coverage/vite-coverage.pid)"
cleanup() {
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  rm -f coverage/vite-coverage.pid
}
trap cleanup EXIT

for attempt in $(seq 1 30); do
  if curl --fail --silent --show-error http://127.0.0.1:18181/ >/dev/null; then
    break
  fi
  if ! kill -0 "$pid" 2>/dev/null; then
    cat coverage/vite-coverage.log >&2
    exit 1
  fi
  sleep 1
done
curl --fail --silent http://127.0.0.1:18181/ >/dev/null

(
  cd web
  PW_V8_COVERAGE=1 npm exec -- playwright test
)
node scripts/merge-playwright-v8-coverage.mjs
node scripts/merge-browser-lcov.mjs coverage/web/lcov.info coverage/web-e2e/lcov.info coverage/web-combined/lcov.info
