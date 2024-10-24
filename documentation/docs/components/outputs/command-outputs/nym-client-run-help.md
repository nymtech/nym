```sh
Run the Nym client with provided configuration client optionally overriding set parameters

Usage: nym-client run [OPTIONS] --id <ID>

Options:
      --id <ID>
          Id of client we want to create config for
      --gateway <GATEWAY>
          Id of the gateway we want to connect to. If overridden, it is user's responsibility to ensure prior registration happened
      --nym-apis <NYM_APIS>
          Comma separated list of rest endpoints of the API validators
      --disable-socket <DISABLE_SOCKET>
          Whether to not start the websocket [possible values: true, false]
  -p, --port <PORT>
          Port for the socket to listen on
      --host <HOST>
          Ip for the socket (if applicable) to listen for requests
  -h, --help
          Print help
```
