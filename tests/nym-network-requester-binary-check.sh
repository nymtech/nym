#!/bin/bash

set -e

. assert.sh -v -x

PWD="../"
RELEASE_DIRECTORY="target/release"
VERSION_NUMBER=$1
RANDOM_ID=$(for i in {1..8}; do echo -n $(($RANDOM % 10)); done)
ID="test-${RANDOM_ID}"
BINARY_NAME="nym-network-requester"

echo "the version number is ${VERSION_NUMBER} to be installed from github"

cd ${PWD}${RELEASE_DIRECTORY}

# we have now the bundled the client into the network requester, more a less the same output as the client

curl -L "https://github.com/nymtech/nym/releases/download/nym-binaries-${RELEASE_VERSION_NUMBER}/${BINARY_NAME}" -o $BINARY_NAME
chmod u+x $BINARY_NAME

#----------------------------------------------------------------------------------------------------------
# functions
#----------------------------------------------------------------------------------------------------------

check_nym_network_requester_binary_build() if [ -f $BINARY_NAME ]; then
  echo "running init tests"
  ./${BINARY_NAME} init --id ${ID} --output-json >/dev/null 2>&1

  # currently this outputs to a file name name
  # we currently store the output in a file in the same directory

  if [ -f "client_init_results.json" ]; then
    OUTPUT=$(cat client_init_results.json)

    # get jq values for things we can assert against
    # until the service provider is provided in the output we can validate the id is correct on init
    VALUE=$(echo ${OUTPUT} | jq .id)
    VALUE=${VALUE#\"}
    VALUE=${VALUE%\"}

    echo "${OUTPUT}"
    sleep 2

    # do asserts here based upon the output on init

    assert $(echo ${VALUE}) $(echo ${ID})
    assert_end nym-network-requester-tests
  else
    echo "exting test no binary found"
  fi
else
  echo "exting test no binary found"
fi

#----------------------------------------------------------------------------------------------------------
# tests
#----------------------------------------------------------------------------------------------------------
# we run the release version first

check_nym_network_requester_binary_build

first_init=$(cat /root/.nym/service-providers/network-requester/${ID}/config/config.toml | grep -v "^version =")

#----------------------------------------------------------------------------------------------------------
# lets remove the binary then navigate to the target/release directory for checking the latest version
# expect to have successful output and configuration
#----------------------------------------------------------------------------------------------------------

if [ -f $BINARY_NAME ]; then
  echo "removing nym-network-requester binary"
  rm -rf $BINARY_NAME
else
  echo "no binary found exiting"
  exit 1
fi

#----------------------------------------------------------------------------------------------------------
# we should expect it to pass because no errors should be presented when performing the upgrade of an init
# this should be caught at testing stage - navigate to latest binary build
#----------------------------------------------------------------------------------------------------------

cd ${PWD}${RELEASE_DIRECTORY}

# re-run against the current binary built locally

echo "diff the config files after each init"
echo "-------------------------------------"

check_nym_network_requester_binary_build

second_init=$(cat /root/.nym/service-providers/network-requester/${ID}/config/config.toml | grep -v "^version =")

diff -w <(echo "$first_init") <(echo "$second_init")

# check the status of the diff
if [ $? -eq 0 ]; then
  echo "no differences in config files, exiting script"
  exit 0
else
  echo "there are differences in the config files, it may require a fresh init on the binary"
  exit 1
fi

# we should expect it to pass because no errors should be presented when performing the upgrade of an init
