Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: GPL-3.0-only
-->

## License

Copyright (C) 2022 Nym Technologies SA <contact@nymtech.net>

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

## Network requester

The network requester is used to interpret socks5 client messages that need to
be proxied to a running service i.e. a host and a port.

If you have a service that you want to expose to the mixnet, you'd need to
first run the native client and provide the client address to your users that
will use it in their socks5 configuration.

After starting the native client, start the network requester and configure it,
setting your service's endpoint in  
`${HOME}/.nym/service-providers/network-requester/allowed.list`

Running in `open-proxy` mode allows any traffic to be proxied by the network
requester.

### Statistics service
The network requester can be ran as a gatherer of statistics for all
the services it proxies. For that, run the binary with the
`enable-statistics` flag enabled. Anonymized statistics are then sent to
a central server, through the mixnet.