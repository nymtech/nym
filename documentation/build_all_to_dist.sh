#!/usr/bin/env bash

# this is a script called by the github CI and CD workflows to build all 3 docs projects
# and move them to /dist/ in the root of the monorepo. They are rsynced to various servers
# from there by subsequent workflow tasks.

# array of project dirs
declare -a projects=("dev-portal" "docs" "operators") 

# check you're calling from the right place
if [ $(pwd | awk -F/ '{print $NF}') != "documentation" ]
then
  echo "failure: please run script from documentation/"
else
   for i in "${projects[@]}"
   do
      echo $i && 
      cd "./$i" && RUST_LOG=info mdbook build --dest-dir ../../dist/docs/$i/ && cd ../
   done
   # rename for server paths
   rm -rf ../dist/developers
   mv ../dist/docs/dev-portal ../dist/docs/developers
fi
