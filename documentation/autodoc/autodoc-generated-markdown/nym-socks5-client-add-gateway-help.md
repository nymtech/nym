```sh
Add new gateway to this client

Usage: nym-socks5-client add-gateway [OPTIONS] --id <ID>

Options:
      --id <ID>                  Id of client we want to add gateway for
      --gateway-id <GATEWAY_ID>  Explicitly specify id of the gateway to register with. If unspecified, a random gateway will be chosen instead
      --force-tls-gateway        Specifies whether the client will attempt to enforce tls connection to the desired gateway
      --latency-based-selection  Specifies whether the new gateway should be determined based by latency as opposed to being chosen uniformly
      --set-active               Specify whether this new gateway should be set as the active one
      --nym-apis <NYM_APIS>      Comma separated list of rest endpoints of the API validators
  -o, --output <OUTPUT>          [default: text] [possible values: text, json]
  -h, --help                     Print help
```
