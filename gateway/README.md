<!--
Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: GPL-3.0-only
-->

# Nym Gateway

A Rust gateway implementation.

## License

Copyright (C) 2023 Nym Technologies SA <contact@nymtech.net>

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

## Install debian

```bash
sudo curl -s --compressed "https://nymtech.github.io/nym/nymtech.gpg" | gpg --dearmor | sudo tee /etc/apt/trusted.gpg.d/nymtech.gpg > /dev/null
sudo echo "deb [signed-by=/etc/apt/trusted.gpg.d/nymtech.gpg] https://nymtech.github.io/nym/ /" > nymtech.list

sudo apt-get update
sudo apt-get install nym-gateway

# See below for starting and managing the node
```

## Systemd support

```bash
sudo systemctl enable nym-gateway

# Run 
sudo systemctl start nym-gateway

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