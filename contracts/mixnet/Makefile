wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown

upload: wasm
	validator-client-scripts --nymd-url ${QA_NYMD_URL} --mnemonic "${QA_MNEMONIC}" upload --memo 'Uploading mixnet contract to QAnet' --wasm-path target/wasm32-unknown-unknown/release/mixnet_contracts.wasm

clean:
	cargo clean

