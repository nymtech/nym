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

cargo build --release --manifest-path nym-client/Cargo.toml
cargo build --release --manifest-path mixnode/Cargo.toml
cargo build --release --manifest-path sfw-provider/Cargo.toml

#
## london
#Host testnet-Mix-Layer1.nym
#    Hostname 213.52.129.218
#    User root
#
## frankfurt
#Host testnet-Mix-Layer2.nym
#    Hostname 172.104.244.117
#    User root
#
## london
#Host testnet-Mix-Layer3.nym
#    Hostname 178.79.136.231
#    User root
#

for ((J = 1; J < 4; J++)); do
  LAYER=$J

  REMOTE_IP=$(ssh root@testnet-Mix-Layer$LAYER.nym 'echo $(LANG=c ifconfig eth0 | grep "inet " | awk -F: '\''{print $1}'\'' | awk '\''{print $2}'\'')')
  echo "Deploying to $REMOTE_IP..."

  echo "Killing existing process on server..."
  ssh "root@testnet-Mix-Layer$LAYER.nym" 'pkill -f "./nym-mixnode"'

  echo "Uploading to remote server..."
  scp "$PWD/target/release/nym-mixnode" "root@testnet-Mix-Layer$LAYER.nym:/root"

  echo "Initializing node data on remote server..."
  ssh "root@testnet-Mix-Layer$LAYER.nym" "./nym-mixnode init --id testnet-mix-node --layer $LAYER --host $REMOTE_IP"

  echo "Starting API on remote server..."
  ssh -f "root@testnet-Mix-Layer$LAYER.nym" "sh -c './nym-mixnode run --id testnet-mix-node > /dev/null 2>&1 &'"

  echo "root@testnet-Mix-Layer$LAYER deployed."
done

#
#for ((K = 0; K < NUMPROVIDERS; K++)); do
#  REMOTE_IP=$(ssh root@QA-Mix-Provider$K.nym 'echo $(LANG=c ifconfig eth0 | grep "inet " | awk -F: '\''{print $1}'\'' | awk '\''{print $2}'\'')')
#  echo "Deploying to $REMOTE_IP..."
#
#  echo "Killing existing process on server..."
#  ssh "root@QA-Mix-Provider$K.nym" 'pkill -f "./nym-sfw-provider"'
#
#  echo "Removing old inboxes..."
#  ssh "root@QA-Mix-Provider$K.nym" 'rm -rf inboxes'
#
#  echo "Uploading to remote server..."
#  scp "$PWD/target/release/nym-sfw-provider" "root@QA-Mix-Provider$K.nym:/root"
#
#  echo "Initializing node data on remote server..."
#  ssh "root@QA-Mix-Provider$K.nym" "./nym-sfw-provider init --id qa-sfw-provider --clients-host $REMOTE_IP --mix-host $REMOTE_IP"
#
#  echo "Starting API on remote server..."
#  ssh -f "root@QA-Mix-Provider$K.nym" "sh -c  './nym-sfw-provider run --id qa-sfw-provider  > /dev/null 2>&1 &'"
#
#  echo "QA-Mix-Provider$K deployed."
#done
