#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# this is a script called by the github CI and CD workflows to build all 3 docs projects
# and move them to /dist/ in the root of the monorepo. They are rsynced to various servers
# from there by subsequent workflow tasks.

# array of project dirs
declare -a projects=("docs" "dev-portal" "operators")

# check you're calling from the right place
if [ $(pwd | awk -F/ '{print $NF}') != "documentation" ]
then
  echo "failure: please run script from documentation/"
else
   for i in "${projects[@]}"
   do
      # cd to project dir
      cd "./$i" &&
      # little sanity checks
      echo $(pwd) && echo $(mdbook --version) &&
      # clean old book
      echo "cleaning old book"
      rm -rf ./book/
      # build book
      mdbook test || true
      mdbook build
      # check for destination, if ! then mkdir & check again else echo thumbs up
      if [ ! -d ../../dist/docs/$i ]; then
        echo "dest doesn't exist: creating dir"
        mkdir -p ../../dist/docs/$i
      fi
      if [ -d ../../dist/docs/$i ]; then
        echo "cp destination exists, all good"
      fi
      # clean old dist/$i
      rm -rf ../../dist/docs/$i
      # move newly rendered book/ to dist
      rsync -r ./book/html/ ../../dist/docs/$i
      # sanity check
      ls -laF ../../dist/docs/
      # cd back to ../documentation/
      cd ../
   done
   # rename for server paths
   rm -rf ../dist/docs/developers
   mv ../dist/docs/dev-portal ../dist/docs/developers
fi
