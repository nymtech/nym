#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

cd ../scripts &&
python csv2md.py -s 1 ../docs/data/csv/variables.csv > ../docs/components/outputs/csv2md-outputs/variables.md &&
python csv2md.py -s 0 ../docs/data/csv/isp-sheet.csv > ../docs/components/outputs/csv2md-outputs/isp-sheet.md &&
cd cmdrun &&
./nyx-percent-stake.sh > ../../docs/components/outputs/nyx-outputs/nyx-percent-stake.md &&
./nyx-total-stake.sh > ../../docs/components/outputs/nyx-outputs/nyx-total-stake.md &&
cd ../api-scraping &&
python api_targets.py time_now > ../../docs/components/outputs/api-scraping-outputs/time-now.md &&
echo "prebuild finished"
