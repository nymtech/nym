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
                }
            }
        };
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

async function main() {
    client = new WebWorkerClient();

    const fetchButton = document.querySelector('#fetch-button');
    fetchButton.onclick = function () {
        doFetch();
    }
}


async function doFetch() {
    const payload = document.getElementById('fetch_payload').value;
    await client.doFetch(payload)

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
