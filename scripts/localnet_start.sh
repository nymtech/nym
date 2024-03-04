#!/usr/bin/env bash

set -o errexit

# check that current folder ends with scripts
current_folder=$(basename "$(pwd)")

# Check if the current folder ends with "scripts"
if [[ $current_folder != *scripts ]]; then
  echo "Please run this from the 'scripts' folder"
  exit
fi

# can't just use `mktemp` since syntax differs between linux and macos (thx apple)
suffix=$(openssl rand -base64 10 | tr -dc 'a-zA-Z0-9')
localnetdir="$HOME/.nym/localnets/localnet.$suffix"
mkdir -p "$localnetdir"

echo "Using $localnetdir for the localnet"

# initialise mixnet
echo "initialising mixnode1..."
cargo run --release --bin nym-mixnode -- init --id "mix1-$suffix" --host 127.0.0.1 --mix-port 10001 --verloc-port 20001 --http-api-port 30001 --output=json >>"$localnetdir/mix1.json"

echo "initialising mixnode2..."
cargo run --release --bin nym-mixnode -- init --id "mix2-$suffix" --host 127.0.0.1 --mix-port 10002 --verloc-port 20002 --http-api-port 30002 --output=json >>"$localnetdir/mix2.json"

echo "initialising mixnode3..."
cargo run --release --bin nym-mixnode -- init --id "mix3-$suffix" --host 127.0.0.1 --mix-port 10003 --verloc-port 20003 --http-api-port 30003 --output=json >>"$localnetdir/mix3.json"

echo "initialising gateway..."
cargo run --release --bin nym-gateway -- init --id "gateway-$suffix" --host 127.0.0.1 --mix-port 10004 --clients-port 9000 --output=json >>"$localnetdir/gateway.json"

# build the topology
echo "combining json files..."
python3 build_topology.py "$localnetdir"

networkfile=$localnetdir/network.json
echo "the full network file is located at $networkfile"

# start up the mixnet
echo "starting the mixnet..."
tmux start-server

tmux new-session -d -s localnet -n Mixnet -d "/usr/bin/env sh -c \" cargo run --release --bin nym-mixnode -- run --id mix1-$suffix \""
tmux split-window -t localnet:0 "/usr/bin/env sh -c \" cargo run --release --bin nym-mixnode -- run --id mix2-$suffix \""
tmux split-window -t localnet:0 "/usr/bin/env sh -c \" cargo run --release --bin nym-mixnode -- run --id mix3-$suffix \""
tmux split-window -t localnet:0 "/usr/bin/env sh -c \" cargo run --release --bin nym-gateway -- run --id gateway-$suffix --local \""

while ! nc -z localhost 9000; do
  echo "waiting for nym-gateway to launch on port 9000..."
  sleep 2
done

echo "nym-gateway launched"

# initialise the clients
echo "initialising network requester..."
cargo run --release --bin nym-network-requester -- init --id "network-requester-$suffix" --open-proxy=true --custom-mixnet "$networkfile" --output=json >>"$localnetdir/network_requester.json"
address=$(jq -r .client_address "$localnetdir/network_requester.json")

echo "initialising socks5 client..."
cargo run --release --bin nym-socks5-client -- init --id "socks5-client-$suffix" --provider "$address" --custom-mixnet "$networkfile" --no-cover

# startup the clients
tmux new-window -t 1 -n 'Clients' -d "/usr/bin/env sh -c \" cargo run --release --bin nym-network-requester -- run --id network-requester-$suffix --custom-mixnet $networkfile \"; /usr/bin/env sh -i"
tmux split-window -t localnet:1 "/usr/bin/env sh -c \" cargo run --release --bin nym-socks5-client -- run --id socks5-client-$suffix --custom-mixnet $networkfile \"; /usr/bin/env sh -i"
tmux split-window -t localnet:1

# prepare the command to test the socks5
tmux send-keys -t localnet:1 "time curl -x socks5h://127.0.0.1:1080 https://test-download-files-nym.s3.amazonaws.com/download-files/1MB.zip --output /dev/null 2>&1"

tmux select-layout -t localnet:0 tiled
tmux select-layout -t localnet:1 tiled

tmux attach -t localnet
