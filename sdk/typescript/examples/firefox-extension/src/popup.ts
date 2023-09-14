// Import the DOM utility functionality.
import { displaySenderAddress, displayReceived, sendMessageTo, displayMessageLog } from './dom-utils';

window.addEventListener('DOMContentLoaded', () => {
  const sendButton = document.querySelector('#send-button') as HTMLButtonElement;
  if (sendButton) {
    sendButton.onclick = function () {
      sendMessageTo();
    };
  }
  const recipient = document.getElementById('recipient') as HTMLFormElement;
  recipient.onchange = () => {
    browser.runtime.sendMessage({
      type: 'updateRecipient',
      message: recipient.value,
    });
  };
  browser.runtime.onMessage.addListener((data) => {
    switch (data.type) {
      case 'displaySenderAddress':
        displaySenderAddress(data.message);
        break;
      case 'displayReceived':
        displayReceived(data.message);
        break;
      // case 'sendMessageTo':
      //   sendMessageTo(data.message);
      //   break;
      case 'displayMessageLog':
        displayMessageLog(data.message);
        break;
      case 'updateRecipient':
        recipient.value = data.message;
    }
  });
  browser.runtime.sendMessage({
    type: 'popupReady',
    message: '',
  });
});
