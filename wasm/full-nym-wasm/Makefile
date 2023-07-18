build-full:
	wasm-pack build --all-features --scope nymproject --target no-modules --out-dir ../../dist/wasm/full-nym-wasm
	wasm-opt -Oz -o ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm
