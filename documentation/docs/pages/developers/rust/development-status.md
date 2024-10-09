# Development status
The SDK is still somewhat a work in progress: interfaces are fairly stable but still may change in subsequent releases.

In the future the SDK will be made up of several modules, each of which will allow developers to interact with different parts of Nym infrastructure.

| Module    | Functionality                                                                         | Released |
|-----------|---------------------------------------------------------------------------------------|----------|
| Mixnet    | Create / load clients & keypairs, subscribe to Mixnet events, send & receive messages | ✔️        |
| TcpProxy  | Utilise the TcpProxyClient and TcpProxyServer abstractions for streaming              | ✔️        |
| Ecash     | Create & verify Ecash credentials                                                     | 🛠️        |
| Validator | Sign & broadcast Nyx blockchain transactions, query the blockchain                    | ❌        |

The `Mixnet` module currently exposes the logic of two clients: the [websocket client]() TODO LINK, and the [socks]() TODO client.

The `TcpProxy` module exposes functionality to set up client/server instances that expose a localhost TcpSocket to read/write to.

The `Ecash` module is in progress. Right now it exposes logic allowing for the creation of zk-nym credentials on the Sandbox testnet.
