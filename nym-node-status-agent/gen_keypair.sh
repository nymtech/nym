#!/bin/bash

mkdir -p keys
cargo run --package nym-node-status-agent -- generate-keypair --path keys/private
