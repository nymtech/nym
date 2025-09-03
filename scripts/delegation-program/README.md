## Stake Adjustment Program (`stake_adjustment.py`)

### Overview

This simple argument based program is designed primarily for Delegation program management.

The main goal is to generate a csv of which first two columns without headers can be passed to nym-cli as input for a quick delegation adjustment, using this command:

```sh
./nym-cli mixnet delegators delegate-multi --mnemonic "<MNEMONIC>" --input <PATH>/<FILE>.csv
```

The default values are in sync with DP rules:

`--wallet_address`: Nym Team DP wallet address
`--saturation`: 250k NYM
`--stake_cap`: 90% as per DP rules
`--adjustment_step`: 25k NYM as per DP rules
`--max_wallet_delegation`: 125k NYM as per DP rules
`--denom`: NYM not uNYM to make it smoother and aligned with delegate-multi command of nym-cli

Additionaly the program scrapes [api.nym.spectredao.net/api/v1/nodes](https://api.nym.spectredao.net/api/v1/nodes) and [validator.nymtech.net/api/v1/nym-nodes/described](https://validator.nymtech.net/api/v1/nym-nodes/described) endpoints and returns a sheet with 20 values per eacvh node passed in the csv input.

The outcome is a table with these values:
NODE ID, SUGGESTED WALLET DELEGATION, CURRENT WALLET DELEGATION, SUGGESTED TOTAL STAKE,	CURRENT TOTAL STAKE, SUGGESTED SATURATION, CURRENT SATURATION, UPTIME, VERSION, T&C, BINARY, ROLE, WIREGUARD, IP ADDRESS, HOSTNAME, WSS PORT, MONIKER, IDENTITY KEY, BONDING WALLET, EXPLORER URL.

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
3. Run the program with the `input.csv` positional arg (for DP use the default values without any optional args):
```sh
./stake_adjustment.py input.csv
```
4. The output is a `csv` with bunch of useful data
5. For DP extract the first two columns with `NODE_ID` and `SUGGESTED_WALLET_DELEGATION` without the headers and save them as a `dp_update.csv`
6. Run this `dp_update.csv` as an input with `nym-cli` to adjust the DP delegations:
```sh
./nym-cli mixnet delegators delegate-multi --mnemonic "<MNEMONIC>" --input dp_update.csv
```
### Help

To preview all commands and arguments, run it with `--help` flag:
```sh
./stake_adjustment.py --help
```
