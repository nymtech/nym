// Copyright 2020-2022 Nym Technologies SA
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

import {default_debug, get_gateway, NymClient, set_panic_hook, Config} from "@nymproject/nym-client-wasm"

class ClientWrapper {
    constructor(config) {
        this.rustClient = new NymClient(config);
        this.rustClient.set_on_message(this.on_message);
        this.rustClient.set_on_gateway_connect(this.on_connect);
    }

    selfAddress = () => {
        return this.rustClient.self_address()
    }

    on_message = (msg) => displayReceived(msg);
    on_connect = () => {
        console.log("Established (and authenticated) gateway connection!");
    }

    start = async () => {
        // this is current limitation of wasm in rust - for async methods you can't take self by reference...
        // I'm trying to figure out if I can somehow hack my way around it, but for time being you have to re-assign
        // the object (it's the same one)
        this.rustClient = await this.rustClient.start()
    }

    sendMessage = async (recipient, message) => {
        this.rustClient = await this.rustClient.send_message(recipient, message)
    }

    sendBinaryMessage = async (recipient, message) => {
        this.rustClient = await this.rustClient.send_binary_message(recipient, message)
    }
}

let client = null

async function main() {
    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    // validator server we will use to get topology from
    const validator = "https://validator.nymtech.net/api"; //"http://localhost:8081";
    const preferredGateway = "E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM";

    const gatewayEndpoint = await get_gateway(validator, preferredGateway);

    // only really useful if you want to adjust some settings like traffic rate
    // (if not needed you can just pass a null)
    const debug = default_debug();
    debug.loop_cover_traffic_average_delay_ms = BigInt(60_000);
    // note: we still have poisson distribution so, on average, we will be sending SOME packet every 20ms
    debug.message_sending_average_delay_ms = BigInt(20);
    debug.average_packet_delay_ms = BigInt(10);
    debug.average_ack_delay_ms = BigInt(10);

    const config = new Config("my-awesome-wasm-client", validator, gatewayEndpoint, debug)

    client = new ClientWrapper(config);
    await client.start();

    const self_address = client.rustClient.self_address();
    displaySenderAddress(self_address);

    const sendButton = document.querySelector('#send-button');
    sendButton.onclick = function () {
        sendMessageTo();
    }
}

/**
 * Create a Sphinx packet and send it to the mixnet through the gateway node.
 * 
 * Message and recipient are taken from the values in the user interface.
 *
 * @param {Client} nymClient the nym client to use for message sending
 */
async function sendMessageTo() {
    const message = document.getElementById("message").value;
    const recipient = document.getElementById("recipient").value;

    await client.sendMessage(message, recipient);
    displaySend(message);
}

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} message
 */
function displaySend(message) {
    let timestamp = new Date().toISOString().substr(11, 12);

    let sendDiv = document.createElement("div")
    let paragraph = document.createElement("p")
    paragraph.setAttribute('style', 'color: blue')
    let paragraphContent = document.createTextNode(timestamp + " sent >>> " + message)
    paragraph.appendChild(paragraphContent)

    sendDiv.appendChild(paragraph)
    document.getElementById("output").appendChild(sendDiv)
}

/**
 * Display received text messages in the browser. Colour them green.
 *
 * @param {string} message
 */
function displayReceived(message) {
    const content = message;

    let timestamp = new Date().toISOString().substr(11, 12);
    let receivedDiv = document.createElement("div")
    let paragraph = document.createElement("p")
    paragraph.setAttribute('style', 'color: green')
    let paragraphContent = document.createTextNode(timestamp + " received >>> " + content);
    // let paragraphContent = document.createTextNode(timestamp + " received >>> " + content + ((replySurb != null) ? "Reply SURB was attached here (but we can't do anything with it yet" : " (NO REPLY-SURB AVAILABLE)"))
    paragraph.appendChild(paragraphContent)
    receivedDiv.appendChild(paragraph)
    document.getElementById("output").appendChild(receivedDiv)
}


/**
 * Display the nymClient's sender address in the user interface
 *
 * @param {Client} nymClient
 */
function displaySenderAddress(address) {
    document.getElementById("sender").value = address;
}

// Let's get started!
main();