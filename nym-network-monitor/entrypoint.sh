#!/bin/bash
# Takes timeout in seconds as the first argument, defaults to 60
# Takes number of users as the second argument, defaults to 10

set -ex

_private_key=${PRIVATE_KEY}
network=${NYM_NETWORK:-mainnet}
timeout=${LOCUST_TIMEOUT:-600}
users=${LOCUST_USERS:-10}
processes=${LOCUST_PROCESSES:-4}
_database_url=${DATABASE_URL}

RUST_LOG=info nym-network-monitor --env envs/"${network}".env --private-key "${_private_key}" --database-url "${_database_url}" &
nnm_pid=$!

sleep 10

python -m locust -H http://"${NYM_NETWORK_MONITOR_HOST}":"${NYM_NETWORK_MONITOR_PORT}" --processes "${processes}" --autostart --autoquit 60 -u "${users}" -t "${timeout}"s &
locust_pid=$!

wait $locust_pid
kill -2 $nnm_pid

exit $?
