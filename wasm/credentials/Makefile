build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/credentials
	wasm-opt -Oz -o ../../dist/wasm/credentials/nym_node_tester_wasm_bg.wasm ../../dist/wasm/credentials/nym_node_tester_wasm_bg.wasm
