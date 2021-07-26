#!/bin/sh

export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/root
PASSPHRASE=passphrase

cd /root

if [ "$1" = "genesis" ]; then
  if [ ! -d "/root/.nymd" ]; then
    ./nymd init nymnet --chain-id nymnet 2> /dev/null
    sed -i 's/minimum-gas-prices = ""/minimum-gas-prices = "0.025u'"${BECH32_PREFIX}"'"/' /root/.nymd/config/app.toml
    sed -i '0,/enable = false/s//enable = true/g' /root/.nymd/config/app.toml
    sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = \["*"\]/' /root/.nymd/config/config.toml
    sed -i 's/create_empty_blocks = true/create_empty_blocks = false/' /root/.nymd/config/config.toml
    sed -i 's/laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/' /root/.nymd/config/config.toml
    yes "${PASSPHRASE}" | ./nymd keys add node_admin 2>&1 >/dev/null | tail -n 1 > /genesis_volume/genesis_mnemonic
    ADDRESS=$(yes "${PASSPHRASE}" | ./nymd keys show node_admin -a)
    yes "${PASSPHRASE}" | ./nymd add-genesis-account "${ADDRESS}" 1000000000000000u${BECH32_PREFIX},1000000000000000stake
    yes "${PASSPHRASE}" | ./nymd gentx node_admin 1000000000stake --chain-id nymnet 2> /dev/null
    ./nymd collect-gentxs 2> /dev/null
    ./nymd validate-genesis > /dev/null
    cp /root/.nymd/config/genesis.json /genesis_volume/genesis.json
  else
    echo "Validator already initialized, starting with the existing configuration."
    echo "If you want to re-init the validator, destroy the existing container"
	fi
	./nymd start
elif [ "$1" = "secondary" ]; then
  if [ ! -d "/root/.nymd" ]; then
    ./nymd init nymnet --chain-id nym-secondary 2> /dev/null

    # Wait until the genesis node writes the genesis.json to the shared volume
    while ! [ -s /genesis_volume/genesis.json ]; do
      sleep 1
    done

    cp /genesis_volume/genesis.json /root/.nymd/config/genesis.json
    GENESIS_PEER=$(cat /root/.nymd/config/genesis.json | grep '"memo"' | cut -d'"' -f 4)
    sed -i 's/persistent_peers = ""/persistent_peers = "'"${GENESIS_PEER}"'"/' /root/.nymd/config/config.toml
    sed -i 's/minimum-gas-prices = ""/minimum-gas-prices = "0.025u'"${BECH32_PREFIX}"'"/' /root/.nymd/config/app.toml
    sed -i '0,/enable = false/s//enable = true/g' /root/.nymd/config/app.toml
    sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = \["*"\]/' /root/.nymd/config/config.toml
    sed -i 's/create_empty_blocks = true/create_empty_blocks = false/' /root/.nymd/config/config.toml
    sed -i 's/laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/' /root/.nymd/config/config.toml
    yes "${PASSPHRASE}" | ./nymd keys add node_admin 2> mnemonic > /dev/null
    ./nymd validate-genesis > /dev/null
  else
    echo "Validator already initialized, starting with the existing configuration."
    echo "If you want to re-init the validator, destroy the existing container"
  fi
	./nymd start
else
	echo "Wrong command. Usage: ./$0 [genesis/secondary]"
fi
