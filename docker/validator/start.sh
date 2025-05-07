#!/bin/sh

export LD_LIBRARY_PATH="$LD_LIBRARY_PATH":/home/nym
PASSPHRASE=passphrase
APP_NAME=nyxd
OUTPUT_DIRECTORY="/home/nym/output"
VALIDATOR_DATA_DIRECTORY="/home/nym/.${APP_NAME}"

if [ ! -f "${VALIDATOR_DATA_DIRECTORY}/config/genesis.json" ]; then
  # initialise the validator
  ./${APP_NAME} init "${CHAIN_ID}" --chain-id "${CHAIN_ID}" 2>/dev/null

  echo "init chain successful"
  sleep 2

  # staking/governance token is hardcoded in config
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

  # create accounts
  echo "adding parent mnemonic account details"
  yes "${PASSPHRASE}" | ./${APP_NAME} keys add node_admin 2>&1 >/dev/null | tail -n 1 >${OUTPUT_DIRECTORY}/genesis_mnemonic
  
  # Add secondary account
  yes "${PASSPHRASE}" | ./${APP_NAME} keys add secondary 2>&1 >/dev/null | tail -n 1 >${OUTPUT_DIRECTORY}/secondary_mnemonic

  # add genesis accounts with some initial tokens
  echo "adding genesis account details"
  GENESIS_ADDRESS=$(yes "${PASSPHRASE}" | ./${APP_NAME} keys show node_admin -a)
  SECONDARY_ADDRESS=$(yes "${PASSPHRASE}" | ./${APP_NAME} keys show secondary -a)
  yes "${PASSPHRASE}" | ./${APP_NAME} genesis add-genesis-account "${GENESIS_ADDRESS}" 1000000000000000u"${DENOM}",1000000000000000u"${STAKE_DENOM}"
  yes "${PASSPHRASE}" | ./${APP_NAME} genesis add-genesis-account "${SECONDARY_ADDRESS}" 1000000000000000u"${DENOM}",1000000000000000u"${STAKE_DENOM}"

  echo "adding gentx time :)"
  yes "${PASSPHRASE}" | ./${APP_NAME} genesis gentx node_admin 100000000000u"${STAKE_DENOM}" --chain-id "${CHAIN_ID}" 2>/dev/null
  ./${APP_NAME} genesis collect-gentxs 2>/dev/null
  ./${APP_NAME} genesis validate-genesis >/dev/null

  # copy genesis file
  cp "${VALIDATOR_DATA_DIRECTORY}/config/genesis.json" "${OUTPUT_DIRECTORY}/genesis.json"
fi

./${APP_NAME} start