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

import * as wasm from "nym-sphinx-wasm";

async function main() {
    var gatewayUrl = "ws://127.0.0.1:1793";
    var directoryUrl = "http://127.0.0.1:8080/api/presence/topology";

    // Get the topology, then the mixnode and provider data
    const topology = await getTopology(directoryUrl);

    // Set up a websocket connection to the gateway node
    var connection = await connectWebsocket(gatewayUrl).then(function (c) {
        return c
    }).catch(function (err) {
        console.log("Websocket ERROR: " + err);
    })

    // Set up the send button
    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageToMixnet(connection, topology);
    }
}

// Create a Sphinx packet and send it to the mixnet through the Gateway node. 
function sendMessageToMixnet(connection, topology) {
    var recipient = document.getElementById("recipient").value;
    var sendText = document.getElementById("sendtext").value;
    let packet = wasm.create_sphinx_packet(JSON.stringify(topology), sendText, recipient);
    connection.send(packet);
    displaySend(packet);
    display("Sent a Sphinx packet containing message: " + sendText);
}

async function getTopology(directoryUrl) {
    let response = await http('get', directoryUrl);
    let topology = JSON.parse(response);
    return topology;
}

// Let's get started!
main();

// utility functions below here, nothing too interesting...

function display(message) {
    document.getElementById("output").innerHTML = "<p>" + message + "</p >" + document.getElementById("output").innerHTML;
}

function displaySend(message) {
    document.getElementById("output").innerHTML = "<p style='color: blue; word-break: break-all;'>sent >>> " + message + "</p >" + document.getElementById("output").innerHTML;
}

function http(method, url) {
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

function connectWebsocket(url) {
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