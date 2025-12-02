```sh
usage: nym-node-cli install [-h] [-V] [-d BRANCH] [-v]
                            [--mode {mixnode,entry-gateway,exit-gateway}]
                            [--wireguard-enabled {true,false}]
                            [--hostname HOSTNAME] [--location LOCATION]
                            [--email EMAIL] [--moniker MONIKER]
                            [--description DESCRIPTION]
                            [--public-ip PUBLIC_IP]
                            [--nym-node-binary NYM_NODE_BINARY]
                            [--uplink-dev-v4 IPV4_UPLINK_DEV]
                            [--uplink-dev-v6 IPV6_UPLINK_DEV] [--env KEY=VALUE]

options:
  -h, --help            show this help message and exit
  -V, --version         show program's version number and exit
  -d BRANCH, --dev BRANCH
                        Define github branch (default: develop)
  -v, --verbose         Show full error tracebacks
  --mode {mixnode,entry-gateway,exit-gateway}
                        Node mode: 'mixnode', 'entry-gateway', or 'exit-
                        gateway'
  --wireguard-enabled {true,false}
                        WireGuard functionality switch: true / false
  --hostname HOSTNAME   Node domain / hostname
  --location LOCATION   Node location (country code or name)
  --email EMAIL         Contact email for the node operator
  --moniker MONIKER     Public moniker displayed in explorer & NymVPN app
  --description DESCRIPTION
                        Short public description of the node
  --public-ip PUBLIC_IP
                        External IPv4 address (autodetected if omitted)
  --nym-node-binary NYM_NODE_BINARY
                        URL for nym-node binary (autodetected if omitted)
  --uplink-dev-v4 IPV4_UPLINK_DEV
                        Override ipv4 uplink interface used for NAT/FORWARD (e.g.,
                        'eth0'; autodetected if omitted)
  --uplink-dev-v6 IPV6_UPLINK_DEV
                        Override ipv6 uplink interface used for NAT/FORWARD (e.g.,
                        'eth0.1'; autodetected if omitted)
  --env KEY=VALUE       (Optional) Extra ENV VARS, e.g. --env CUSTOM_KEY=value
```
