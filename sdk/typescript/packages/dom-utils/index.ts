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
  createChatEntry({message, sendOrReceive: "SEND"});
}

const createISOTimeStamp = () => new Date().toISOString().substring(11, 12);

const createChatEntry = ({message, sendOrReceive}: {message: string, sendOrReceive: "SEND" | "RECEIVE"}) => {
  const timestamp = createISOTimeStamp();

  const sendDiv = document.createElement('div');  
  const paragraph = document.createElement('p');
  paragraph.setAttribute('style', `color: ${sendOrReceive === "SEND" ? "blue" : "green"}`);
  const paragraphContent = document.createTextNode(`${timestamp} sent >>> ${message}`);
  paragraph.appendChild(paragraphContent);

  sendDiv.appendChild(paragraph);
  document.getElementById('output')?.appendChild(sendDiv);

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */

function displaySend(message: string) {
  createChatEntry({message, sendOrReceive: "SEND"});
  return undefined
}

/**
 * Display received text messages in the browser. Colour them green.
 *
 * @param {string} message
 */

export function displayReceived(message: string) {
  return createChatEntry({message, sendOrReceive: "RECEIVE"});
   return undefined
}

/**
 * Display the nymClient's sender address in the user interface
 *
 * @param {Client} nymClient
 */
export function displaySenderAddress(address: string) {
  (document.getElementById('sender') as HTMLFormElement).value = address;
}
