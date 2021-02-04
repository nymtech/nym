#!/bin/bash

# Don't get confused by the ComsWasm docs currently up. They're old. CosmWasm (and Tendermint) now combines nymd + wasmcli into nymd. 

# RESET the chain every time it starts
rm -rf ~/.nymd/

# default home is ~/.nymd
# if you want to setup multiple apps on your local make sure to change this value
export APP_HOME="$HOME/.nymd"

# initialize nymd configuration files
nymd init nymnet --chain-id nymnet --home "${APP_HOME}"

# add minimum gas prices config to app configuration file
sed -i -r 's/minimum-gas-prices = ""/minimum-gas-prices = "0.025unym"/' "${APP_HOME}/config/app.toml"

# enable the rpc server
sed -i -r 's/enable = false/enable = true/' "$APP_HOME/config/app.toml"

# disallow everybody else from running smart contract code in our blockchain
python set_contract_upload_permissions.py

# nymd keys add dave # adds a >key for dave if one doesn't already exist
DAVE_ADDRESS=$(nymd keys show dave -a)

# add your wallet addresses to genesis
nymd add-genesis-account "$DAVE_ADDRESS" 1000000000000000unym,100000000000000000stake --home "$APP_HOME"

# add dave's address as validator's address
nymd gentx dave 1000000000stake --chain-id nymnet --home "$APP_HOME"

# collect gentxs to genesis
nymd collect-gentxs --home "$APP_HOME"

# validate the genesis file
nymd validate-genesis --home "$APP_HOME"




