#!/bin/bash

#//
#// Licensed under the Apache License, Version 2.0 (the "License");
#// you may not use this file except in compliance with the License.
#// You may obtain a copy of the License at
#//
#// http://www.apache.org/licenses/LICENSE-2.0
#//
#// Unless required by applicable law or agreed to in writing, software
#// distributed under the License is distributed on an "AS IS" BASIS,
#// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#// See the License for the specific language governing permissions and
#// limitations under the License.

cargo build --release --manifest-path nym-client/Cargo.toml --features=qa
cargo build --release --manifest-path mixnode/Cargo.toml --features=qa
cargo build --release --manifest-path sfw-provider/Cargo.toml --features=qa

MAX_LAYERS=3
NUMMIXES=${1:-6} # Set $NUMMIXES to default of 3, but allow the user to set other values if desired
NUMPROVIDERS=1

for ((J = 0; J < NUMMIXES; J++)); do # Note: to disable logging (or direct it to another output) modify the constant on top of mixnode or provider;
  # Will make it later either configurable by flags or config file.
  if ((NUMMIXES > 6)); then
    echo "Maximum of 6 mixes allowed"
    exit 1
  fi
  LAYER=$((J % MAX_LAYERS + 1))

  REMOTE_IP=$(ssh root@QA-Mix$J-Layer$LAYER.nym 'echo $(LANG=c ifconfig eth0 | grep "inet " | awk -F: '\''{print $1}'\'' | awk '\''{print $2}'\'')')
  echo "Deploying to $REMOTE_IP..."

  echo "Killing existing process on server..."
  ssh "root@QA-Mix$J-Layer$LAYER.nym" 'pkill -f "./nym-mixnode"'

  echo "Uploading to remote server..."
  scp "$PWD/target/release/nym-mixnode" "root@QA-Mix$J-Layer$LAYER.nym:/root"

  echo "Initializing node data on remote server..."
  ssh "root@QA-Mix$J-Layer$LAYER.nym" "./nym-mixnode init --id qa-mix-node --layer $LAYER --host $REMOTE_IP"

  echo "Starting API on remote server..."
  ssh -f "root@QA-Mix$J-Layer$LAYER.nym" "sh -c './nym-mixnode run --id qa-mix-node > /dev/null 2>&1 &'"

  echo "QA-Mix$J-Layer$LAYER deployed."
done

for ((K = 0; K < NUMPROVIDERS; K++)); do
  REMOTE_IP=$(ssh root@QA-Mix-Provider$K.nym 'echo $(LANG=c ifconfig eth0 | grep "inet " | awk -F: '\''{print $1}'\'' | awk '\''{print $2}'\'')')
  echo "Deploying to $REMOTE_IP..."

  echo "Killing existing process on server..."
  ssh "root@QA-Mix-Provider$K.nym" 'pkill -f "./nym-sfw-provider"'

  echo "Removing old inboxes..."
  ssh "root@QA-Mix-Provider$K.nym" 'rm -rf inboxes'

  echo "Uploading to remote server..."
  scp "$PWD/target/release/nym-sfw-provider" "root@QA-Mix-Provider$K.nym:/root"

  echo "Initializing node data on remote server..."
  ssh "root@QA-Mix-Provider$K.nym" "./nym-sfw-provider init --id qa-sfw-provider --clients-host $REMOTE_IP --mix-host $REMOTE_IP"

  echo "Starting API on remote server..."
  ssh -f "root@QA-Mix-Provider$K.nym" "sh -c  './nym-sfw-provider run --id qa-sfw-provider  > /dev/null 2>&1 &'"

  echo "QA-Mix-Provider$K deployed."
done


#  ssh -f "root@139.162.211.184" "sh -c  './nym-client run --id client  > /dev/null 2>&1 &'"
