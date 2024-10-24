```sh
Start this nym-node

Usage: nym-node run [OPTIONS]

Options:
      --id <ID>
          Id of the nym-node to use [env: NYMNODE_ID=] [default: default-nym-node]
      --config-file <CONFIG_FILE>
          Path to a configuration file of this node [env: NYMNODE_CONFIG=]
      --accept-operator-terms-and-conditions
          Explicitly specify whether you agree with the terms and conditions of a nym node operator as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0> [env: NYMNODE_ACCEPT_OPERATOR_TERMS=]
      --deny-init
          Forbid a new node from being initialised if configuration file for the provided specification doesn't already exist [env: NYMNODE_DENY_INIT=]
      --init-only
          If this is a brand new nym-node, specify whether it should only be initialised without actually running the subprocesses [env: NYMNODE_INIT_ONLY=]
      --local
          Flag specifying this node will be running in a local setting [env: NYMNODE_LOCAL=]
      --mode <MODE>
          Specifies the current mode of this nym-node [env: NYMNODE_MODE=] [possible values: mixnode, entry-gateway, exit-gateway]
  -w, --write-changes
          If this node has been initialised before, specify whether to write any new changes to the config file [env: NYMNODE_WRITE_CONFIG_CHANGES=]
      --bonding-information-output <BONDING_INFORMATION_OUTPUT>
          Specify output file for bonding information of this nym-node, i.e. its encoded keys. NOTE: the required bonding information is still a subject to change and this argument should be treated only as a preview of future features [env: NYMNODE_BONDING_INFORMATION_OUTPUT=]
  -o, --output <OUTPUT>
          Specify the output format of the bonding information (`text` or `json`) [env: NYMNODE_OUTPUT=] [default: text] [possible values: text, json]
      --public-ips <PUBLIC_IPS>
          Comma separated list of public ip addresses that will be announced to the nym-api and subsequently to the clients. In nearly all circumstances, it's going to be identical to the address you're going to use for bonding [env: NYMNODE_PUBLIC_IPS=]
      --hostname <HOSTNAME>
          Optional hostname associated with this gateway that will be announced to the nym-api and subsequently to the clients [env: NYMNODE_HOSTNAME=]
      --location <LOCATION>
          Optional **physical** location of this node's server. Either full country name (e.g. 'Poland'), two-letter alpha2 (e.g. 'PL'), three-letter alpha3 (e.g. 'POL') or three-digit numeric-3 (e.g. '616') can be provided [env: NYMNODE_LOCATION=]
      --http-bind-address <HTTP_BIND_ADDRESS>
          Socket address this node will use for binding its http API. default: `0.0.0.0:8080` [env: NYMNODE_HTTP_BIND_ADDRESS=]
      --landing-page-assets-path <LANDING_PAGE_ASSETS_PATH>
          Path to assets directory of custom landing page of this node [env: NYMNODE_HTTP_LANDING_ASSETS=]
      --http-access-token <HTTP_ACCESS_TOKEN>
          An optional bearer token for accessing certain http endpoints. Currently only used for prometheus metrics [env: NYMNODE_HTTP_ACCESS_TOKEN=]
      --expose-system-info <EXPOSE_SYSTEM_INFO>
          Specify whether basic system information should be exposed. default: true [env: NYMNODE_HTTP_EXPOSE_SYSTEM_INFO=] [possible values: true, false]
      --expose-system-hardware <EXPOSE_SYSTEM_HARDWARE>
          Specify whether basic system hardware information should be exposed. default: true [env: NYMNODE_HTTP_EXPOSE_SYSTEM_HARDWARE=] [possible values: true, false]
      --expose-crypto-hardware <EXPOSE_CRYPTO_HARDWARE>
          Specify whether detailed system crypto hardware information should be exposed. default: true [env: NYMNODE_HTTP_EXPOSE_CRYPTO_HARDWARE=] [possible values: true, false]
      --mixnet-bind-address <MIXNET_BIND_ADDRESS>
          Address this node will bind to for listening for mixnet packets default: `0.0.0.0:1789` [env: NYMNODE_MIXNET_BIND_ADDRESS=]
      --nym-api-urls <NYM_API_URLS>
          Addresses to nym APIs from which the node gets the view of the network [env: NYMNODE_NYM_APIS=]
      --nyxd-urls <NYXD_URLS>
          Addresses to nyxd chain endpoint which the node will use for chain interactions [env: NYMNODE_NYXD=]
      --wireguard-enabled <WIREGUARD_ENABLED>
          Specifies whether the wireguard service is enabled on this node [env: NYMNODE_WG_ENABLED=] [possible values: true, false]
      --wireguard-bind-address <WIREGUARD_BIND_ADDRESS>
          Socket address this node will use for binding its wireguard interface. default: `0.0.0.0:51822` [env: NYMNODE_WG_BIND_ADDRESS=]
      --wireguard-private-ip <WIREGUARD_PRIVATE_IP>
          Private IP address of the wireguard gateway. default: `10.1.0.1` [env: NYMNODE_WG_IP=]
      --wireguard-announced-port <WIREGUARD_ANNOUNCED_PORT>
          Port announced to external clients wishing to connect to the wireguard interface. Useful in the instances where the node is behind a proxy [env: NYMNODE_WG_ANNOUNCED_PORT=]
      --wireguard-private-network-prefix <WIREGUARD_PRIVATE_NETWORK_PREFIX>
          The prefix denoting the maximum number of the clients that can be connected via Wireguard. The maximum value for IPv4 is 32 and for IPv6 is 128 [env: NYMNODE_WG_PRIVATE_NETWORK_PREFIX=]
      --verloc-bind-address <VERLOC_BIND_ADDRESS>
          Socket address this node will use for binding its verloc API. default: `0.0.0.0:1790` [env: NYMNODE_VERLOC_BIND_ADDRESS=]
      --entry-bind-address <ENTRY_BIND_ADDRESS>
          Socket address this node will use for binding its client websocket API. default: `0.0.0.0:9000` [env: NYMNODE_ENTRY_BIND_ADDRESS=]
      --announce-ws-port <ANNOUNCE_WS_PORT>
          Custom announced port for listening for websocket client traffic. If unspecified, the value from the `bind_address` will be used instead [env: NYMNODE_ENTRY_ANNOUNCE_WS_PORT=]
      --announce-wss-port <ANNOUNCE_WSS_PORT>
          If applicable, announced port for listening for secure websocket client traffic [env: NYMNODE_ENTRY_ANNOUNCE_WSS_PORT=]
      --enforce-zk-nyms <ENFORCE_ZK_NYMS>
          Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet or if it also accepts non-paying clients [env: NYMNODE_ENFORCE_ZK_NYMS=] [possible values: true, false]
      --mnemonic <MNEMONIC>
          Custom cosmos wallet mnemonic used for zk-nym redemption. If no value is provided, a fresh mnemonic is going to be generated [env: NYMNODE_MNEMONIC=]
      --upstream-exit-policy-url <UPSTREAM_EXIT_POLICY_URL>
          Specifies the url for an upstream source of the exit policy used by this node [env: NYMNODE_UPSTREAM_EXIT_POLICY=]
      --open-proxy <OPEN_PROXY>
          Specifies whether this exit node should run in 'open-proxy' mode and thus would attempt to resolve **ANY** request it receives [env: NYMNODE_OPEN_PROXY=] [possible values: true, false]
  -h, --help
          Print help
```
