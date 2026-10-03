# Portable original-source snapshots

These files are reviewed original source documents or datasets used only when
the named provider is temporarily unavailable. The real-browser tests first
attempt each public URL, record an explicit snapshot-fallback annotation, and
always verify the snapshot byte length and SHA-256 before using its contents.

| File | Public source | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `../../../../data/verified-sources/zoom-q1-fy2025-prepared-remarks.pdf` | https://investors.zoom.us/static-files/70629942-ff77-4bed-91d6-422766c47e6b | 118862 | `79f0e6b5126a47869f196d07aa3ab3626c4a61cdbb46e17a79762ab264fbeaf4` |
| `../../../../data/verified-sources/fred-cpi-2026-10-03.csv` | https://fred.stlouisfed.org/graph/fredgraph.csv?id=CPIAUCSL | 2097 | `d4f940d3358dd45bb74e61cf0a4cfe06194b35050d34a6a122f23f86304577e3` |
| `../../../../data/verified-sources/nuveen-proxy-2025.pdf` | https://documents.nuveen.com/Documents/Nuveen/Viewer.aspx?download=1&uniqueId=0779f60a-86ee-4128-87a3-1db9363e171e | 1438050 | `328251373d35c20d0450538dad87c1bf28ca6747393dbc7ddb72d57bb1ccfb19` |

The FRED file is the CPIAUCSL observation snapshot retrieved on 2026-10-03
after then-current revisions. It is not a historical real-time vintage.

The Nuveen PDF is packaged with the product because the official URL currently
returns a TIAA security page instead of the reviewed PDF. The application still
tries the official URL on a cold cache and uses this archived original only
after its byte length and SHA-256 match the reviewed evidence exactly.
