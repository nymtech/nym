#!/bin/sh

# Wait for the mnemonic(s) to be generated
while ! [ -s /genesis_volume/genesis_mnemonic ]; do
        sleep 1
done

while ! [ -s /genesis_volume/secondary_mnemonic ]; do
        sleep 1
done

echo "This is the current genesis mnemonic:"
cat /genesis_volume/genesis_mnemonic

echo "This is the current secondary mnemonic:"
cat /genesis_volume/secondary_mnemonic
