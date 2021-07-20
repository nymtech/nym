#!/bin/sh

# Wait for the mnemonic to be generated
while ! [ -s /genesis_volume/genesis_mnemonic ]; do
        sleep 1
done

echo "This is the current mnemonic:"
cat /genesis_volume/genesis_mnemonic
