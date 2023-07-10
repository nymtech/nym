build: build-go-opt build-rust

check-fmt: check-fmt-go check-fmt-rust

build-go:
	$(MAKE) -C go-mix-conn build-go

build-go-opt:
	$(MAKE) -C go-mix-conn build-go-opt

build-rust:
	wasm-pack build --scope nymproject --target no-modules
	wasm-opt -Oz -o pkg/nym_mix_fetch_bg.wasm pkg/nym_mix_fetch_bg.wasm

check-fmt-go:
	$(MAKE) -C go-mix-conn check-fmt

check-fmt-rust:
	cargo fmt --check
	cargo clippy --target wasm32-unknown-unknown -- -Dwarnings

