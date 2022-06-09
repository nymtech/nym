Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
SPDX-License-Identifier: Apache-2.0
-->

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