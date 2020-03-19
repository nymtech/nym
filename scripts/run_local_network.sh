#!/bin/bash

#// Copyright 2020 The Nym Mixnet Authors
#//
#// Licensed under the Apache License, Version 2.0 (the "License");
#// you may not use this file except in compliance with the License.
#// You may obtain a copy of the License at
#//
#//      http://www.apache.org/licenses/LICENSE-2.0
#//
#// Unless required by applicable law or agreed to in writing, software
#// distributed under the License is distributed on an "AS IS" BASIS,
#// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#// See the License for the specific language governing permissions and
#// limitations under the License.


echo "Killing old testnet processes..."

killall nym-mixnode
killall nym-sfw-provider

echo "Press CTRL-C to stop."
echo "Make sure you have started nym-directory !"

cargo build --release --manifest-path nym-client/Cargo.toml --features=local
cargo build --release --manifest-path mixnode/Cargo.toml --features=local
cargo build --release --manifest-path sfw-provider/Cargo.toml --features=local

MAX_LAYERS=3
NUMMIXES=${1:-3} # Set $NUMMIXES to default of 3, but allow the user to set other values if desired


$PWD/target/release/nym-sfw-provider init --id provider-local --clients-host 127.0.0.1 --mix-host 127.0.0.1 --mix-port 4000 --mix-announce-port 4000
$PWD/target/release/nym-sfw-provider run --id provider-local &

sleep 1

for (( j=0; j<$NUMMIXES; j++ ))

# Note: to disable logging (or direct it to another output) modify the constant on top of mixnode or provider;
# Will make it later either configurable by flags or config file.
do
    let layer=j%MAX_LAYERS+1
    $PWD/target/release/nym-mixnode init --id mix-local$j --host 127.0.0.1 --port $((9980+$j)) --layer $layer --announce-host 127.0.0.1:$((9980+$j))
    $PWD/target/release/nym-mixnode run --id mix-local$j &
    sleep 1
done


# trap call ctrl_c()
trap ctrl_c SIGINT SIGTERM SIGTSTP
function ctrl_c() {
        echo "** Trapped SIGINT, SIGTERM and SIGTSTP"
        for (( j=0; j<$NUMMIXES; j++ ));
        do
            kill_port $((9980+$j))
        done
}

function kill_port() {
    PID=$(lsof -t -i:$1)
    echo "$PID"
    kill -TERM $PID || kill -KILL $PID
}
