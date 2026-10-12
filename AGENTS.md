# Quality gate

Every change must preserve these release requirements.

- Product code in every language must have strictly greater than 95% line coverage; exactly 95% fails.
- Each product module and each UI component must independently exceed 95% line coverage; a high repository-wide average never compensates for a weak module.
- Enforce coverage independently for the Rust backend and browser-executed TypeScript/React frontend, and report both figures in CI.
- Test real user-visible behavior from browser end to end as well as unit-level behavior where each is the correct seam.
- Exclude only non-product operational and utility scripts (for example scripts/, CI glue, build helpers, generated artifacts, and test fixtures) from the coverage denominator. Do not exclude application modules or UI components to improve a number.
- A coverage rule is not complete until make coverage and CI fail when the relevant threshold is missed.

# Product UI

- Render every visible control with the project's styled control system; browser-default appearance, native select menus, number spinners, and unstyled buttons are not release-ready.
- Reuse the existing shared control for the same or similar interaction. Extend that control once when a new state is needed; keep its appearance and behavior consistent across pages, themes, and responsive widths.
- Verify changed controls in a real browser in light and dark themes, at desktop and narrow widths, including open, hover, focus, disabled, and keyboard states where applicable.

# README

- After every user-visible feature or deployment change, check README.md against the actual application and commands; update stale names, capabilities, limitations, and steps in the same change.
- Use README.md only for a concise beginner-facing technical overview, core quant-trading and engineering ideas, and an executable build/test/deployment guide. Write natural Chinese that is vivid, precise, and free of filler, feature wish lists, or internal progress-report language; put detailed lessons in the product, not README.md.
