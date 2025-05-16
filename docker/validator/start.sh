#!/usr/bin/env bash

export LD_LIBRARY_PATH="$LD_LIBRARY_PATH":/root
PASSPHRASE=passphrase
APP_NAME=nyxd
OUTPUT_DIRECTORY="/root/output"
VALIDATOR_DATA_DIRECTORY="/root/.${APP_NAME}"

mkdir -p "${VALIDATOR_DATA_DIRECTORY}/config"
mkdir -p "${OUTPUT_DIRECTORY}"

if [ ! -f "${VALIDATOR_DATA_DIRECTORY}/config/genesis.json" ]; then
  # initialise the validator
  ./${APP_NAME} init "${CHAIN_ID}" --chain-id "${CHAIN_ID}" 2>/dev/null

  echo "init chain successful"
  sleep 5

  echo "checking config files:"
  ls -la ${VALIDATOR_DATA_DIRECTORY}/config/

  echo "changing params"
  sed -i "s/\"stake\"/\"u${STAKE_DENOM}\"/" "${VALIDATOR_DATA_DIRECTORY}/config/genesis.json"
  sed -i 's/minimum-gas-prices = "0stake"/minimum-gas-prices = "0.025u'"${DENOM}"'"/' "${VALIDATOR_DATA_DIRECTORY}/config/app.toml"
  sed -i '0,/enable = false/s//enable = true/g' "${VALIDATOR_DATA_DIRECTORY}/config/app.toml"
  if [ "$RETAIN_BLOCKS" = "no" ]; then
    # amending to say if min retain blocks should be set yes or no...
    sed -i 's/min-retain-blocks = 0/min-retain-blocks = 70000/' "${VALIDATOR_DATA_DIRECTORY}/config/app.toml"
  fi
  sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = \["*"\]/' "${VALIDATOR_DATA_DIRECTORY}/config/config.toml"
  sed -i 's/create_empty_blocks = true/create_empty_blocks = false/' "${VALIDATOR_DATA_DIRECTORY}/config/config.toml"
  sed -i 's/laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/' "${VALIDATOR_DATA_DIRECTORY}/config/config.toml"
  sed -i 's/address = "tcp:\/\/localhost:1317"/address = "tcp:\/\/0.0.0.0:1317"/' "${VALIDATOR_DATA_DIRECTORY}/config/app.toml"

  echo "params changed"

  echo "adding parent mnemonic account details"
  yes "${PASSPHRASE}" | ./${APP_NAME} keys add node_admin 2>&1 >/dev/null | tail -n 1 >${OUTPUT_DIRECTORY}/node_admin_mnemonic

  # add genesis accounts with some initial tokens
  echo "adding genesis account details"
  GENESIS_ADDRESS=$(yes "${PASSPHRASE}" | ./${APP_NAME} keys show node_admin -a)
  yes "${PASSPHRASE}" | ./${APP_NAME} genesis add-genesis-account "${GENESIS_ADDRESS}" 1000000000000000u"${DENOM}",1000000000000000u"${STAKE_DENOM}"

  echo "adding gentx time :)"
  yes "${PASSPHRASE}" | ./${APP_NAME} genesis gentx node_admin 100000000000u"${STAKE_DENOM}" --chain-id "${CHAIN_ID}" 2>/dev/null
  ./${APP_NAME} genesis collect-gentxs 2>/dev/null
  ./${APP_NAME} genesis validate-genesis >/dev/null

  # make a copy of the genesis file to the output directory
  cp "${VALIDATOR_DATA_DIRECTORY}/config/genesis.json" "${OUTPUT_DIRECTORY}/genesis.json"
fi

./${APP_NAME} start &
sleep 10

sleep infinity