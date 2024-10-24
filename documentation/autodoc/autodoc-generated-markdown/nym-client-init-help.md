```sh
Initialise a Nym client. Do this first!

Usage: nym-client init [OPTIONS] --id <ID>

Options:
      --id <ID>
          Id of client we want to create config for
      --gateway <GATEWAY>
          Id of the gateway we are going to connect to
      --force-tls-gateway
          Specifies whether the client will attempt to enforce tls connection to the desired gateway
      --latency-based-selection
          Specifies whether the new gateway should be determined based by latency as opposed to being chosen uniformly
      --nym-apis <NYM_APIS>
          Comma separated list of rest endpoints of the API validators
      --disable-socket <DISABLE_SOCKET>
          Whether to not start the websocket [possible values: true, false]
  -p, --port <PORT>
          Port for the socket (if applicable) to listen on in all subsequent runs
      --host <HOST>
          Ip for the socket (if applicable) to listen for requests
  -o, --output <OUTPUT>
          [default: text] [possible values: text, json]
  -h, --help
          Print help
```
