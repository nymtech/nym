const { createNymMixnetClient } = require('../dist/cjs/index.js');

(async () => {
  const nym = await createNymMixnetClient();

  nym.events.subscribeToTextMessageReceivedEvent(async ({ args: { payload, mimeType } }) => {
    console.log(`received message: ${payload}`);
    console.log(`with mimeType: ${mimeType}`);
  });

  // start the client and connect to a gateway
  await nym.client.start({
    nymApiUrl: 'https://validator.nymtech.net/api/',
    clientId: 'my-client',
  });

  nym.events.subscribeToConnected(async (e) => {
    // send a message to yourself
    const message = 'Hello';
    const recipient = await nym.client.selfAddress();
    console.log('main thread address: ', recipient);
    console.log(`sending "${message}" to ourselves...`);
    await nym.client.send({ payload: { message, mimeType: 'text/plain' }, recipient });
  });
})();
