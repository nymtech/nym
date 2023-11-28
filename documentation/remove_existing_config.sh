#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# Script to remove existing ~'/.nym/ files on docs deployment. Used to avoid issues with `mdbook-cmdrun` output when
# e.g. erroring about overwriting existing keys. `mdbook-cmdrun` output for the moment has to be checked manually.

DIR=~/.nym

# check for config directory
if [ ! -d $DIR ]; then
  echo "config dir doesn't exist: nothing to do"
else
  echo "config dir exists - deleting"
  rm -rf $DIR
  # check exit code of rm -rf - if !0 then exit
  if [ $? -ne 0 ]; then
    echo "exit code was $?. looks like the something went wrong with deleting the directory"
    exit 1
  fi
fi