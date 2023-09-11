# Nym SDK (Typescript) - 🥛🥛🥛 Full Fat

#### ⚠ WARNING: ESM with inline blobs ⚠
#### Please note: this is an ESM package targeting ES6. Use this when you have bundling issues. It includes inline blobs that will interfere with CSPs that disallow them.

{{>variants}}

----------------------------

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
  nym.client.send({ payload, recipient });
  
};
```

This will start the WASM client on a worker thread, so that your code can stay nice and snappy.

Send a message to another user (you will need to know their address at a Gateway):

```ts
  const payload = 'Hello mixnet';
  const recipient = '<< RECIPIENT ADDRESS GOES HERE >>';
  await nym.client.send({ payload, recipient });
```
