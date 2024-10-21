# Other Examples

### Browser only
With the Typescript SDK you can run a Nym client in a webworker - meaning you can connect to the mixnet through the browser without having to worry about any other code than your web framework.

- Oreowallet have integrated `mixFetch` into their browser-extension wallet to run transactions through the mixnet.
  - [Codebase](https://github.com/oreoslabs/oreowallet-extension/tree/mixFetch)

- [NoTrustVerify](https://notrustverify.ch/) have set up an example application using [`mixFetch`](https://sdk.nymtech.net/examples/mix-fetch) to fetch crypto prices from CoinGecko over the mixnet.
  - [Website](https://notrustverify.github.io/mixfetch-examples/)
  - [Codebase](https://github.com/notrustverify/mixfetch-examples)

### Services
Custom services involve two pieces of code that communicate via the mixnet: a client, and a custom server/service. This custom service will most likely interact with the wider internet / a clearnet service on your behalf, with the mixnet between you and the service, acting as a privacy shield.

> The current model of relying on a Service Provider has some issues, such as additional complexity in deployment and maintenance, as well as creating potential chokepoints for app traffic. Work is going on (in the open in our [monorepo](https://github.com/nymtech/nym) ofc) to start removing this requirement as much as possible, by allowing for the creation of packet-contents in such a way that the existing Network Requester/Exit Gateway infrastructure can support network requests in a similar way to `mixFetch`. More on this as and when it is released.

- The [Nym Zcash RPC demo](https://github.com/nymtech/nym-zcash-rpc-demo) and [Nym Zcash gRPC demo](https://github.com/nymtech/nym-zcash-grpc-demo), are also proof of concept 'generic' mixnet piping examples which exposes a TPC Socket on the client side for incoming traffic, piping it through the mixnet, and then streams TCP packets 'out' the other side. A good example of non-app-specific traffic transport which developers could also quite easily use as a template for their own app-specific work.
    - [Codebase](https://github.com/nymtech/nym-zcash-rpc-demo)

> Note this has now been included in the Rust SDK as the [TCP Proxy](../tcpproxy).

- PasteNym is a private pastebin alternative. It involves a browser-based frontend utilising the Typescript SDK and a Python-based backend service communicating with a standalone Nym Websocket Client. **If you're a Python developer, start here!**.
  - [Frontend codebase](https://github.com/notrustverify/pastenym)
  - [Backend codebase](https://github.com/notrustverify/pastenym-frontend)
