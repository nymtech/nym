build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/credentials
	wasm-opt -Oz -o ../../dist/wasm/credentials/nym_credential_client_wasm_bg.wasm ../../dist/wasm/credentials/nym_credential_client_wasm_bg.wasm

# make my life easier so I wouldn't need to deal with any bundlers. sorry @MS : )
build-debug-dev:
	wasm-pack build --scope nymproject --target no-modules