build: build-rust

build-rust:
	wasm-pack build --target web --no-default-features --features rustcrypto

build-debug:
	wasm-pack build --debug --target web --no-default-features --features rustcrypto,debug

build-release-opt:
	wasm-pack build --target web --no-default-features --features rustcrypto
	wasm-opt -Oz -o pkg/smolmix_wasm_bg.wasm pkg/smolmix_wasm_bg.wasm

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
