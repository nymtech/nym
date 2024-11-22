```sh
Attempt to migrate an existing mixnode or gateway into a nym-node

Usage: nym-node migrate [OPTIONS] <--id <ID>|--config-file <CONFIG_FILE>> <NODE_TYPE>

Arguments:
  <NODE_TYPE>  Type of node (mixnode or gateway) to migrate into a nym-node [possible values: mixnode, gateway]

Options:
      --id <ID>
          Id of the node that's going to get migrated
      --config-file <CONFIG_FILE>
          Path to a configuration file of the node that's going to get migrated
      --preserve-id
          Specify whether to preserve id of the imported node
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