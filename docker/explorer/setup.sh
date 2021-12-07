#!/bin/sh

git clone https://github.com/nymtech/nym.git

mkdir /explorer-copy
cp -r nym/explorer/* /explorer-copy

cd /explorer-copy

npm install
#baseUrl
sed -i 's/https:\/\/testnet-milhon-explorer.nymtech.net\//http:\/\/localhost:3080/' ./src/api/constants.ts

#master validator
sed -i 's/https:\/\/testnet-milhon-validator1.nymtech.net/http:\/\/localhost:26657/' ./src/api/constants.ts

#big dipper url
sed -i 's/https:\/\/testnet-milhon-blocks.nymtech.net\//http:\/\/localhost:3080/' ./src/api/constants.ts

nohup npm run start