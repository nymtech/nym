// Copyright 2020-2023 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

class WebWorkerClient {
    worker = null;

    constructor() {
        this.worker = new Worker('./worker.js');

        this.worker.onmessage = (ev) => {
            if (ev.data && ev.data.kind) {
                switch (ev.data.kind) {
                    case 'Ready':
                        const {selfAddress} = ev.data.args;
                        displaySenderAddress(selfAddress);
                        break;
                    case 'ReceiveMessage':
                        const {message, senderTag, isTestPacket } = ev.data.args;
                        displayReceived(message, senderTag, isTestPacket);
                        break;
                    case 'DisplayString':
                        const { rawString } = ev.data.args;
                        displayReceivedRawString(rawString)
                        break;
                    case 'DisableMagicTestButton':
                        const magicButton = document.querySelector('#magic-button');
                        magicButton.setAttribute('disabled', "true")
                        break;
                    case 'DisplayTesterResults':
                        const {score, sentPackets, receivedPackets, receivedAcks, duplicatePackets, duplicateAcks} = ev.data.args;
                        const resultText = `Test score: ${score}. Sent ${sentPackets} packets. Received ${receivedPackets} packets and ${receivedAcks} acks back. We also got ${duplicatePackets} duplicate packets and ${duplicateAcks} duplicate acks.`
                        displayReceivedRawString(resultText)
                        break;
                }
            }
        };
    }

    sendMessage = (message, recipient) => {
        if (!this.worker) {
            console.error('Could not send message because worker does not exist');
            return;
        }

        this.worker.postMessage({
            kind: 'SendMessage',
            args: {
                message, recipient,
            },
        });
    };

    sendMagicPayload = (mixnodeIdentity) => {
        if (!this.worker) {
            console.error('Could not send message because worker does not exist');
            return;
        }

        this.worker.postMessage({
            kind: 'MagicPayload',
            args: {
                mixnodeIdentity,
            },
        });
    }
}

let client = null;

async function main() {
    client = new WebWorkerClient();

    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageTo();
    };

    const magicButton = document.querySelector('#magic-button');
    magicButton.onclick = function () {
        sendMagicPayload();
    }
}

/**
 * Create a Sphinx packet and send it to the mixnet through the gateway node.
 *
 * Message and recipient are taken from the values in the user interface.
 *
 */
async function sendMessageTo() {
    const message = document.getElementById('message').value;
    const recipient = document.getElementById('recipient').value;

    await client.sendMessage(message, recipient);
    displaySend(message);
}

async function sendMagicPayload() {
    const payload = document.getElementById('magic_payload').value;
    await client.sendMagicPayload(payload)

    displaySend(`clicked the button and the payload is: ${payload}...`);
}

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displaySend(message) {
    let timestamp = new Date().toISOString().substr(11, 12);

    let sendDiv = document.createElement('div');
    let paragraph = document.createElement('p');
    paragraph.setAttribute('style', 'color: blue');
    let paragraphContent = document.createTextNode(timestamp + ' sent >>> ' + message);
    paragraph.appendChild(paragraphContent);

    sendDiv.appendChild(paragraph);
    document.getElementById('output').appendChild(sendDiv);
}

/**
 * Display received text messages in the browser. Colour them green.
 *
 * @param {Uint8Array} raw
 */
function displayReceived(raw, sender_tag, isTestPacket) {
    let content = new TextDecoder().decode(raw);
    if (sender_tag !== undefined) {
        console.log("this message also contained some surbs from", sender_tag)
    }

    if (isTestPacket) {
        const decoded = JSON.parse(content)
        content = `Received packet ${decoded.msg_id} / ${decoded.total_msgs} for node ${decoded.encoded_node_identity} (test: ${decoded.test_id})`
    }

    displayReceivedRawString(content)
}

function displayReceivedRawString(raw) {
    let timestamp = new Date().toISOString().substr(11, 12);
    let receivedDiv = document.createElement('div');
    let paragraph = document.createElement('p');
    paragraph.setAttribute('style', 'color: green');
    let paragraphContent = document.createTextNode(timestamp + ' received >>> ' + raw);
    paragraph.appendChild(paragraphContent);
    receivedDiv.appendChild(paragraph);
    document.getElementById('output').appendChild(receivedDiv);
}

/**
 * Display the nymClient's sender address in the user interface
 *
 * @param {String} address
 */
function displaySenderAddress(address) {
    document.getElementById('sender').value = address;
}

main();
