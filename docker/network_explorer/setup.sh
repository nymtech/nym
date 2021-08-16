#!/bin/sh

git checkout b1ba77461a50ab6b31b8384a8f8e348141b37a43

sed -i 's/https:\/\/testnet-finney-blocks.nymtech.net\//http:\/\/localhost:3080/' ./layouts/default.vue
sed -i 's/https:\/\/testnet-milhon-validator1.nymtech.net/http:\/\/localhost:26657/' networkVariables.json
sed -i 's/https:\/\/testnet-finney-blocks.nymtech.net\//http:\/\/localhost:3080/' pages/index.vue

yarn install

