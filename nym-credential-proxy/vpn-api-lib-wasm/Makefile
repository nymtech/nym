build:
	wasm-pack build --scope nymproject --target web --out-dir ../../dist/wasm/nym-vpn-api-lib-wasm
	wasm-opt -Oz -o ../../dist/wasm/nym-vpn-api-lib-wasm/nym_vpn_api_lib_wasm_bg.wasm ../../dist/wasm/nym-vpn-api-lib-wasm/nym_vpn_api_lib_wasm_bg.wasm

