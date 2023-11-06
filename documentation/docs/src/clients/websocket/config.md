# Configuration

## Default listening port 
The Nym native client exposes a websocket interface that your code connects to. To program your app, choose a websocket library for whatever language you're using. The **default** websocket port is `1977`, you can override that in the client config if you want.

You can either set this via the `--port` flag at `init` or `run`, or you can manually edit `~/.nym/clients/<CLIENT-ID>/config/config.toml`.

> Remember to restart your client if you change your listening port via editing your config file. 

## Choosing a Gateway
By default your client will choose a random gateway to connect to.

However, there are several options for choosing a gateway, if you do not want one that is randomly assigned to your client:
* If you wish to connect to a specific gateway, you can specify this with the `--gateway` flag when running `init`.
* You can also choose a gateway based on its location relative to your client. This can be done by appending the `--latency-based-routing` flag to your `init` command. This command means that to select a gateway, your client will:
    * fetch a list of all available gateways
    * send few ping messages to all of them, and measure response times.
    * create a weighted distribution to randomly choose one, favouring ones with lower latency.

> Note this doesn't mean that your client will pick the closest gateway to you, but it will be far more likely to connect to gateway with a 20ms ping rather than 200ms

## Configuring your client
When you initalise a client instance, a configuration directory will be generated and stored in `$HOME_DIR/.nym/clients/<client-name>/`.

```
tree $HOME/<user>/.nym/clients/example-client
├── config
│   └── config.toml
└── data
    ├── ack_key.pem
    ├── gateway_shared.pem
    ├── private_encryption.pem
    ├── private_identity.pem
    ├── public_encryption.pem
    └── public_identity.pem
```

The `config.toml` file contains client configuration options, while the two `pem` files contain client key information.

The generated files contain the client name, public/private keypairs, and gateway address. The name `<client_id>` in the example above is just a local identifier so that you can name your clients.

### Configuring your client for Docker
By default, the native client listens to host `127.0.0.1`. However this can be an issue if you wish to run a client in a Dockerized environment, where it can be convenenient to listen on a different host such as `0.0.0.0`.

You can set this via the `--host` flag during either the `init` or `run` commands.

Alternatively, a custom host can be set in the `config.toml` file under the `socket` section. If you do this, remember to restart your client process.
