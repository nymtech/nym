opt: wasm
	wasm-opt -Os ../target/wasm32-unknown-unknown/release/mixnet_contract.wasm -o ../target/wasm32-unknown-unknown/release/mixnet_contract.wasm

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown
