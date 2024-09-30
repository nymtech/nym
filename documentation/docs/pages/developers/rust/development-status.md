# Development status
The SDK is still somewhat a work in progress: interfaces are fairly stable but still may change in subsequent releases.

In the future the SDK will be made up of several components, each of which will allow developers to interact with different parts of Nym infrastructure.

| Component | Functionality                                                                         | Released |
|-----------|---------------------------------------------------------------------------------------|----------|
| Mixnet    | Create / load clients & keypairs, subscribe to Mixnet events, send & receive messages | ✔️        |
| Ecash   | Create & verify Ecash credentials                                                       | 🛠️        |
| Validator | Sign & broadcast Nyx blockchain transactions, query the blockchain                    | ❌        |

The `mixnet` component currently exposes the logic of two clients: the [websocket client]() TODO LINK, and the [socks]() TODO client.

The `ecash` component is currently being worked on. Right now it exposes logic allowing for the creation of Ecash credentials on the Sandbox testnet.
