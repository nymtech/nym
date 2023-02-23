## Binary init checker

A simple tool to ensure that all binaries init with the correct format, using the `assert.sh` library

Simply run `./build_and_run.sh $RELEASE_BRANCH` `$CURRENT_PRODUCTION_RELEASE_VERSION`

For example:

`./build_and_run.sh release/v1.1.11 v1.1.10`

Currently, this is run on linux based machines as the nym-core binaries are published via a linux build agent


This will run through all the binaries and check the fields that we expect to be initialised when passing the parameters into nyms core binaries

## TODO
    - Create GH workflow and Run in CI
