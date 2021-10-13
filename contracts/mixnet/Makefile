upload: code_id

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown

code_id: 
	echo 13 > code_id

migrate: upload
	cat code_id | xargs -I {} validator-client-scripts --nymd-url ${QA_NYMD_URL} --mnemonic "${QA_MNEMONIC}" --contract ${QA_CONTRACT}  migrate --code-id {}

init: upload
	cat code_id | xargs -I {} validator-client-scripts --nymd-url ${QA_NYMD_URL} --mnemonic "${QA_MNEMONIC}" --contract ${QA_CONTRACT}  init --code-id {}

clean:
	cargo clean
	rm -f code_id

