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
import {
    createSphinxPacket,
    makeAuthenticateRequest,
    makeRegisterRequest,
    makeSendablePacket
} from "nym-client-wasm/client"

class GatewayClient {

    constructor(directoryUrl, identity) {
        this.connection = null;
        this.directoryUrl = directoryUrl;
        this.gatewayUrl = null;
        this.identity = identity;
    }

    get ownAddress() {
        return this.identity.address;
    }

    async connect() {
        await this.refreshTopology();
        this.gatewayUrl = this.gatewayUrlFromTopology();
        this.connection = await this.connectWebSocket(this.gatewayUrl);
    }

    async register() {
        const registerReq = makeRegisterRequest(this.ownAddress);
        await this.connection.send(registerReq);
    }

    /* Gets the current Nym network topology, to find out what nodes exist. 
       Paths through the mix network are chosen by clients.   */
    async refreshTopology() {
        let response = await this.http('get', this.directoryUrl);
        this.topology = JSON.parse(response);
    }

    gatewayUrlFromTopology() {
        if (this.topology.gatewayNodes.length > 0) {
            return this.topology.gatewayNodes[0].clientListener;
        } else {
            throw "Unable to get Nym gateway address from " + directoryUrl + ", have you connected?";
        }
    }

    sendMessage(message, recipient) {
        let sphinxPacket = wasm.create_sphinx_packet(JSON.stringify(this.topology), message, recipient);
        this.connection.send(sphinxPacket);
    }

    /* Make an HTTP request */
    http(method, url) {
        return new Promise(function (resolve, reject) {
            let xhr = new XMLHttpRequest();
            xhr.open(method, url);
            xhr.onload = function () {
                if (this.status >= 200 && this.status < 300) {
                    resolve(xhr.response);
                } else {
                    reject({
                        status: this.status,
                        statusText: xhr.statusText
                    });
                }
            };
            xhr.onerror = function () {
                reject({
                    status: this.status,
                    statusText: xhr.statusText
                });
            };
            xhr.send();
        });
    }

    connectWebSocket(url) {
        return new Promise(function (resolve, reject) {
            var server = new WebSocket(url);
            server.onopen = function () {
                resolve(server);
            };
            server.onerror = function (err) {
                reject(err);
            };

        });
    }

}

class Identity {
    constructor() {
        this.identity = JSON.parse(wasm.keygen());
        return this.identity;
    }
}

async function main() {
    // let directory = "https://qa-directory.nymtech.net/api/presence/topology";
    let directory = "http://localhost:8080/api/presence/topology";
    let identity = new Identity(); // or load one from storage if you have one already
    let gateway = new GatewayClient(directory, identity);
    await gateway.connect(); // makes a websocket connection to the gateway
    await gateway.register(); // registers your new identity, not needed if you've registered before

    document.getElementById("sender").value = gateway.ownAddress;

    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageTo(gateway);
    }
}

// Create a Sphinx packet and send it to the mixnet through the Gateway node. 
function sendMessageTo(gateway) {
    var message = document.getElementById("sendtext").value;
    var recipient = document.getElementById("recipient").value;
    gateway.sendMessage(message, recipient);
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

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}


// Let's get started!
main();