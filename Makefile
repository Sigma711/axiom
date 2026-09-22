SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help

# Prefer rustup when available; distro cargo may not understand Cargo.lock v4.
ifneq ($(wildcard $(HOME)/.cargo/bin/cargo),)
export PATH := $(HOME)/.cargo/bin:$(PATH)
endif
# WSL noninteractive shells do not source nvm; prefer an installed Node 24.
AXIOM_NODE_DIR := $(shell printf '%s\n' $(HOME)/.nvm/versions/node/v24.* | sort -V | tail -n 1)
ifneq ($(wildcard $(AXIOM_NODE_DIR)/bin/node),)
export PATH := $(AXIOM_NODE_DIR)/bin:$(PATH)
endif
NPM ?= npm
PLAYWRIGHT_ARGS ?=
AXIOM_PORT ?= 8080

.PHONY: help setup browser-deps build build-web build-rust build-debug run-debug dev serve serve-test test test-rust test-one test-web test-e2e test-e2e-real test-visual lint lint-rust lint-web check-format fmt check ci coverage tools verify-published
help:
	@printf '%s\n' 'make setup      Install locked frontend dependencies and browser' 'make build      Build frontend and Rust release' 'make serve      Run the production app (AXIOM_PORT=8080)' 'make dev        Run the frontend dev server' 'make test       Rust, frontend unit tests, real-browser E2E and visual checks' 'make check      Formatting, lint, typecheck, build and every test' 'make coverage   Rust and Chromium executable-source coverage, each enforced at 95%' 'make fmt        Format Rust source'

setup:
	cd web && $(NPM) ci
	cd web && $(NPM) exec -- playwright install chromium
browser-deps:
	cd web && $(NPM) exec -- playwright install-deps chromium
	@if [ "$$(id -u)" = 0 ]; then apt-get install -y fonts-dejavu-core fonts-droid-fallback; else sudo apt-get install -y fonts-dejavu-core fonts-droid-fallback; fi
	fc-match sans-serif
	fc-match "sans-serif:charset=4e00"
	fc-match monospace

build: build-web build-rust
build-web:
	cd web && $(NPM) run build
build-rust:
	cargo build --release --locked
build-debug:
	cargo build --locked
run-debug: build-debug build-web
	AXIOM_PORT=$(AXIOM_PORT) ./target/debug/axiom

serve: build
	AXIOM_PORT=$(AXIOM_PORT) cargo run --release --locked
serve-test:
	AXIOM_PORT=18080 AXIOM_OFFLINE=1 AXIOM_DATA_DIR=target/e2e-data cargo run --locked
dev:
	cd web && $(NPM) run dev -- --host 127.0.0.1

test: test-rust test-web test-e2e test-e2e-real
test-rust:
	cargo test --all-targets --locked --no-fail-fast
test-one:
	@test -n "$(TEST)" || (printf '%s\n' 'Use make test-one TEST=integration_test'; exit 2)
	cargo test --locked --test $(TEST)
test-web:
	cd web && $(NPM) run typecheck
	cd web && $(NPM) run test:unit
test-e2e: build-web
	cd web && $(NPM) run test:e2e -- $(PLAYWRIGHT_ARGS)
test-e2e-real: build-web
	cd web && AXIOM_REAL=1 $(NPM) exec -- playwright test $(PLAYWRIGHT_ARGS)
test-visual: test-e2e test-e2e-real

lint: check-format lint-rust lint-web
check-format:
	cargo fmt --all -- --check
lint-rust:
	cargo clippy --all-targets --locked -- -D warnings
lint-web:
	cd web && $(NPM) run typecheck
fmt:
	cargo fmt --all
check: lint build test
ci: check

tools:
	rustup component add rustfmt clippy llvm-tools-preview
	cargo install cargo-llvm-cov --locked
coverage: build-web
	rm -rf coverage/rust coverage/web-e2e coverage/web-combined coverage/lcov.info coverage/html
	mkdir -p coverage/rust
	cargo llvm-cov --all-targets --locked --lcov --output-path coverage/rust/lcov.info
	bash scripts/run-browser-coverage.sh
	cat coverage/rust/lcov.info coverage/web-combined/lcov.info > coverage/lcov.info
	node scripts/check-line-coverage.mjs coverage/rust/lcov.info coverage/web-combined/lcov.info
	cargo llvm-cov report --html --output-dir coverage/html

# Run after committing and pushing; rebuilds the AST map against that revision.
verify-published:
	@test -z "$$(git status --porcelain)" || (printf '%s\n' 'Commit all source changes before verifying published GitHub links'; exit 2)
	git fetch --quiet origin main
	git merge-base --is-ancestor HEAD FETCH_HEAD || (printf '%s\n' 'Current commit is not published on origin/main; push before verification'; exit 2)
	AXIOM_REQUIRE_GITHUB_LINKS=1 cargo test --locked --test code_links_test
