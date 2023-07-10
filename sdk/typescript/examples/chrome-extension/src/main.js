// main.js
// Simple example of how to load Nym's TypeScript SDK and bind it to a DOM.
// Look at dom-utils.js for the DOM utility functionality referenced here.

// Import the Nym mixnet ESM module.
import { createNymMixnetClient } from '@nymproject/sdk';

// Import the DOM utility functionality.
import { displaySenderAddress, displayReceived, sendMessageTo } from './dom-utils.js';

async function main() {
  // Initialize the Nym mixnet client.
  let nymClient = await createNymMixnetClient();
  if (!nymClient) {
    console.error('Oh no! Could not create client');
    return;
  }

  const nymApiUrl = 'https://validator.nymtech.net/api';
  const preferredGatewayIdentityKey = 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM';

  // subscribe to connect event, so that we can show the client's address
  nymClient.events.subscribeToConnected((e) => {
    if (e.args.address) {
      displaySenderAddress(e.args.address);
    }
  });

  // subscribe to message received events and show any string messages received
  nymClient.events.subscribeToTextMessageReceivedEvent((e) => {
    displayReceived(e.args.payload);
  });

  const sendButton = document.querySelector('#send-button');
  if (sendButton) {
    sendButton.onclick = function () {
      if (nymClient) {
        sendMessageTo(nymClient);
      }
    };
  }

  nymClient.events.subscribeToRawMessageReceivedEvent((e) => console.log('Received: ', e.args.payload));
  await nymClient.client.start({
    clientId: 'My awesome client',
    nymApiUrl,
    preferredGatewayIdentityKey,
  });
}

window.addEventListener('DOMContentLoaded', () => {
  main();
});
