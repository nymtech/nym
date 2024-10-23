#!/bin/bash

stake_unyx=$(curl -s -L https://api.nymtech.net/cosmos/staking/v1beta1/pool | jq 'values["pool"]["bonded_tokens"]')
stake_unyx=$(python -c "print(int($stake_unyx))")
stake_nyx=$(python -c "print($stake_unyx / 1000000)")
voting288k_percent=$(python -c "print(288000 / $stake_nyx * 100)")
echo ${voting288k_percent:0:4}%
