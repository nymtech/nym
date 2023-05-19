// dom-utils.js
// Contains utility functionality to help manipulate the DOM elements necessary to demonstrate the Nym example.

/**
 * Create a Sphinx packet and send it to the mixnet through the gateway node.
 *
 * Message and recipient are taken from the values in the user interface.
 *
 * @param {Client} nymClient the nym client to use for message sending
 */
async function sendMessageTo(nym) {
	const message = document.getElementById('message').value;
	const recipient = document.getElementById('recipient').value;
	await nym.client.send({
		payload: {
			message,
			mimeType: 'text/plain'
		},
		recipient
	});
	displaySend(message);
}

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displaySend(message) {
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
function displayReceived(message) {
	const content = message;
	const timestamp = new Date().toLocaleTimeString();
	const receivedDiv = document.createElement('div');
	const paragraph = document.createElement('p');
	paragraph.setAttribute('style', 'color: green');
	const paragraphContent = document.createTextNode(`${timestamp} received >>> ${content}`);
	paragraph.appendChild(paragraphContent);
	receivedDiv.appendChild(paragraph);
	document.getElementById('output')?.appendChild(receivedDiv);
}

/**
 * Display the nymClient's sender address in the user interface
 *
 * @param {Client} nymClient
 */
function displaySenderAddress(address) {
	document.getElementById('sender').value = address;
}

export { sendMessageTo, displaySend, displayReceived , displaySenderAddress }