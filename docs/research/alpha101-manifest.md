# Alpha101 manifest provenance

`src/alpha101_catalog.json` contains all 101 original identities and mathematical formulas from [Kakushadze, 101 Formulaic Alphas, Appendix A, pp. 8-15](https://arxiv.org/pdf/1601.00991). Chinese summaries are original explanations. Downloaded PDF SHA-256: `1f9c21afe32dcb3ee77b31548acdaea00451fbfa1c0ee10c907867bcc736fce9`.

Extraction used pypdf on the primary PDF, normalizing whitespace and standalone page numbers only. Alpha 53 has a space before its colon, explicitly accommodated. All 101 contiguous IDs, balanced expressions and unique summaries were checked; tests also lock exact formulas 6, 41, 53 and 101 and check input tokens for every formula. This is transcription validation, not independent executable verification.

[Definitions on pp. 15-16](https://arxiv.org/pdf/1601.00991#page=15) distinguish cross-sectional rank from time-series rank and define adv as average daily dollar volume; noninteger day windows are floored. VWAP requires true transaction-weighted data. Industry memberships and cap need point-in-time data. The manifest records exact adv windows and industry levels and forbids single-security substitutes for cross-sectional operations.

The source defines SignedPower as x^a; some external implementations instead use signed-magnitude power. It defines min/max as rolling aliases but later passes two expressions, often interpreted elementwise. These unresolved source semantics are explicitly flagged on relevant factors. Tie handling, variance conventions, missing values, division by zero and extreme-day index direction also require a declared implementation policy.

[Timing discussion on p. 4](https://arxiv.org/pdf/1601.00991#page=4) identifies 42, 48, 53 and 54 as delay-0. A next-session teaching experiment must distinguish its timing from the original. Every row is catalog_only: no claim of executable implementation, input availability, verified backtest or investment efficacy. Runtime availability belongs to independently validated code.
