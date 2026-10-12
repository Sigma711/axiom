import json
import math
import sys
from pathlib import Path

import pandas as pd
import qlib
from qlib.data import ops
from qlib.data.base import Feature
from qlib.utils import parse_field

repo = Path(__file__).resolve().parents[1]
catalog = json.loads((repo / 'docs/research/alpha158-official-expressions.json').read_text())
assert qlib.__version__ == '0.9.7'
assert catalog['source_commit'] == '54355232463878d2eebb91fe0ee5fa7fa1f5976c'
edge = len(sys.argv) > 1 and sys.argv[1] == 'edge'
bars = []
for i in range(84):
    opening = (100 + (i // 6) * 0.25) if edge else (100 + i * 0.35 + 2 * math.sin(i / 3))
    close = opening if edge else opening + 1.2 * math.cos(i / 4)
    high = max(opening, close) + 0.7 + 0.1 * math.sin(i)
    low = min(opening, close) - 0.5 - 0.1 * math.cos(i)
    volume = (0 if i % 17 == 0 else 1000 + (i // 4) * 70) if edge else (1000 + i * 40 + 200 * math.sin(i / 2))
    bars.append({'timestamp': pd.Timestamp('2025-01-01') + pd.Timedelta(days=i), 'open': opening, 'high': high, 'low': low, 'close': close, 'volume': volume})
data = {field: pd.Series([bar[field] for bar in bars], dtype='float64') for field in ('open', 'high', 'low', 'close', 'volume')}

class LocalFeature(Feature):
    def _load_internal(self, instrument, start_index, end_index, freq):
        return data[self._name].iloc[start_index:end_index + 1].copy()

ops.Feature = LocalFeature

factors = {}
for item in catalog['features']:
    if item['id'] == 'vwap0':
        continue
    expression = eval(parse_field(item['expression']), {'Feature': LocalFeature, 'Operators': ops})
    values = expression.load('fixture', 0, len(bars)-1, 'day')
    factors[item['id']] = [float(value) if pd.notna(value) and math.isfinite(value) else None for value in values]

for bar in bars:
    bar['timestamp'] = bar['timestamp'].isoformat() + 'Z'
name = 'alpha158-qlib-edge-oracle.json' if edge else 'alpha158-qlib-oracle.json'
(repo / 'tests/fixtures' / name).write_text(json.dumps({
    'qlib_version': '0.9.7',
    'source_commit': catalog['source_commit'],
    'bars': bars,
    'factors': factors,
}, separators=(',', ':'), allow_nan=False) + '\n')
print(len(factors), len(bars))
