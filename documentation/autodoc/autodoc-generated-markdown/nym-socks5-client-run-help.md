```sh
Run the Nym client with provided configuration client optionally overriding set parameters

Usage: nym-socks5-client run [OPTIONS] --id <ID>

Options:
      --id <ID>
          Id of client we want to create config for

      --gateway <GATEWAY>
          Id of the gateway we want to connect to. If overridden, it is user's responsibility to ensure prior registration happened

      --nym-apis <NYM_APIS>
          Comma separated list of rest endpoints of the API validators

      --use-anonymous-replies <USE_ANONYMOUS_REPLIES>
          Specifies whether this client is going to use an anonymous sender tag for communication with the service provider. While this is going to hide its actual address information, it will make the actual communication slower and consume nearly double the bandwidth as it will require sending reply SURBs.
          
          Note that some service providers might not support this.
          
          [possible values: true, false]

      --provider <PROVIDER>
          Address of the socks5 provider to send messages to

  -p, --port <PORT>
          Port for the socket to listen on

      --host <HOST>
          The custom host on which the socks5 client will be listening for requests

  -h, --help
          Print help (see a summary with '-h')
```
