# `taskset -c 0-11` caps cargo to 12 cores so the build does
# not saturate CI runners. Override empty to
# disable on machines without taskset (macOS, BSDs, sandboxes):
#   make TASKSET= build-debug
TASKSET ?= taskset -c 0-11

.PHONY: build build-rust build-debug build-release-opt check-fmt dev dev-build \
        internal-dev-install test test\:smoke test\:suite

build: build-rust

build-rust:
	$(TASKSET) wasm-pack build --scope nymproject --target web

build-debug:
	$(TASKSET) wasm-pack build --debug --scope nymproject --target web --features debug

build-release-opt:
	$(TASKSET) wasm-pack build --scope nymproject --target web
	$(TASKSET) wasm-opt -Oz -o pkg/smolmix_wasm_bg.wasm pkg/smolmix_wasm_bg.wasm

check-fmt:
	cargo fmt --check
	cargo clippy --target wasm32-unknown-unknown -- -Dwarnings

dev: build-debug
	cd internal-dev && ./node_modules/.bin/webpack serve --open

# Build the internal-dev webpack bundle for Playwright.
dev-build: build-debug
	cd internal-dev && ./node_modules/.bin/webpack --mode production

# One-time setup for internal-dev. Run this after fresh clone or when
# internal-dev/package.json changes. Standalone install (no workspace walk).
internal-dev-install:
	cd internal-dev && pnpm install --ignore-workspace

# Playwright tests (headless browser).
# Prereqs: make dev-build && cd tests && pnpm install && pnpm exec playwright install
# IPR_ADDRESS env var is optional; tests fall back to the default in
# internal-dev/index.html / internal-dev/headless.js when unset.
test\:smoke:
	cd tests && pnpm exec playwright test --project=smoke-chromium --project=smoke-firefox --project=smoke-webkit

test\:suite:
	cd tests && pnpm exec playwright test --project=suite-chromium --project=suite-firefox --project=suite-webkit

test:
	cd tests && pnpm exec playwright test
