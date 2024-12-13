#!/usr/bin/env bash

set -o errexit
set -o pipefail

echo 'calling from' $(pwd)

if [ ! -d ../../dist/docs/ ]; then
    # echo "dest doesn't exist: creating dir"
    mkdir -p ../../dist/docs/
fi

rsync -r  ./.next/server/*  ../../dist/docs/
