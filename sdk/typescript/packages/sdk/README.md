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

### Packaging

If you're a Nym platform developer who's made changes to the Rust (or JS) files and wants to re-publish the package to NPM, here's how you do it: 

1. bump version numbers as necessary for SemVer
2. `yarn build` builds the release directory
3. `npm publish --access=public` will publish your changed package to NPM
