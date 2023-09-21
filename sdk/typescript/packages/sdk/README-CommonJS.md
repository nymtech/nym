# Nym SDK (Typescript) CommonJS and ES5

#### ⚠ WARNING: ES5 & CommonJS that requires bundling ⚠
#### Please note: this is a CommonJS package targeting ES5. Use this for older projects that can't process `import` syntax. You will need to handle bundling of the web-workers and WASM.

{{>variants}}

----------------------------

The Nym SDK for Typescript will get you creating apps that can use the Nym Mixnet and Coconut credentials quickly.

## TL;DR

Include the SDK in your project:

```
npm install @nymproject/sdk-commonjs
```

Open a connection to a Gateway on the Nym Mixnet:

```js
const { createNymMixnetClient } = require('@nymproject/sdk-commonjs');

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
