build: build-go-opt build-rust

build-go:
	$(MAKE) -C go-mix-conn build-go

build-go-opt:
	$(MAKE) -C go-mix-conn build-go-opt

build-rust:
	wasm-pack build --scope nymproject --target no-modules
	wasm-opt -Oz -o pkg/nym_mix_fetch_bg.wasm pkg/nym_mix_fetch_bg.wasm