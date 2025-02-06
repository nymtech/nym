#!/bin/bash

./nym-node run --init-only

BOND_INFO=$(./nym-node bonding-information)
IDENTITY_KEY=$(grep -oP '(?<=Identity Key: ).*' <<<"$BOND_INFO")
SPHINX_KEY=$(grep -oP '(?<=Sphinx Key: ).*' <<<"$BOND_INFO")
VERSION=$(grep -oP '(?<=Version: ).*' <<<"$BOND_INFO" | sed 's/+.*//')

echo "Entering into signature signing..."
CONTRACT_MSG=$(./nym-cli --mnemonic "$NYMNODE_MNEMONIC" mixnet operators nymnode create-node-bonding-sign-payload --host "$NYMNODE_PUBLIC_IPS" --identity-key "$IDENTITY_KEY" --amount 100000000)
SIGNATURE=$(./nym-node sign --contract-msg "$CONTRACT_MSG" | grep -A1 'is:' | tail -n1 | sed 's/^\s*//')

echo "Starting the bond node flow..."
./nym-cli --mnemonic "$NYMNODE_MNEMONIC" mixnet operators nymnode bond --host "$NYMNODE_PUBLIC_IPS" --identity-key "$IDENTITY_KEY" --amount 100000000 --signature "$SIGNATURE"

./nym-node run --deny-init
