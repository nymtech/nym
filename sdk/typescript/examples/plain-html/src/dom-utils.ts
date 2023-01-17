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

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displaySend(message: string) {
  const timestamp = new Date().toISOString().substr(11, 12);

  const sendDiv = document.createElement('div');
  const paragraph = document.createElement('p');
  paragraph.setAttribute('style', 'color: blue');
  const paragraphContent = document.createTextNode(`${timestamp} sent >>> ${message}`);
  paragraph.appendChild(paragraphContent);

  sendDiv.appendChild(paragraph);
  document.getElementById('output')?.appendChild(sendDiv);
}

/**
 * Display received text messages in the browser. Colour them green.
 *
 * @param {string} message
 */
export function displayReceived(message: string) {
  const content = message;

  const timestamp = new Date().toLocaleTimeString();
  const receivedDiv = document.createElement('div');
  const paragraph = document.createElement('p');
  paragraph.setAttribute('style', 'color: green');
  const paragraphContent = document.createTextNode(`${timestamp} received >>> ${content}`);
  // const paragraphContent = document.createTextNode(timestamp + " received >>> " + content + ((replySurb != null) ? "Reply SURB was attached here (but we can't do anything with it yet" : " (NO REPLY-SURB AVAILABLE)"))
  paragraph.appendChild(paragraphContent);
  receivedDiv.appendChild(paragraph);
  document.getElementById('output')?.appendChild(receivedDiv);
}

/**
 * Display the nymClient's sender address in the user interface
 *
 * @param {Client} nymClient
 */
export function displaySenderAddress(address: string) {
  (document.getElementById('sender') as HTMLFormElement).value = address;
}
