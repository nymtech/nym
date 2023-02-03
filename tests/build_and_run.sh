#!/bin/bash

PWD="../"
GIT_BRANCH=$1
VERSION_NUMBER=$2

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

./nym-mixnode-binary-check.sh "$VERSION_NUMBER"

sleep 2 

echo "running gateway binary check"
#./nym-gateway-binary-check.sh "$VERSION_NUMBER"