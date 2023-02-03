#!/bin/bash

. assert.sh -v -x

PWD="../"
RELEASE_DIRECTORY="target/release"
RELEASE_VERSION_NUMBER=$1
WALLET_ADDRESS_CONST=n1435n84se65tn7yv536am0sfvng4yyrwj7thhxr
MOCK_HOST="1.2.3.4"
OUTPUT=$(for i in {1..8}; do echo -n $(($RANDOM % 10)); done)
ID="test-${OUTPUT}"
BINARY_NAME="nym-mixnode"

echo "the version number is ${VERSION_NUMBER} to be installed from github"

# steps
# we curl the existing binary from the release page of github
# we init the binary to check successful init
# then in our testing branch, we build the binary locally
# re run the init based upon the configuration injected
# we validate that no errors are return from upgrading the binary against the test

# install the current release binary
# so this is dependant on running on a linux machine for the time being
curl -L "https://github.com/nymtech/nym/releases/download/nym-binaries-${RELEASE_VERSION_NUMBER}/${BINARY_NAME}" -o nym-mixnode
chmod u+x "$BINARY_NAME"

#--------------------------------------
# functions
#--------------------------------------
check_mixnode_binary_build() {
  if [ -f "$BINARY_NAME" ]; then
    echo "running init tests"
    # we wont use config env files for now
    OUTPUT=$(./${BINARY_NAME} --output json init --id ${ID} --host ${MOCK_HOST} --wallet-address ${WALLET_ADDRESS_CONST})
    # get jq values for things we can assert against
    # tidy this bit up - okay for first push

    VALUE="$(echo ${OUTPUT} | jq .wallet_address | tr -d '"')"

    # do asserts here based upon the output on init
    assert "echo ${VALUE}" $(echo ${WALLET_ADDRESS_CONST})
    assert_end nym-mixnode-tests
  else
    echo "exiting test no binary found"
  fi
}
# this is dependant on where it runs on ci potentially, will need to tweak this in the future
PATH_TO_ORIGIN_INIT_CONFIG="$(cat /root/.nym/mixnodes/${ID}/config/config.toml)"

#-------------------------------
# tests
#-------------------------------

# we run the release version first
check_mixnode_binary_build
#lets remove the binary then navigate to the target/release directory for checking the latest version
if [ -f "$BINARY_NAME" ]; then
  echo "removing nym-mixnode"
  rm -rf "$BINARY_NAME"
  echo "successfully removed nym-mixnode"
else
  echo "no binary found exiting"
  exit 1
fi
# we should expect it to pass because no errors should be presented when performing the upgrade of an init
# this should be caught at testing stage
# navigate to latest binary build
cd $PWD$RELEASE_DIRECTORY
#re run against the current binary built locally
check_mixnode_binary_build

PATH_TO_SECOND_ORIGIN_INIT_CONFIG="$(cat /root/.nym/mixnodes/${ID}/config/config.toml)"

# compare the files
echo "$PATH_TO_ORIGIN_INIT_CONFIG" | diff - "$PATH_TO_SECOND_ORIGIN_INIT_CONFIG" >/dev/null

# check the status of the diff
if [ $? -eq 0 ]; then
  echo "no differences in config files, exiting script"
  exit 0
else
  echo "there are differences in the config files, it may require a fresh init on the binary"
  exit 1
fi
