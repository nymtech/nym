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

import * as wasm from "nym-client-wasm";
import { makeRegisterRequest, makeAuthenticateRequest, makeSendablePacket, getTopology } from "nym-client-wasm/helpers"

class GatewayConnection {
    constructor(gateway_url, ownAddress, registeredCallback) {
        const conn = new WebSocket(gateway_url);

        // lovely bindings here we go!
        this.onSocketMessage = this.onSocketMessage.bind(this);
        this.handleBlobResponse = this.handleBlobResponse.bind(this);
        this.handleRegisterResponse = this.handleRegisterResponse.bind(this);
        this.handleAuthenticateResponse = this.handleAuthenticateResponse.bind(this);
        this.handleErrorResponse = this.handleErrorResponse.bind(this);
        this.handleSendConfirmation = this.handleSendConfirmation.bind(this);

        conn.onclose = this.onSocketClose;
        conn.onmessage = this.onSocketMessage;
        conn.onerror = (ev) => console.error("Socket error: ", ev);
        conn.onopen = this.onSocketOpen.bind(this);

        this.ownAddress = ownAddress;
        this.conn = conn;
        this.registeredCallback = registeredCallback
    }

    closeConnection() {
        this.conn.close();
    }

    sendMessage(topology, message, recipient) {
        let gatewaySphinxPacket = makeSendablePacket(topology, message, recipient);
        this.conn.send(gatewaySphinxPacket)
    }

    sendRegisterRequest() {
        const registerReq = makeRegisterRequest(this.ownAddress);
        this.conn.send(registerReq);
    }

    sendAuthenticateRequest(token) {
        const authenticateReq = makeAuthenticateRequest(this.ownAddress, token);
        this.conn.send(authenticateReq);
    }

    onSocketOpen() {
        console.log("opened socket connection!")

        // the first message you should send is either register or authenticate, otherwise your sphinx packets
        // are not going to be accepted (nor you will receive any)
        this.sendRegisterRequest()
    }

    onSocketClose(ev) {
        console.log("The websocket was closed", ev);
    }

    handleSendConfirmation(additionalData) {
        console.log("received send confirmation", additionalData);
    }

    handleRegisterResponse(additionalData) {
        console.log("registered! - ", additionalData);
        this.registeredCallback();
    }

    handleAuthenticateResponse(additionalData) {
        console.log("authenticated! - ", additionalData);
    }

    handleErrorResponse(additionalData) {
        console.error("received error response: ", additionalData)
    }

    handleBlobResponse(data) {
        // note that the actual handling depends on the expected content, however,
        // in this example we're just sending a text messages to ourselves and hence
        // we can safely read it as text
        let reader = new FileReader();

        reader.onload = () => {
            displayReceived(reader.result)
        };

        reader.readAsText(data);
    }

    onSocketMessage(ev) {
        if (ev.data instanceof Blob) {
            this.handleBlobResponse(ev.data)
        } else {
            const receivedData = JSON.parse(ev.data);
            switch (receivedData.type) {
                case "send": return this.handleSendConfirmation(receivedData);
                case "register": return this.handleRegisterResponse(receivedData);
                case "authenticate": return this.handleAuthenticateResponse(receivedData);
                case "error": return this.handleErrorResponse(receivedData);
            }

            console.log("Received unknown response!");
            console.log(receivedData);
        }
    }
}


function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

function main() {
    var gatewayUrl = "ws://127.0.0.1:1001";
    var directoryUrl = "http://127.0.0.1:8080/api/presence/topology";


    // NOTE: in an actual application, you should save those keys somewhere and only generate them a single time
    // then reuse them in subsequent runs
    const keypair_address = JSON.parse(wasm.keygen());
    console.log("our keypair and address are: ", keypair_address);

    document.getElementById("recipient").value = keypair_address.address
    document.querySelector('#send-button').disabled = true;

    // basically to ensure connection is established before we do anything else
    // and more importantly to ensure we are registered so that we could send messages to ourselves
    // (i.e. we exist in the topology we're about to pull
    let registered_callback = async () => {
        // this function is called immediately after we receive registration response
        // however, gateways are sending their presence data every 1.5s, so it might take at most 1.5s before
        // our address is present in the topology, so just wait that long
        // (JS to DH: this is done so that we could have multiple gateways online even for the wasm client)
        await sleep(1500);

        // Get the topology, then the mixnode and provider data
        const topology = await getTopology(directoryUrl);
        // Set up the send button
        const string_topology = JSON.stringify(topology);
        const sendButton = document.querySelector('#send-button');
        sendButton.disabled = false;
        sendButton.onclick = function () {
            sendMessageToMixnet(conn, string_topology);
        }
    }

    // Set up a websocket connection to the gateway node
    let conn = new GatewayConnection(gatewayUrl, keypair_address.address, registered_callback);
}

// Create a Sphinx packet and send it to the mixnet through the Gateway node. 
function sendMessageToMixnet(connection, topology) {
    var recipient = document.getElementById("recipient").value;
    var sendText = document.getElementById("sendtext").value;
    connection.sendMessage(topology, sendText, recipient);
    displaySend(sendText);
}

function displaySend(message) {
    let timestamp = new Date().toISOString().substr(11, 12);
    document.getElementById("output").innerHTML = document.getElementById("output").innerHTML + "<p style='color: blue; word-break: break-all;'>" + timestamp + " <b>sent</b> >>> " + message + "</p >";
}

function displayReceived(message) {
    let timestamp = new Date().toISOString().substr(11, 12);
    document.getElementById("output").innerHTML = document.getElementById("output").innerHTML + "<p style='color: green; word-break: break-all;'>" + timestamp + " <b>received</b> >>> " + message + "</p >";
}

// Let's get started!
main();