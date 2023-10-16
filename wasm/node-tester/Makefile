all: build

build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/node-tester
	wasm-opt -Oz -o ../../dist/wasm/node-tester/nym_node_tester_wasm_bg.wasm ../../dist/wasm/node-tester/nym_node_tester_wasm_bg.wasm
