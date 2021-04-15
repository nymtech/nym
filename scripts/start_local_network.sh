# Copyright 2020 - The Nym Mixnet Authors 
# SPDX-License-Identifier: Apache-2.0

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

MAX_LAYERS=3
NUMMIXES=3

function kill_old() {
    echo "Killing old testnet processes..."
    killall nym-mixnode
    killall nym-gateway
    killall nym-client
}

if [ $# -ne 1 ]; then
    echo "Expected a single argument to be passed - the directory server (that you should have independently started locally!)"
    exit 1
fi

DIR=$1

echo "Press CTRL-C to stop."

kill_old
export RUST_LOG=warning
# NOTE: If we wanted to suppress stdout and stderr, replace `&` with `> /dev/null 2>&1 &` in the `run`

# cargo run --bin nym-gateway -- init --id gateway-local --mix-host 127.0.0.1:10000 --clients-host 127.0.0.1:10001 --directory $DIR
cargo run --release --bin nym-gateway -- run --id gateway-local &

sleep 1

# Note: to disable logging (or direct it to another output) modify the constant on top of mixnode or provider;
# Will make it later either configurable by flags or config file.
for (( j=0; j<$NUMMIXES; j++ )); do
    let layer=j%MAX_LAYERS+1
    cargo run --release --bin nym-mixnode -- init --id mix-local$j --host 127.0.0.1 --port $((9980+$j)) --layer $layer --directory $DIR
    cargo run --release --bin nym-mixnode -- run --id mix-local$j &
    sleep 1
done


# just run forever (so we'd get all network warnings in this window and you wouldn't get confused when you started another process here)
# also it seems that SIGINT is nicely passed to all processes so they kill themselves
tail -f /dev/null
