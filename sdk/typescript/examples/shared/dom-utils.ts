import { NymMixnetClient, MimeTypes } from '@nymproject/sdk';

/**
 * Create a Sphinx packet and send it to the mixnet through the gateway node.
 *
 * Message and recipient are taken from the values in the user interface.
 *
 * @param {Client} nymClient the nym client to use for message sending
 */

export async function sendMessageTo(nym: NymMixnetClient) {
  const message = (document.getElementById('message') as HTMLFormElement).value;
  const recipient = (document.getElementById('recipient') as HTMLFormElement).value;

  await nym.client.send({ payload: { message, mimeType: MimeTypes.TextPlain }, recipient });
  displaySend(message);
}

const createISOTimeStamp = () => new Date().toISOString().substring(11, 16);

const createChatEntry = ({ message, sendOrReceive }: { message: string; sendOrReceive: 'SEND' | 'RECEIVE' }) => {
  const timestamp = createISOTimeStamp();
  console.log(timestamp);

  const sendDiv = document.createElement('div');
  const paragraph = document.createElement('p');
  paragraph.setAttribute('style', `color: ${sendOrReceive === 'SEND' ? 'blue' : 'green'}`);
  const paragraphContent = document.createTextNode(
    `${timestamp} ${sendOrReceive === 'SEND' ? 'sent' : 'received'} >>> ${message}`,
  );
  paragraph.appendChild(paragraphContent);

  sendDiv.appendChild(paragraph);
  document.getElementById('output')?.appendChild(sendDiv);
};

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */

export function displaySend(message: string) {
  createChatEntry({ message, sendOrReceive: 'SEND' });
  return undefined;
}

/**
 * Display received text messages in the browser. Colour them green.
 *
 * @param {string} message
 */

export function displayReceived(message: string) {
  createChatEntry({ message, sendOrReceive: 'RECEIVE' });
  return undefined;
}

/**
 * Display the nymClient's sender address in the user interface
 *
 * @param {string} address
 */
export function displaySenderAddress(address: string) {
  (document.getElementById('sender') as HTMLFormElement).value = address;
}
