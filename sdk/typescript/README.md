# Nym SDK (Typescript)

The Nym SDK for Typescript will get you creating apps that can use the Nym Mixnet and Coconut credentials quickly.

## TL;DR

Include the SDK in your project:

```
npm install @nymproject/sdk
```

Open a connection to a Gateway on the Nym Mixnet:

```ts
import { client } from '@nymproject/sdk';

const session = await client.connect('<<GATEWAY>>');
```

This will start the WASM client on a worker thread, so that your code can stay nice and snappy.

Send a message to another user (you will need to know their address at a Gateway):

```ts
const result = await client.send('<<USER ADDRESS>>', 'Hello Timmy!');
```

## Examples

- [Plain HTML + Javascript](examples/plain-html) - very simple chat app written in plain Javascript
- [Chat App](examples/chat) - simple chat app written in React with Webpack

Coming soon:

- [Node tester](examples/node-tester) - a React app that sends test packets to a mixnode and measure the network speed
- [Mixnet topology viewer](examples/topology) - a Svelte app that shows the mixnodes current in the active set
- [Get a bandwidth voucher](examples/coconut-bandwidth-voucher) - get a bandwidth voucher to use the mixnet