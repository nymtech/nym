#!/bin/sh

cd /nym/clients/validator
yarn install
yarn build

cd /nym/docker/typescript_client/upload_contract
npm install

# Wait until the mnemonic is created
while ! [ -s /genesis_volume/genesis_mnemonic ]; do
	sleep 1
done

# Wait until the validator opens its port
while ! nc -z genesis_validator 26657; do
	sleep 1
done

chmod 777 /contract_volume
npx ts-node upload-wasm.ts
