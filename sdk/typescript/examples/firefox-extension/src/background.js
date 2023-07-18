// main.js
// Simple example of how to load Nym's TypeScript SDK and bind it to a DOM.
// Look at dom-utils.js for the DOM utility functionality referenced here.

// Import the Nym mixnet ESM module.
// Import The web workers for the Nym mixnet ESM module.These are required for to run the Nym mixnet client.

import { createNymMixnetClient } from '../../../packages/sdk/dist/full-fat/index.js';
import '../../../packages/sdk/dist/full-fat/web-worker-0.js';
import '../../../packages/sdk/dist/full-fat/web-worker-1.js';

const backgroundState = {
  isReady: false,
  address: '',
  recipient: '',
  messageLog: [],
};

async function initBackground() {
  // Initialize the Nym mixnet client.
  let nymClient = await createNymMixnetClient().catch((err) => {
    console.log(err);
  });
  if (!nymClient) {
    console.error('Oh no! Could not create client');
    return;
  }
  const nymApiUrl = 'https://validator.nymtech.net/api';
  const preferredGatewayIdentityKey = 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM';

  // subscribe to connect event, so that we can show the client's address
  nymClient.events.subscribeToConnected((e) => {
    if (e.args.address) {
      backgroundState.address = e.args.address;
      browser.runtime.sendMessage({
        type: 'displaySenderAddress',
        message: backgroundState.address,
      });
    }
  });

  // subscribe to message received events and show any string messages received
  nymClient.events.subscribeToTextMessageReceivedEvent((e) => {
    backgroundState.messageLog.push({
      type: 'received',
      message: e.args.payload,
    });
    browser.runtime.sendMessage({
      type: 'displayReceived',
      message: e.args.payload,
    });
  });

  nymClient.events.subscribeToRawMessageReceivedEvent((e) => console.log('Received: ', e.args.payload));
  await nymClient.client.start({
    clientId: 'My awesome client',
    nymApiUrl,
    preferredGatewayIdentityKey,
  });
  browser.runtime.onMessage.addListener(async (data) => {
    switch (data.type) {
      case 'nymClientSendMessage':
        if (nymClient) {
          await nymClient.client.send({
            payload: {
              message: data.message,
              mimeType: 'text/plain',
            },
            recipient: data.recipient,
          });
          backgroundState.messageLog.push({
            type: 'sent',
            message: data.message,
          });
          break;
        }
    }
  });
  backgroundState.isReady = true;
}

window.addEventListener('DOMContentLoaded', () => {
  browser.runtime.onMessage.addListener((data) => {
    switch (data.type) {
      case 'popupReady':
        if (backgroundState.isReady) {
          browser.runtime.sendMessage({
            type: 'displaySenderAddress',
            message: backgroundState.address,
          });
          browser.runtime.sendMessage({
            type: 'displayMessageLog',
            message: backgroundState.messageLog,
          });
          browser.runtime.sendMessage({
            type: 'updateRecipient',
            message: backgroundState.recipient,
          });
        } else {
          initBackground();
        }
        break;
      case 'updateRecipient':
        backgroundState.recipient = data.message;
    }
  });
});
