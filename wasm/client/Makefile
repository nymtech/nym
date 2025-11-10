all: build build-node


build: test build-wasm

build-wasm:
	taskset -c 0-7 wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/client
	taskset -c 0-7 wasm-opt -Oz -o ../../dist/wasm/client/nym_client_wasm_bg.wasm ../../dist/wasm/client/nym_client_wasm_bg.wasm

build-debug-dev:
	wasm-pack build --debug --scope nymproject --target no-modules

build-rust-node:
	taskset -c 0-7 wasm-pack build --scope nymproject --target nodejs --out-dir ../../dist/node/wasm/client
	taskset -c 0-7 wasm-opt -Oz -o ../../dist/node/wasm/client/nym_client_wasm_bg.wasm ../../dist/node/wasm/client/nym_client_wasm_bg.wasm

build-package-json-node:
	node build-node.mjs

build-node: build-rust-node build-package-json-node

test:
	wasm-pack test --node
