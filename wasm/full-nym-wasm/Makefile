all: build-full

build-full:
	taskset -c 0-7 wasm-pack build --all-features --scope nymproject --target web --out-dir ../../dist/wasm/full-nym-wasm
	taskset -c 0-7 wasm-opt -Oz -o ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm
