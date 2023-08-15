build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/client
	wasm-opt -Oz -o ../../dist/wasm/client/nym_client_wasm_bg.wasm ../../dist/wasm/client/nym_client_wasm_bg.wasm

build-node:
	wasm-pack build --scope nymproject --target nodejs --out-dir ../../dist-node/wasm/client
	wasm-opt -Oz -o ../../dist/wasm/client/nym_client_wasm_bg.wasm ../../dist-node/wasm/client/nym_client_wasm_bg.wasm