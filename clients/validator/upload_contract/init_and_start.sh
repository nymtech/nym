#!/bin/sh

while ! [ -s /genesis_volume/genesis_mnemonic ]; do
	sleep 1
done
MNEMONIC=$(cat /genesis_volume/genesis_mnemonic | tail -n 1)

sed -i 's/mnemonic = "";/mnemonic = "'"${MNEMONIC}"'";/' upload-wasm.ts
# Give the validator some time to open the port
sleep 10
npx ts-node upload-wasm.ts | tail -n 1 | cut -d' ' -f 3 > /contract_volume/contract_address
