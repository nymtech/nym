all: build build-node


build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/zknym-lib
	wasm-opt -Oz -o ../../dist/wasm/zknym-lib/zknym_lib_bg.wasm ../../dist/wasm/zknym-lib/zknym_lib_bg.wasm

build-debug-dev:
	wasm-pack build --scope nymproject --target no-modules

build-rust-node:
	wasm-pack build --scope nymproject --target nodejs --out-dir ../../dist/node/wasm/zknym-lib
	wasm-opt -Oz -o ../../dist/node/wasm/zknym-lib/zknym_lib_bg.wasm ../../dist/node/wasm/zknym-lib/zknym_lib_bg.wasm

#build-package-json-node:
#	node build-node.mjs

build-node: # build-rust-node build-package-json-node
