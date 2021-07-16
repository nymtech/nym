#!/bin/sh

# Wait until the mnemonic is created
while ! [ -s /genesis_volume/genesis_mnemonic ]; do
	sleep 1
done

# Wait until the validator opens its port
while ! nc -z genesis_validator 26657; do
	sleep 1
done

npx ts-node upload-wasm.ts
