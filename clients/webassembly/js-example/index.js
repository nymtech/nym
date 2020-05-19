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
} from "nym-client-wasm/client"

async function main() {
    let directory = "https://qa-directory.nymtech.net";
    let identity = new Identity(); // or load one from storage if you have one already

    document.getElementById("sender").value = "loading...";

    let nymClient = new Client(directory, identity, null); // provide your authToken if you've registered before
    nymClient.onEstablishedGatewayConnection = (_) => document.getElementById("sender").value = nymClient.formatAsRecipient() // overwrite default behaviour with our implementation
    nymClient.onParsedBlobResponse = displayReceived // overwrite default behaviour with our implementation
    nymClient.onErrorResponse = (event) => alert("Received invalid gateway response", event.data)
    await nymClient.start();

    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageTo(nymClient);
    }
}

// Create a Sphinx packet and send it to the mixnet through the Gateway node. 
function sendMessageTo(client) {
    var message = document.getElementById("sendtext").value;
    var recipient = document.getElementById("recipient").value;
    client.sendMessage(message, recipient);
    displaySend(message);
}

function displaySend(message) {
    let timestamp = new Date().toISOString().substr(11, 12);
    let out = "<p style='color: blue; word-break: break-all;'>" + timestamp + " <b>sent</b> >>> " + message + "</p >";
    document.getElementById("output").innerHTML = out + document.getElementById("output").innerHTML;
}

function displayReceived(message) {
    let timestamp = new Date().toISOString().substr(11, 12);
    let out = "<p style='color: green; word-break: break-all;'>" + timestamp + " <b>received</b> >>> " + message + "</p >";
    document.getElementById("output").innerHTML = out + document.getElementById("output").innerHTML;
}


// Let's get started!
main();