#!/bin/bash

PWD="../"
GIT_BRANCH=$1
VERSION_NUMBER=$2

# run the script from the correct location 

if [[ $(pwd) != */tests ]]; then
    echo "Please run the script from the 'tests' directory."
    exit 1
fi

# lets make sure the branch is up to date
# ---------------------------------------
git checkout develop 
git fetch origin
git checkout $GIT_BRANCH
git pull origin $GIT_BRANCH
# ---------------------------------------

echo "working directory ${PWD}"

#build all binaries...
#expect the cargo tool chain to be installed on the machine
cargo build --release --all

#here there should be the applicable binaries to test inits
echo "running mixnode binary check"
./nym-mixnode-binary-check.sh 

sleep 2 

echo "running gateway binary check"
./nym-gateway-binary-check.sh 

sleep 2 

echo "running socks-5 binary check"
./nym-socks-5-binary-check.sh 

sleep 2 

echo "running network-requester binary check"
./nym-network-requester-binary-check.sh 

sleep 2 

echo "running client binary check"
./nym-client-binary-check.sh 


