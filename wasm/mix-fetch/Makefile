build: build-go-opt build-rust build-package-json

check-fmt: check-fmt-go check-fmt-rust

build-go:
	$(MAKE) -C go-mix-conn build-go

build-go-opt:
	$(MAKE) -C go-mix-conn build-go-opt

build-rust:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/mix-fetch
	wasm-opt -Oz -o ../../dist/wasm/mix-fetch/mix_fetch_wasm_bg.wasm ../../dist/wasm/mix-fetch/mix_fetch_wasm_bg.wasm

build-package-json:
	node build.mjs

check-fmt-go:
	$(MAKE) -C go-mix-conn check-fmt

check-fmt-rust:
	cargo fmt --check
	cargo clippy --target wasm32-unknown-unknown -- -Dwarnings