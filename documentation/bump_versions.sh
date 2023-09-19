#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# takes one manadatory arg and one optional arg: wallet release and minimum rust versions
# it then uses sed to bump them in the three book.toml files.
#
# e.g if the upcoming wallet release version was 1.2.9 you'd run this as:
# `./bump_versions.sh "1.2.9"`
#
# you can also set the minumum rust version by passing an optional additional argument:
# `./bump_versions.sh "1.2.9" "1.67"`

# array of project dirs
declare -a projects=("docs" "dev-portal" "operators")

# check number of args passed
if [ "$#" -lt 1 ] || [ "$#" -gt 2 ];
then
    echo "failure: please pass at least 1 and at most 2 args: "
    echo "./bump_version.sh <new wallet_release_version> [OPTIONAL]<new minimum_rust_version>"
    exit 0
fi

# check you're calling from the right place
if [ $(pwd | awk -F/ '{print $NF}') != "documentation" ]
then
  echo "failure: please run script from documentation/"
  exit 0
else
  ## now loop through the above array sed-ing the variable values in the book.toml files
  for i in "${projects[@]}"
  do
    # sed the vars in the book.toml file for each project
    echo "setting wallet version in $i/"
    sed -i 's/wallet_release_version =.*/wallet_release_version = "'$2'"/' "$i"/book.toml
    if [ "$3" ]
    then
      echo "setting minimum rust version in $i/"
      sed -i 's/minimum_rust_version = .*/minimum_rust_version = "'$3'"/' "$i"/book.toml
    fi
  done
fi
