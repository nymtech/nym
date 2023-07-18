build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/client
	wasm-opt -Oz -o ../../dist/wasm/client/nym_client_wasm_bg.wasm ../../dist/wasm/client/nym_client_wasm_bg.wasm
