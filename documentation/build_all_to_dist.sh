#!/bin/bash

# commands assume you run script from `nym/documentation/`

# array of project dirs
declare -a projects=("docs" "dev-portal" "operators")

## now loop through the above array
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
mv ../../dist/docs/dev-portal ../../dist/docs/developers
