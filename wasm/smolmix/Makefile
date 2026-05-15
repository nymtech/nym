build: build-rust

build-rust:
	taskset -c 0-11 wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/smolmix

build-debug:
	taskset -c 0-11 wasm-pack build --debug --scope nymproject --target web --out-dir ../../dist/wasm/smolmix --features debug

build-release-opt:
	taskset -c 0-11 wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/smolmix
	taskset -c 0-11 wasm-opt -Oz -o ../../dist/wasm/smolmix/smolmix_wasm_bg.wasm ../../dist/wasm/smolmix/smolmix_wasm_bg.wasm

check-fmt:
	cargo fmt --check
	cargo clippy --target wasm32-unknown-unknown -- -Dwarnings

dev: build-debug
	cd internal-dev && npm start

# Build the internal-dev webpack bundle for Playwright.
dev-build: build-debug
	cd internal-dev && npm install && npm run build

# Playwright tests (headless browser).
# Prereqs: make dev-build && cd tests && npm install && npx playwright install
# IPR_ADDRESS env var is optional; tests fall back to the default in
# internal-dev/index.html / internal-dev/headless.js when unset.
test\:smoke:
	cd tests && npx playwright test --project=smoke-chromium --project=smoke-firefox --project=smoke-webkit

test\:suite:
	cd tests && npx playwright test --project=suite-chromium --project=suite-firefox --project=suite-webkit

test:
	cd tests && npx playwright test
