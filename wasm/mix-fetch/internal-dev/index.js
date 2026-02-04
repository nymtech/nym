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
                    case 'DisplayString':
                        const { rawString } = ev.data.args;
                        displayReceivedRawString(rawString)
                        break;
                    case 'Log':
                        const { message, level } = ev.data.args;
                        displayLog(message, level);
                        break;
                    case 'MixFetchReady':
                        onMixFetchReady();
                        break;
                    case 'MixFetchError':
                        const { error } = ev.data.args;
                        onMixFetchError(error);
                        break;
                }
            }
        };
    }

    startMixFetch = (preferredGateway) => {
        if (!this.worker) {
            console.error('Could not send message because worker does not exist');
            return;
        }

        this.worker.postMessage({
            kind: 'StartMixFetch',
            args: {
                preferredGateway,
            },
        });
    }

    doFetch = (target) => {
        if (!this.worker) {
            console.error('Could not send message because worker does not exist');
            return;
        }

        this.worker.postMessage({
            kind: 'FetchPayload',
            args: {
                target,
            },
        });
    }
}

let client = null;

const DEFAULT_GATEWAY = "q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1";

async function main() {
    client = new WebWorkerClient();

    const startButton = document.querySelector('#start-mixfetch');
    startButton.onclick = function () {
        const gatewayMode = document.querySelector('input[name="gateway-mode"]:checked').value;
        const preferredGateway = gatewayMode === 'default' ? DEFAULT_GATEWAY : undefined;

        startButton.disabled = true;
        document.querySelectorAll('input[name="gateway-mode"]').forEach(r => r.disabled = true);
        updateStatus('Starting...', 'orange');

        displayLog(`Starting MixFetch with ${gatewayMode} gateway${preferredGateway ? ` (${preferredGateway})` : ''}...`, 'info');
        client.startMixFetch(preferredGateway);
    }

    const fetchButton1 = document.querySelector('#fetch-button-1');
    fetchButton1.onclick = function () {
        doFetch(1);
    }

    const fetchButton2 = document.querySelector('#fetch-button-2');
    fetchButton2.onclick = function () {
        doFetch(2);
    }

    const fetch10Button = document.querySelector('#fetch-10-concurrent');
    fetch10Button.onclick = function () {
        doFetch10Concurrent();
    }
}

function updateStatus(text, color) {
    const status = document.getElementById('mixfetch-status');
    status.textContent = text;
    status.style.color = color;
}

function onMixFetchReady() {
    updateStatus('Ready', 'green');
    document.getElementById('fetch-controls').disabled = false;
    displayLog('MixFetch is ready!', 'info');
}

function onMixFetchError(error) {
    updateStatus('Error: ' + error, 'red');
    document.querySelector('#start-mixfetch').disabled = false;
    document.querySelectorAll('input[name="gateway-mode"]').forEach(r => r.disabled = false);
    displayLog('MixFetch error: ' + error, 'error');
}


async function doFetch(id) {
    const payload = document.getElementById(`fetch_payload_${id}`).value;
    await client.doFetch(payload)

    displaySend(`[${id}] clicked the button and the payload is: ${payload}...`);
}

async function doFetch10Concurrent() {
    const baseUrl = 'https://jsonplaceholder.typicode.com/posts/';
    displaySend('Starting 10 concurrent requests to posts/1-10...');

    const requests = [];
    for (let i = 1; i <= 10; i++) {
        const url = `${baseUrl}${i}`;
        displaySend(`[${i}] Sending request to ${url}`);
        requests.push(client.doFetch(url));
    }

    await Promise.all(requests);
    displaySend('All 10 concurrent requests dispatched!');
}

/**
 * Display log messages from MixFetch. Colors based on level.
 *
 * @param {string} message
 * @param {string} level - 'info', 'error', 'warn', or 'debug'
 */
function displayLog(message, level) {
    let timestamp = new Date().toISOString().substr(11, 12);

    const colors = {
        info: 'gray',
        error: 'red',
        warn: 'orange',
        debug: 'purple',
    };

    let logDiv = document.createElement('div');
    let paragraph = document.createElement('p');
    paragraph.setAttribute('style', `color: ${colors[level] || 'gray'}`);
    let paragraphContent = document.createTextNode(timestamp + ' [' + level.toUpperCase() + '] ' + message);
    paragraph.appendChild(paragraphContent);

    logDiv.appendChild(paragraph);
    document.getElementById('output').appendChild(logDiv);
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

main();
