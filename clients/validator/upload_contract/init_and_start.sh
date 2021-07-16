#!/bin/sh

# Wait until the mnemonic is created
while ! [ -s /genesis_volume/genesis_mnemonic ]; do
	sleep 1
done
MNEMONIC=$(cat /genesis_volume/genesis_mnemonic)

sed -i 's/mnemonic = "";/mnemonic = "'"${MNEMONIC}"'";/' upload-wasm.ts

# Wait until the validator opens its port
while ! nc -z genesis_validator 26657; do
	sleep 1
done
npx ts-node upload-wasm.ts | tail -n 1 | cut -d' ' -f 3 > /contract_volume/contract_address
