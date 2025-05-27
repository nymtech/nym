#!/bin/sh

set -e

/usr/src/nym/target/release/nym-api init --mnemonic "$MNEMONIC" && /usr/src/nym/target/release/nym-api run
