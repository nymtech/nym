```sh
Initialise a Nym client. Do this first!

Usage: nym-socks5-client init [OPTIONS] --id <ID> --provider <PROVIDER>

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

      --provider <PROVIDER>
          Address of the socks5 provider to send messages to

      --use-reply-surbs <USE_REPLY_SURBS>
          Specifies whether this client is going to use an anonymous sender tag for communication with the service provider. While this is going to hide its actual address information, it will make the actual communication slower and consume nearly double the bandwidth as it will require sending reply SURBs.
          
          Note that some service providers might not support this.
          
          [possible values: true, false]

  -p, --port <PORT>
          Port for the socket to listen on in all subsequent runs

      --host <HOST>
          The custom host on which the socks5 client will be listening for requests

  -o, --output <OUTPUT>
          [default: text]
          [possible values: text, json]

  -h, --help
          Print help (see a summary with '-h')
```
