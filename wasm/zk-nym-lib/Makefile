build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/zk-nym-lib
	wasm-opt -Oz -o ../../dist/wasm/zk-nym-lib/zk_nym_lib_bg.wasm ../../dist/wasm/zk-nym-lib/zk_nym_lib_bg.wasm

# make my life easier so I wouldn't need to deal with any bundlers. sorry @MS : )
build-debug-dev:
	wasm-pack build --scope nymproject --target no-modules