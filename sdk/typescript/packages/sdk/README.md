# Nym SDK (Typescript)

The Nym SDK for Typescript will get you creating apps that can use the Nym Mixnet and Coconut credentials quickly.

## TL;DR

Include the SDK in your project:

```
npm install @nymproject/sdk
```

Open a connection to a Gateway on the Nym Mixnet:

```ts
import { createNymMixnetClient } from '@nymproject/sdk';

const main = async () => {
  const nym = await createNymMixnetClient();

  const nymApiUrl = 'https://validator.nymtech.net/api';

  // show message payload content when received 
  nym.events.subscribeToTextMessageReceivedEvent((e) => {
    console.log('Got a message: ', e.args.payload);
  });

  // start the client and connect to a gateway
  await nym.client.start({
    clientId: 'My awesome client',
    nymApiUrl,
  });

  // send a message to yourself
  const payload = 'Hello mixnet';
  const recipient = nym.client.selfAddress();
  nym.client.sendMessage({ payload, recipient });
  
};
```

This will start the WASM client on a worker thread, so that your code can stay nice and snappy.

Send a message to another user (you will need to know their address at a Gateway):

```ts
  const payload = 'Hello mixnet';
  const recipient = '<< RECIPIENT ADDRESS GOES HERE >>';
  await nym.client.sendMessage({ payload, recipient });
```

### Building from source

You will need to have the Rust WASM toolchain installed. Please [follow the instructions here](https://rustwasm.github.io/docs/book/game-of-life/setup.html)
to install `wasm-pack`.

Make sure you have either Node 16 LTS or Node 18 LTS installed and are using it to do the build.

Run these commands to build the SDK:
```
yarn
yarn build
```

The output bundle will be created in the `dist` directory.

### Packaging

If you're a Nym platform developer who's made changes to the Rust (or JS) files and wants to re-publish the package to NPM, here's how you do it: 

1. bump version numbers as necessary for SemVer
2. `yarn build` builds the release directory
3. `npm publish --access=public` will publish your changed package to NPM
