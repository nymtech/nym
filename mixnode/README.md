<!--
Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: GPL-3.0-only
-->

# Nym Mixnode

A Rust mixnode implementation.

## License

Copyright (C) 2020 Nym Technologies SA <contact@nymtech.net>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.

## Usage

* `nym-mixnode` prints a help message showing usage options
* `nym-mixnode run --help` prints a help message showing usage options for the run command
* `nym-mixnode run --layer 1 --host x.x.x.x` will start the mixnode in layer 1 and bind to the specified host IP address. Coordinate with other people in your network to find out which layer needs coverage.

By default, the Nym Mixnode will start on port 1789. If desired, you can change the port using the `--port` option.

## Install debian

```bash
sudo curl -s --compressed "https://nymtech.github.io/nym/nymtech.gpg" | gpg --dearmor | sudo tee /etc/apt/trusted.gpg.d/nymtech.gpg > /dev/null
sudo echo "deb [signed-by=/etc/apt/trusted.gpg.d/nymtech.gpg] https://nymtech.github.io/nym/ /" > nymtech.list

sudo apt-get update
sudo apt-get install nym-mixnode

# See below for starting and managing the node
```

## Systemd support

```bash
sudo systemctl enable nym-mixnode

# Run 
sudo systemctl start nym-mixnode

# Check status
sudo systemctl status nym-mixnode

# Logs
journalctl -f -u nym-mixnode

```

## Build debian package

```bash 
# cargo install cargo-deb

# Build package
cargo deb -p nym-mixnode

# Install

# This will init the mixnode to `/etc/nym` as `nym` user, and create a systemd service
sudo dpkg -i target/debian/<PACKAGE>
```