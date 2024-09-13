# Browser only
With the Typescript SDK you can run a Nym client in a webworker - meaning you can connect to the mixnet through the browser without having to worry about any other code than your web framework.

- Oreowallet have integrated `mixFetch` into their browser-extension wallet to run transactions through the mixnet.
  - [Codebase](https://github.com/oreoslabs/oreowallet-extension/tree/mixFetch)

- [NoTrustVerify](https://notrustverify.ch/) have set up an example application using [`mixFetch`](https://sdk.nymtech.net/examples/mix-fetch) to fetch crypto prices from CoinGecko over the mixnet.
  - [Website](https://notrustverify.github.io/mixfetch-examples/)
  - [Codebase](https://github.com/notrustverify/mixfetch-examples)

- There is a coconut-scheme based Credential Library playground [here](https://coco-demo.nymtech.net/). This is a WASM implementation of our Coconut libraries which generate raw Coconut credentials. Test it to create and re-randomize your own credentials. For more information on what is happening here check out the [Coconut docs](https://nymtech.net/docs/coconut.html).

- You can find a browser-based 'hello world' chat app [here](https://chat-demo.nymtech.net). Either open in two browser windows and send messages to yourself, or share with a friend and send messages to each other through the mixnet.
