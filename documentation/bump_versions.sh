#!/bin/bash
# this takes two args: platform release version and wallet release version.
# it then uses sed to bump them in the three book.toml files.
#
# e.g if the upcoming platform release was v1.1.29 and the release version 1.2.9 you'd run this as:
# `./bump_versions.sh "1.1.29" "1.2.9"`

# array of project dirs
declare -a projects=("docs" "dev-portal" "operators")

## now loop through the above array sed-ing the variable values in the book.toml files
for i in "${projects[@]}"
do
  # sed the vars in the book.toml file for each project
  sed -i 's/platform_release_version =.*/platform_release_version = "'$1'"/' "$i"/book.toml
  sed -i 's/wallet_release_version =.*/wallet_release_version = "'$2'"/' "$i"/book.toml
done