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
                    case 'ReceivedCredential':
                        const { credential } = ev.data.args;
                        displayCredential(credential)
                        break;
                }
            }
        };
    }

    getCredential = (amount, mnemonic) => {
        if (!this.worker) {
            console.error('Could not get credential because worker does not exist');
            return;
        }

        this.worker.postMessage({
            kind: 'GetCredential',
            args: {
                amount,
                mnemonic
            },
        });
    };
}

let client = null;

async function main() {
    client = new WebWorkerClient();

    const coconutButton = document.querySelector('#coconut-button');
    coconutButton.onclick = function () {
        getCredential();
    };
}

async function getCredential() {
    const amount = document.getElementById('credential-amount').value;
    const mnemonic = document.getElementById('mnemonic').value;

    await client.getCredential(amount, mnemonic);
}



function displayCredential(credential) {
    console.log("got credential", credential)

    let timestamp = new Date().toISOString().substr(11, 12);

    let credentialDiv = document.createElement('div');
    let paragraph = document.createElement('p');
    paragraph.setAttribute('style', 'color: blue');
    let paragraphContent = document.createTextNode(timestamp + ' 🥥🥥🥥 >>> ' + JSON.stringify(credential));
    paragraph.appendChild(paragraphContent);

    credentialDiv.appendChild(paragraph);
    document.getElementById('output').appendChild(credentialDiv);
}


main();
