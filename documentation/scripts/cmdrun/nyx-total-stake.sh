#!/bin/bash

stake=$(curl -s -L https://api.nymtech.net/cosmos/staking/v1beta1/pool | jq 'values["pool"]["bonded_tokens"]')
echo ${stake:1:2}.${stake:3:3}
