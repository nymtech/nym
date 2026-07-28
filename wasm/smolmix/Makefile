# `taskset -c 0-11` caps cargo to 12 cores so the build does
# not saturate CI runners. Override empty to
# disable on machines without taskset (macOS, BSDs, sandboxes):
#   make TASKSET= build-debug
TASKSET ?= taskset -c 0-11

.PHONY: build build-rust build-debug build-release-opt check-fmt dev dev-build \
        internal-dev-install mark-pkg-private test test\:smoke test\:suite

build: build-rust

build-rust:
	$(TASKSET) wasm-pack build --scope nymproject --target web --out-name smolmix_wasm
	$(MAKE) mark-pkg-private

build-debug:
	$(TASKSET) wasm-pack build --debug --scope nymproject --target web --out-name smolmix_wasm --features debug
	$(MAKE) mark-pkg-private

build-release-opt:
	$(TASKSET) wasm-pack build --scope nymproject --target web --out-name smolmix_wasm
	$(TASKSET) wasm-opt -Oz -o pkg/smolmix_wasm_bg.wasm pkg/smolmix_wasm_bg.wasm
	$(MAKE) mark-pkg-private

# nym-smolmix-wasm is workspace-internal: its bytes are base64-inlined into
# @nymproject/mix-tunnel at build time, so it must never reach npm. wasm-pack
# does not write `"private": true` itself, so we patch the generated package.json
# in place. Guard against re-application so the target stays idempotent.
#
# The generated package name is also pinned to @nymproject/smolmix-wasm (not the
# derived @nymproject/nym-smolmix-wasm). The crate carries the nym- prefix, but
# the private artifact keeps its established name so mix-tunnel's imports stay
# unchanged; `--out-name smolmix_wasm` on the wasm-pack calls above pins the file
# basenames to match.
mark-pkg-private:
	@if [ -f pkg/package.json ]; then \
		jq '. + {private: true, name: "@nymproject/smolmix-wasm"}' pkg/package.json > pkg/package.json.tmp \
			&& mv pkg/package.json.tmp pkg/package.json; \
	fi

check-fmt:
	cargo fmt --check
	cargo clippy --target wasm32-unknown-unknown -- -Dwarnings
	cargo clippy --target wasm32-unknown-unknown --features debug -- -Dwarnings

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
