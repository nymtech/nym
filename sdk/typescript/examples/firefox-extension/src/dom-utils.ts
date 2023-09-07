// dom-utils.js
// Contains utility functionality to help manipulate the DOM elements necessary to demonstrate the Nym example.

import { BackgroundState } from './background';
import { displayReceived, displaySend, displaySenderAddress } from '../../shared/dom-utils';

/**
 * Create a Sphinx packet and send it to the mixnet through the gateway node.
 *
 * Message and recipient are taken from the values in the user interface.
 *
 * @param {Client} nymClient the nym client to use for message sending
 */
async function sendMessageTo() {
  const message = (document.getElementById('message') as HTMLFormElement).value;
  const recipient = (document.getElementById('recipient') as HTMLFormElement).value;
  browser.runtime.sendMessage({
    type: 'nymClientSendMessage',
    message,
    recipient,
  });
  displaySend(message);
}

function displayMessageLog(messageLog: BackgroundState['messageLog']) {
  for (let i = 0; i < messageLog.length; i++) {
    if (messageLog[i].type === 'sent') {
      displaySend(messageLog[i].message);
    } else if (messageLog[i].type === 'received') {
      displayReceived(messageLog[i].message);
    }
  }
}

export { sendMessageTo, displaySend, displayReceived, displaySenderAddress, displayMessageLog };
