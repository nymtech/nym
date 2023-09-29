all: build-full

build-full:
	wasm-pack build --all-features --scope nymproject --target web --out-dir ../../dist/wasm/full-nym-wasm
	wasm-opt -Oz -o ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm
