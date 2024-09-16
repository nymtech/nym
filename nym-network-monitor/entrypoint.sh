#!/bin/bash
# Takes timeout in seconds as the first argument, defaults to 60
# Takes number of users as the second argument, defaults to 10

set -ex

private_key=${1}
network=${2:-mainnet}
timeout=${3:-600}
users=${4:-10}

RUST_LOG=info nym-network-monitor --env envs/"${network}".env --host 127.0.0.1 --port 8080 --private-key "${private_key}" &
nnm_pid=$!

sleep 10

python -m locust -H http://127.0.0.1:8080 --processes 4 --autostart --autoquit 60 -u "${users}" -t "${timeout}"s &
locust_pid=$!

wait $locust_pid
kill -2 $nnm_pid

exit $?
