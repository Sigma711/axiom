# Alpha158: official definition and faithful reuse

Verified 2026-10-12 against Microsoft Qlib primary sources. Executing the official `Alpha158DL.get_feature_config()` class in isolation yielded 158 expressions and 158 unique names.

The checked-in expression reference is generated from Qlib commit `54355232463878d2eebb91fe0ee5fa7fa1f5976c` in `docs/research/alpha158-official-expressions.json`. Two checked-in numeric fixtures compare all 157 executable formulas, point by point, with Qlib 0.9.7's own expression operators on changing bars and on ties/zero-volume bars; `make alpha-oracle-fixtures` regenerates them. Microsoft's MIT notice for reused expression definitions is in `licenses/qlib-MIT.txt`.

## Canonical feature contract

Default count: **9 candlestick + 4 current-price ratios + 29 rolling families × 5 windows = 158**. The generator is parameterized; customized windows/includes/excludes need not yield 158. Required raw fields: adjusted `open`, `high`, `low`, `close`, `volume`, and `vwap`. Windows: 5, 10, 20, 30, 60 observations. Candlestick names: KMID, KLEN, KMID2, KUP, KUP2, KLOW, KLOW2, KSFT, KSFT2. Price names: OPEN0, HIGH0, LOW0, VWAP0. Rolling families:

```
ROC MA STD BETA RSQR RESI MAX MIN QTLU QTLD RANK RSV IMAX IMIN IMXD
CORR CORD CNTP CNTN CNTD SUMP SUMN SUMD VMA VSTD WVMA VSUMP VSUMN VSUMD
```

The authoritative ordered name/expression pairs come directly from `Alpha158DL.get_feature_config()`; retain both, rather than translating factor names heuristically. For example, ROCd is `Ref($close,d)/$close`, not textbook percentage ROC; BETAd is normalized time-trend slope, not market beta. Rolling inclusion uses the key `LOW` although emitted names are MINd. [Official generator](https://github.com/microsoft/qlib/blob/main/qlib/contrib/data/loader.py#L57-L301).

## Operators and numerical compatibility

Required named operators: Abs, Corr, Greater, IdxMax, IdxMin, Less, Log, Max, Mean, Min, Quantile, Rank, Ref, Resi, Rsquare, Slope, Std, Sum, plus arithmetic and comparisons. Greater/Less mean elementwise maximum/minimum. Rank is a **time-series percentile within one instrument's rolling window**, with average ranks for ties; it is not cross-sectional. IdxMax/IdxMin return a **one-based position from the start of the actual window**, with the first extremum winning ties; do not implement “bars since high.” Pandas rolling generally uses `min_periods=1`; Std follows pandas sample standard deviation. Correlation, regressions, NaNs, quantiles, and zero variance require reference comparison. [Official operator implementation](https://github.com/microsoft/qlib/blob/main/qlib/data/ops.py).

## Data and cross-sectional boundaries

All 158 raw features can be calculated from one instrument's time series. The Alpha158 handler's default label is `Ref($close,-2)/Ref($close,-1)-1`: tomorrow-to-following-day return, unavailable at feature time. Default learning processors drop missing labels and normalize labels cross-sectionally by date; raw feature computation and learning preprocessing are separate contracts. [Handler](https://github.com/microsoft/qlib/blob/main/qlib/contrib/data/handler.py#L90-L146), [CSZScoreNorm](https://github.com/microsoft/qlib/blob/main/qlib/data/dataset/processor.py#L276-L297).

Qlib documents adjusted OHLC and volume, restoration factors, a common calendar, and NaNs for suspended stocks. Replacing VWAP with OHLC-average would silently change VWAP0; use actual trade-weighted VWAP or clearly report missingness. Match amount/volume units if deriving it. Require at least 61 observations for a fully populated 60-lag feature; this is stricter than Qlib's partial-window behavior and must be disclosed as a readiness policy. [Official data contract](https://qlib.readthedocs.io/en/latest/component/data.html).

A single stock can teach raw features; cross-sectional labels, stock-selection IC, and market portfolio evaluation require a same-date multi-instrument universe. Do not treat temporal observations of one ticker as that universe. The official LightGBM benchmark uses CSI300, chronological train/validation/test partitions, SignalRecord, SigAnaRecord, and a cost-bearing portfolio backtest. [Official benchmark](https://github.com/microsoft/qlib/blob/main/examples/benchmarks/LightGBM/workflow_config_lightgbm_Alpha158.yaml).

## Reuse route and teaching implications

Recommended: pin a Qlib commit/version, export its ordered expression manifest, then evaluate actual daily market data with Qlib as the numerical oracle. A Rust implementation should reuse this manifest and compare every output with the oracle on rising/falling, ties, constant price/volume, gaps, zeros, short windows, and real adjusted bars. Keep raw factors, imputed/normalized model inputs, and future labels visibly distinct. Restrict fitted preprocessing to training periods and avoid label overlap across evaluation boundaries.

Qlib is MIT licensed: adaptation and redistribution are permitted with the Microsoft copyright and MIT permission notice preserved for copied substantial code. This licenses code, not market data. [Official license](https://github.com/microsoft/qlib/blob/main/LICENSE). Alpha158 is an engineered feature set, not a single trading strategy or a claim of positive returns.
