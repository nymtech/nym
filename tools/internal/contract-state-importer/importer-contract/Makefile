all: build

build:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --lib --target wasm32-unknown-unknown --no-default-features
	wasm-opt --signext-lowering -O ../../../../target/wasm32-unknown-unknown/release/importer_contract.wasm -o ../../../../target/wasm32-unknown-unknown/release/importer_contract.wasm