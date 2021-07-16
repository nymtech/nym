#!/bin/sh

while ! [ -s /contract_volume/contract_address ]; do
	sleep 1
done
CONTRACT_ADDRESS=$(cat /contract_volume/contract_address)
sed -i 's/export const BONDING_CONTRACT_ADDRESS: string = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen";/export const BONDING_CONTRACT_ADDRESS: string = "'"${CONTRACT_ADDRESS}"'";/' pages/_app.tsx 
sed -i 's/export const VALIDATOR_URLS: string\[\] = \[/export const VALIDATOR_URLS: string\[\] = \[ "localhost:26657",/' pages/_app.tsx 
sed -i 's/"https:\/\/testnet-milhon-validator1.nymtech.net",//' pages/_app.tsx
sed -i 's/"https:\/\/testnet-milhon-validator2.nymtech.net",//' pages/_app.tsx
yarn dev
