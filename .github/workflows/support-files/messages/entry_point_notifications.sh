#!/usr/bin/env bash

# pass exit codes out to GitHub Actions
set -euxo pipefail

# change to the directory that contains this script
cd "${0%/*}"

# run the node script
node send_message.js