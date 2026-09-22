# Quality gate

Every change must preserve these release requirements.

- Product code in every language must have at least 95% line coverage.
- Each product module and each UI component must independently have at least 95% line coverage; a high repository-wide average never compensates for a weak module.
- Enforce coverage independently for the Rust backend and browser-executed TypeScript/React frontend, and report both figures in CI.
- Test real user-visible behavior from browser end to end as well as unit-level behavior where each is the correct seam.
- Exclude only non-product operational and utility scripts (for example scripts/, CI glue, build helpers, generated artifacts, and test fixtures) from the coverage denominator. Do not exclude application modules or UI components to improve a number.
- A coverage rule is not complete until make coverage and CI fail when the relevant threshold is missed.
