// Copyright 2020 Nym Technologies SA
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

import {
    Client,
    Identity
} from "@nymproject/nym-client-wasm/client"

async function main() {
    // Set up identity and client
    let directory = "http://localhost:8080";
    let identity = new Identity();

    console.log(identity);

    console.log("should be done!");
//    let nymClient = new Client(directory, identity, null);
//
//    // Wire up events callbacks
//    nymClient.onConnect = (_) => displaySenderAddress(nymClient);
//    nymClient.onText = displayReceived;
//    nymClient.onErrorResponse = (event) => alert("Received invalid gateway response", event.data);
//    const sendButton = document.querySelector('#send-button');
//    sendButton.onclick = function () {
//        sendMessageTo(nymClient);
//    }
//
//    // Start the Nym client. Connects to a Nym gateway via websocket.
//    await nymClient.start();
}

/**
 * Create a Sphinx packet and send it to the mixnet through the gateway node.
 * 
 * Message and recipient are taken from the values in the user interface.
 *
 * @param {Client} nymClient the nym client to use for message sending
 */
function sendMessageTo(nymClient) {
    var message = document.getElementById("message").value;
    var recipient = document.getElementById("recipient").value;
    nymClient.sendMessage(message, recipient);
    displaySend(message);
}
/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displaySend(message) {
    let timestamp = new Date().toISOString().substr(11, 12);
    let out = "<p style='color: blue; word-break: break-all;'>" + timestamp + " <b>sent</b> >>> " + message + "</p >";
    document.getElementById("output").innerHTML = out + document.getElementById("output").innerHTML;
}

/**
 * Display received text messages in the browser. Colour them green.
 *
 * @param {string} message
 */
function displayReceived(message) {
    let timestamp = new Date().toISOString().substr(11, 12);
    let out = "<p style='color: green; word-break: break-all;'>" + timestamp + " <b>received</b> >>> " + message + "</p >";
    document.getElementById("output").innerHTML = out + document.getElementById("output").innerHTML;
}


/**
 * Display the nymClient's sender address in the user interface
 *
 * @param {Client} nymClient
 */
function displaySenderAddress(nymClient) {
    document.getElementById("sender").value = nymClient.formatAsRecipient();
}

// Let's get started!
main();