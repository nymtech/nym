#!/bin/bash

set -e

. assert.sh -v -x

PWD="../"
RELEASE_DIRECTORY="target/release"
VERSION_NUMBER=$1
WALLET_ADDRESS_CONST=n1435n84se65tn7yv536am0sfvng4yyrwj7thhxr
MOCK_HOST="1.2.3.4"
OUTPUT=$(for i in {1..8}; do echo -n $(( $RANDOM % 10 )); done)
ID="test-${OUTPUT}"

echo "the version number is ${VERSION_NUMBER}"

cd ${PWD}${RELEASE_DIRECTORY}

if [ -f nym-gateway ]; then
  echo "running init tests"
  # we wont use config env files for now
  # unless we want to use a specific environment
  OUTPUT=$(./nym-gateway --output json init --id ${ID} --host ${MOCK_HOST} --wallet-address ${WALLET_ADDRESS_CONST}) > /dev/null 2>&1
  
  # get jq values for things we can assert against
  VALUE=$(echo ${OUTPUT} | jq .wallet_address)
  VALUE=${VALUE#\"}
  VALUE=${VALUE%\"}
  
  # do asserts here based upon the output on init

  assert $(cat ${VALUE}) $(echo ${WALLET_ADDRESS_CONST})
  assert_end nym-gateway-tests
else
  echo "exting test no binary found"
fi


