## Stake Adjustment Program (`stake_adjustment.py`)

### Overview

This simple argument based program is designed primarily for Delegation program management.
The main goal is to generate a `.csv` which can also be reused for `nym-cli` as input:
```sh
./nym-cli mixnet delegators delegate-multi --mnemonic "<MNEMONIC>" --input <PATH>/<FILE>.csv
```

The default values therefore are:
`--wallet_address`: Nym Team DP wallet address
`--saturation`: 250k NYM
`--stake_cap`: 90% as per DP rules
`--adjustment_step`: 25k NYM as per DP rules
`--max_wallet_delegation`: 125k NYM as per DP rules
`--denom`: NYM not uNYM to make it smoother and aligned with delegate-multi command of nym-cli

### Install and Run

1. Download from this branch and make executable
```sh
wget https://raw.githubusercontent.com/nymtech/nym/refs/heads/feature/operators/delegation-program-adjuster/scripts/delegation-program/stake_adjustment.py && chmod u+x stake_adjustment.py 
```
2. Make a simple column csv with `node_ids` (for DP, use all the DP nodes `node_id` column, and save it to the same dir, example can be `input.csv`:
```csv
# example
1398
1365
2464
1423
1269
1870
1824
1707
```
3. Run the program with the input arg and (for DP use the dedfault values):
```sh
./stake_adjustment.py input.csv
```

4. The output is a csv with a few useful columns
5. For DP extract the first two columns with `NODE_ID` and `SUGGESTED_WALLET_DELEGATION` without the headers and save them as a `dp_update.csv`
6. Run this `dp_update.csv` as an input with `nym-cli` to adjust the DP delegations:
```sh
./nym-cli mixnet delegators delegate-multi --mnemonic "<MNEMONIC>" --input dp_update.csv
```
