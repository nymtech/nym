# Custom Services 
Custom services involve two pieces of code that communicate via the mixnet: a client, and a custom server/service. This custom service will most likely interact with the wider internet / a clearnet service on your behalf, with the mixnet between you and the service, acting as a privacy shield. 

> The current model of relying on a Service Provider has some issues, such as additional complexity in deployment and maintenance, as well as creating potential chokepoints for app traffic. Work is going on (in the open in our [monorepo](https://github.com/nymtech/nym) ofc) to start removing this requirement as much as possible, by allowing for the creation of packet-contents in such a way that the existing Network Requester/Exit Gateway infrastructure can support network requests in a similar way to `mixFetch`. More on this as and when it is released. 

- [Nym Zcash RPC demo](https://github.com/nymtech/nym-zcash-rpc-demo), although used to only pipe RPC traffic, is a proof of concept 'generic' mixnet piping example which exposes a TPC Socket on the client side for incoming traffic, pipes this through the mixnet, and then streams TCP packets 'out' the other side. A good example of non-app-specific traffic transport which developers could also quite easily use as a template for their own app-specific work. 
  - [Codebase](https://github.com/nymtech/nym-zcash-rpc-demo)

- PasteNym is a private pastebin alternative. It involves a browser-based frontend utilising the Typescript SDK and a Python-based backend service communicating with a standalone Nym Websocket Client. **If you're a Python developer, start here!**.
  - [Frontend codebase](https://github.com/notrustverify/pastenym)
  - [Backend codebase](https://github.com/notrustverify/pastenym-frontend) 
  
- Nostr-Nym is another application written by [NoTrustVerify](https://notrustverify.ch/), standing between mixnet users and a Nostr server in order to protect their metadata from being revealed when gossiping. **Useful for Go and Python developers**.  
  - [Codebase](https://github.com/notrustverify/nostr-nym)
  
- Spook and Nym-Ethtx are both examples of Ethereum transaction broadcasters utilising the mixnet, written in Rust. Since they were written before the release of the Rust SDK, they utilise standalone clients to communicate with the mixnet. 
  - [Spook](https://github.com/EdenBlockVC/spook) (**Typescript**)
  - [Nym-Ethtx](https://github.com/noot/nym-ethtx) (**Rust**)
  
- NymDrive is an early proof of concept application for privacy-enhanced file storage on IPFS. **JS and CSS**, and a good example of packaging as an Electrum app.  
  - [Codebase](https://github.com/saleel/nymdrive)
