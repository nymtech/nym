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
   NymClient,
   set_panic_hook
} from "@nymproject/nym-client-wasm"

// current limitation of rust-wasm for async stuff : (
let client = null

async function main() {
    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    // validator we will use to get topology from
    // If you run into CORS errors make sure to set `enable-unsafe-cors` as true
    // and add `'Access-Control-Allow-Origin'` to the accepted headers
    // in your validator's `config.toml`,
    const validator = "http://mixnet.club:1317"; //  1317 = standard API port.

    // mixnet contract address
    const mixnet_contract = "hal1k0jntykt7e4g3y88ltc60czgjuqdy4c9c6gv94"

    client = new NymClient(validator,mixnet_contract);

    const on_message = (msg) => displayReceived(msg);
    const on_connect = () => console.log("Established (and authenticated) gateway connection!");

    client.set_on_message(on_message);
    client.set_on_gateway_connect(on_connect);

    // this is current limitation of wasm in rust - for async methods you can't take self my reference...
    // I'm trying to figure out if I can somehow hack my way around it, but for time being you have to re-assign
    // the object (it's the same one)
    client = await client.initial_setup();

    const self_address = client.self_address();
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
    // TODO add input validation of required length for the address & check other two aren't blank before sending
    const telegram = document.getElementById("telegram").value; // check isnt empty + check first char is @
    const address = document.getElementById("hal_address").value; // check .length == 42
    const message = document.getElementById("message").value; // check isnt empty
    const self_address = client.self_address();

    const concat = telegram.concat(' | ').concat(address).concat(' | ').concat(message).concat(' | ').concat(self_address);
    console.log(concat);

    const recipient = document.getElementById("recipient").value;

    // you can just say "hey, im @<telegram-handle>, send replies to <nym address here>"
    // 😎

    client = await client.send_message(concat, recipient);
    displaySend(telegram, address, message, self_address);
}

/**
 * Display messages that have been sent up the websocket. Colours them blue.
 *
 * @param {string} telegram
 * @param {string} hal_address
 * @param {string} message
 * @param {string} self_address
 */
function displaySend(telegram, hal_address, message, self_address) {
    let timestamp = new Date().toISOString().substr(11, 12);

    let sendDiv = document.createElement("div")
    let paragraph = document.createElement("p")
    paragraph.setAttribute('style', 'color: blue')


    let paragraphContent = document.createTextNode(
      timestamp + " sent the following information >>> " +
      ' | telegram: ' + telegram +
      ' | HAL address: ' + hal_address +
      ' | message: ' + message +
      ' | replyTo: ' + self_address
    );

    paragraph.appendChild(paragraphContent);

    sendDiv.appendChild(paragraph)
    document.getElementById("output").appendChild(sendDiv)
}

/**
 * Display received text messages in the browser. Colour them green.
 *
 * @param {string} message
 */
function displayReceived(message) {
    const content = message.message
    const replySurb = message.replySurb

    let timestamp = new Date().toISOString().substr(11, 12);
    let receivedDiv = document.createElement("div")
    let paragraph = document.createElement("p")
    paragraph.setAttribute('style', 'color: green')
    //  TODO add parser and display properly formatted content
    let paragraphContent = document.createTextNode(timestamp + " received >>> " + content + ((replySurb != null) ? "Reply SURB was attached here (but we can't do anything with it yet" : " (NO REPLY-SURB AVAILABLE)"))
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
