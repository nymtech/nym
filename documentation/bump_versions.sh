#!/bin/bash
# this takes two args: platform release version and wallet release version.
# it then uses sed to bump them in the three book.toml files.
#
# e.g if the upcoming platform release was v1.1.29 and the release version 1.2.9 you'd run this as:
# `./bump_versions.sh "1.1.29" "1.2.9"`
#
# you can also set the minumum rust version by passing an optional 3rd argument:
# `./bump_versions.sh "1.1.29" "1.2.9" "1.67"`

# array of project dirs
declare -a projects=("docs" "dev-portal" "operators")

# check number of args passed
if [ "$#" -lt 2 ] || [ "$#" -gt 3 ];
then
    echo "failure: please pass at least 2 and at most 3 args: "
    echo "./bump_version.sh <new platform_release_version> <new wallet_release_version> [OPTIONAL]<new minimum_rust_version>"
    exit 0
fi

# check you're calling from the right place
if [ $(pwd | awk -F/ '{print $NF}') != "documentation" ]
then
  echo "failure: please run script from documentation/"
else
  ## now loop through the above array sed-ing the variable values in the book.toml files
  for i in "${projects[@]}"
  do
    # sed the vars in the book.toml file for each project
    echo "setting platform and wallet versions in $i/"
    sed -i 's/platform_release_version =.*/platform_release_version = "'$1'"/' "$i"/book.toml
    sed -i 's/wallet_release_version =.*/wallet_release_version = "'$2'"/' "$i"/book.toml
    if [ "$3" ]
    then
      echo "setting minimum rust version in $i/"
      sed -i 's/minimum_rust_version = .*/minimum_rust_version = "'$3'"/' "$i"/book.toml
    fi
  done
fi
