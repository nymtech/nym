#!/bin/sh

MNEMONIC=$(cat /genesis_volume/genesis_mnemonic | tail -n 1)

sed -i 's/mnemonic = "";/mnemonic = "'"${MNEMONIC}"'";/' upload-wasm.ts
# Give the validator some time to open the port
sleep 5
npx ts-node upload-wasm.ts
