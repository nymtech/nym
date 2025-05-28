#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

cd ../scripts &&
python csv2md.py -s 1 ../docs/data/csv/variables.csv > ../docs/components/outputs/csv2md-outputs/variables.md &&
python csv2md.py -s 0 ../docs/data/csv/isp-sheet.csv > ../docs/components/outputs/csv2md-outputs/isp-sheet.md &&

cd cmdrun &&
./nyx-percent-stake.sh > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/nyx-percent-stake.md &&
./nyx-total-stake.sh > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/nyx-total-stake.md &&

cd ../api-scraping &&
python api_targets.py validator --api mainnet --endpoint circulating-supply --value circulating_supply amount --separator _ > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/circulating-supply.md &&

python api_targets.py validator --api mainnet --endpoint circulating-supply --format markdown --separator _ > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/token-table.md &&

python api_targets.py validator --api mainnet --endpoint epoch/reward_params --value interval staking_supply_scale_factor --format percent > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/staking-scale-factor.md &&

python api_targets.py validator --api mainnet --endpoint epoch/reward_params --value interval stake_saturation_point --separator _ > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/stake-saturation.md &&

python api_targets.py validator --api mainnet --endpoint epoch/reward_params --value interval staking_supply --separator _ > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/staking_supply.md &&

python api_targets.py validator --api mainnet --endpoint epoch/reward_params --value interval epoch_reward_budget --format markdown --separator _ > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/epoch-reward-budget.md &&

python api_targets.py time_now > ../../docs/components/outputs/api-scraping-outputs/time-now.md &&

python api_targets.py calculate --staking_target --separator _ > ../../docs/components/outputs/api-scraping-outputs/nyx-outputs/staking-target.md &&

cd ../../../scripts &&
echo '```python' > ../documentation/docs/components/outputs/command-outputs/node-api-check-query-help.md &&
python node_api_check.py query_stats --help >> ../documentation/docs/components/outputs/command-outputs/node-api-check-query-help.md &&
echo '```' >> ../documentation/docs/components/outputs/command-outputs/node-api-check-query-help.md &&

echo '```python' > ../documentation/docs/components/outputs/command-outputs/node-api-check-help.md &&
python node_api_check.py --help >> ../documentation/docs/components/outputs/command-outputs/node-api-check-help.md &&
echo '```' >> ../documentation/docs/components/outputs/command-outputs/node-api-check-help.md &&

cd ../target/release/ &&
echo '```sh' > ../../documentation/docs/components/outputs/command-outputs/nym-node-help.md &&
./nym-node --help >> ../../documentation/docs/components/outputs/command-outputs/nym-node-help.md &&
echo '```' >> ../../documentation/docs/components/outputs/command-outputs/nym-node-help.md &&

echo '```sh' > ../../documentation/docs/components/outputs/command-outputs/nym-node-run-help.md &&
./nym-node run --help >> ../../documentation/docs/components/outputs/command-outputs/nym-node-run-help.md &&
echo '```' >> ../../documentation/docs/components/outputs/command-outputs/nym-node-run-help.md &&

echo '```sh' > ../../documentation/docs/components/outputs/command-outputs/nymvisor-help.md &&
./nymvisor --help >> ../../documentation/docs/components/outputs/command-outputs/nymvisor-help.md &&
echo '```' >> ../../documentation/docs/components/outputs/command-outputs/nymvisor-help.md &&

echo '```sh' > ../../documentation/docs/components/outputs/command-outputs/nym-api-help.md &&
./nym-api --help >> ../../documentation/docs/components/outputs/command-outputs/nym-api-help.md &&
echo '```' >> ../../documentation/docs/components/outputs/command-outputs/nym-api-help.md &&

echo "prebuild finished"
