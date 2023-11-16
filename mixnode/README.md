<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

# Nym Mixnode

A Rust mixnode implementation.

## Usage

* `nym-mixnode` prints a help message showing usage options
* `nym-mixnode run --help` prints a help message showing usage options for the run command
* `nym-mixnode run --layer 1 --host x.x.x.x` will start the mixnode in layer 1 and bind to the specified host IP address. Coordinate with other people in your network to find out which layer needs coverage.

By default, the Nym Mixnode will start on port 1789. If desired, you can change the port using the `--port` option.

## Build debian package

```bash 
# cargo install cargo-deb

# Build package
cargo deb -p nym-mixnode

# Install

# This will init the mixnode to `/etc/nym` as `nym` user, and create a systemd service
sudo dpkg -i target/debian/<PACKAGE>

# Run 
sudo systemctl start nym-mixnode

# Check status
sudo systemctl status nym-mixnode

# Logs
journalctl -f -u nym-mixnode

```