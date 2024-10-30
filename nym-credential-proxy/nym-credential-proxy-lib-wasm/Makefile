build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/nym-credential-proxy-lib-wasm
	wasm-opt -Oz -o ../../dist/wasm/nym-credential-proxy-lib-wasm/nym_credential_proxy_lib_wasm_bg.wasm ../../dist/wasm/nym-credential-proxy-lib-wasm/nym_credential_proxy_lib_wasm_bg.wasm

